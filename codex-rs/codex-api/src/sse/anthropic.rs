use crate::common::ResponseEvent;
use crate::common::ResponseStream;
use crate::error::ApiError;
use crate::telemetry::SseTelemetry;
use codex_client::ByteStream;
use codex_client::StreamResponse;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TokenUsage;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::time::timeout;
use tracing::debug;
use tracing::trace;

const REQUEST_ID_HEADER: &str = "request-id";

pub fn spawn_anthropic_message_stream(
    stream_response: StreamResponse,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
) -> ResponseStream {
    let upstream_request_id = stream_response
        .headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let (tx_event, rx_event) = mpsc::channel::<Result<ResponseEvent, ApiError>>(1600);
    tokio::spawn(process_anthropic_sse(
        stream_response.bytes,
        tx_event,
        idle_timeout,
        telemetry,
    ));

    ResponseStream {
        rx_event,
        upstream_request_id,
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    message: Option<Value>,
    #[serde(default)]
    content_block: Option<Value>,
    #[serde(default)]
    delta: Option<Value>,
    #[serde(default)]
    error: Option<AnthropicError>,
}

#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(default)]
    message: Option<String>,
}

#[derive(Default)]
struct AnthropicStreamState {
    response_id: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    active_block: Option<ActiveBlock>,
}

enum ActiveBlock {
    Text {
        item_id: String,
        text: String,
    },
    ToolUse {
        item_id: String,
        call_id: String,
        name: String,
        arguments: String,
    },
}

pub async fn process_anthropic_sse(
    stream: ByteStream,
    tx_event: mpsc::Sender<Result<ResponseEvent, ApiError>>,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
) {
    let mut stream = stream.eventsource();
    let mut state = AnthropicStreamState::default();

    loop {
        let start = Instant::now();
        let response = timeout(idle_timeout, stream.next()).await;
        if let Some(t) = telemetry.as_ref() {
            t.on_sse_poll(&response, start.elapsed());
        }
        let sse = match response {
            Ok(Some(Ok(sse))) => sse,
            Ok(Some(Err(e))) => {
                debug!("Anthropic SSE error: {e:#}");
                let _ = tx_event.send(Err(ApiError::Stream(e.to_string()))).await;
                return;
            }
            Ok(None) => {
                let _ = tx_event
                    .send(Err(ApiError::Stream(
                        "stream closed before message_stop".into(),
                    )))
                    .await;
                return;
            }
            Err(_) => {
                let _ = tx_event
                    .send(Err(ApiError::Stream("idle timeout waiting for SSE".into())))
                    .await;
                return;
            }
        };

        trace!("Anthropic SSE event: {}", &sse.data);

        let event: AnthropicStreamEvent = match serde_json::from_str(&sse.data) {
            Ok(event) => event,
            Err(e) => {
                debug!(
                    "failed to parse Anthropic SSE event: {e}, data: {}",
                    &sse.data
                );
                continue;
            }
        };

        match process_anthropic_event(event, &mut state) {
            Ok(events) => {
                for event in events {
                    let is_completed = matches!(event, ResponseEvent::Completed { .. });
                    if tx_event.send(Ok(event)).await.is_err() {
                        return;
                    }
                    if is_completed {
                        return;
                    }
                }
            }
            Err(error) => {
                let _ = tx_event.send(Err(error)).await;
                return;
            }
        }
    }
}

fn process_anthropic_event(
    event: AnthropicStreamEvent,
    state: &mut AnthropicStreamState,
) -> Result<Vec<ResponseEvent>, ApiError> {
    match event.kind.as_str() {
        "message_start" => {
            if let Some(message) = event.message.as_ref() {
                state.response_id = message
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                state.input_tokens = message
                    .get("usage")
                    .and_then(|usage| usage.get("input_tokens"))
                    .and_then(Value::as_i64);
            }
            Ok(vec![ResponseEvent::Created])
        }
        "content_block_start" => content_block_start(event.content_block, state),
        "content_block_delta" => content_block_delta(event.delta, state),
        "content_block_stop" => content_block_stop(state),
        "message_delta" => {
            if let Some(delta) = event.delta.as_ref() {
                state.output_tokens = delta
                    .get("usage")
                    .and_then(|usage| usage.get("output_tokens"))
                    .and_then(Value::as_i64)
                    .or(state.output_tokens);
            }
            Ok(Vec::new())
        }
        "message_stop" => Ok(vec![ResponseEvent::Completed {
            response_id: state
                .response_id
                .clone()
                .unwrap_or_else(|| "anthropic-message".to_string()),
            token_usage: token_usage(state),
            end_turn: Some(true),
        }]),
        "error" => {
            let message = event
                .error
                .and_then(|error| error.message)
                .unwrap_or_else(|| "Anthropic stream error".to_string());
            Err(ApiError::Stream(message))
        }
        _ => Ok(Vec::new()),
    }
}

fn content_block_start(
    content_block: Option<Value>,
    state: &mut AnthropicStreamState,
) -> Result<Vec<ResponseEvent>, ApiError> {
    let Some(content_block) = content_block else {
        return Ok(Vec::new());
    };
    match content_block.get("type").and_then(Value::as_str) {
        Some("text") => {
            let item_id = format!(
                "{}-text",
                state.response_id.as_deref().unwrap_or("anthropic-message")
            );
            state.active_block = Some(ActiveBlock::Text {
                item_id: item_id.clone(),
                text: String::new(),
            });
            Ok(vec![ResponseEvent::OutputItemAdded(
                ResponseItem::Message {
                    id: Some(item_id),
                    role: "assistant".to_string(),
                    content: Vec::new(),
                    phase: None,
                },
            )])
        }
        Some("tool_use") => {
            let call_id = content_block
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("anthropic-tool-use")
                .to_string();
            let name = content_block
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            state.active_block = Some(ActiveBlock::ToolUse {
                item_id: call_id.clone(),
                call_id: call_id.clone(),
                name: name.clone(),
                arguments: String::new(),
            });
            Ok(vec![ResponseEvent::OutputItemAdded(
                ResponseItem::FunctionCall {
                    id: Some(call_id.clone()),
                    name,
                    namespace: None,
                    arguments: String::new(),
                    call_id,
                },
            )])
        }
        _ => Ok(Vec::new()),
    }
}

fn content_block_delta(
    delta: Option<Value>,
    state: &mut AnthropicStreamState,
) -> Result<Vec<ResponseEvent>, ApiError> {
    let Some(delta) = delta else {
        return Ok(Vec::new());
    };
    match delta.get("type").and_then(Value::as_str) {
        Some("text_delta") => {
            let text_delta = delta
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(ActiveBlock::Text { text, .. }) = state.active_block.as_mut() {
                text.push_str(&text_delta);
            }
            Ok(vec![ResponseEvent::OutputTextDelta(text_delta)])
        }
        Some("input_json_delta") => {
            let partial_json = delta
                .get("partial_json")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(ActiveBlock::ToolUse {
                item_id,
                call_id,
                arguments,
                ..
            }) = state.active_block.as_mut()
            {
                arguments.push_str(&partial_json);
                Ok(vec![ResponseEvent::ToolCallInputDelta {
                    item_id: item_id.clone(),
                    call_id: Some(call_id.clone()),
                    delta: partial_json,
                }])
            } else {
                Ok(Vec::new())
            }
        }
        _ => Ok(Vec::new()),
    }
}

fn content_block_stop(state: &mut AnthropicStreamState) -> Result<Vec<ResponseEvent>, ApiError> {
    match state.active_block.take() {
        Some(ActiveBlock::Text { item_id, text }) => {
            Ok(vec![ResponseEvent::OutputItemDone(ResponseItem::Message {
                id: Some(item_id),
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText { text }],
                phase: None,
            })])
        }
        Some(ActiveBlock::ToolUse {
            item_id,
            call_id,
            name,
            arguments,
        }) => Ok(vec![ResponseEvent::OutputItemDone(
            ResponseItem::FunctionCall {
                id: Some(item_id),
                name,
                namespace: None,
                arguments,
                call_id,
            },
        )]),
        None => Ok(Vec::new()),
    }
}

fn token_usage(state: &AnthropicStreamState) -> Option<TokenUsage> {
    let input_tokens = state.input_tokens.unwrap_or(0);
    let output_tokens = state.output_tokens.unwrap_or(0);
    if input_tokens == 0 && output_tokens == 0 {
        return None;
    }
    Some(TokenUsage {
        input_tokens,
        cached_input_tokens: 0,
        output_tokens,
        reasoning_output_tokens: 0,
        total_tokens: input_tokens + output_tokens,
    })
}
