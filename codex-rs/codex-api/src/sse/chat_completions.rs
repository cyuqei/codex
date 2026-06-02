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
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::time::timeout;
use tracing::debug;
use tracing::trace;

const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn spawn_chat_completions_stream(
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
    tokio::spawn(process_chat_completions_sse(
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
struct ChatStreamChunk {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
    #[serde(default)]
    error: Option<ChatError>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    #[serde(default)]
    delta: ChatDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatToolCallDelta>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatToolCallDelta {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<ChatToolFunctionDelta>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatToolFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}

impl From<ChatUsage> for TokenUsage {
    fn from(value: ChatUsage) -> Self {
        TokenUsage {
            input_tokens: value.prompt_tokens,
            cached_input_tokens: 0,
            output_tokens: value.completion_tokens,
            reasoning_output_tokens: 0,
            total_tokens: value.total_tokens,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatError {
    #[serde(default)]
    message: Option<String>,
}

#[derive(Default)]
struct ChatStreamState {
    response_id: Option<String>,
    item_id: Option<String>,
    text: String,
    tool_calls: BTreeMap<usize, ToolCallState>,
    usage: Option<TokenUsage>,
    saw_done: bool,
}

#[derive(Default)]
struct ToolCallState {
    item_id: Option<String>,
    call_id: Option<String>,
    name: String,
    arguments: String,
}

fn finish_text_item(state: &mut ChatStreamState, events: &mut Vec<ResponseEvent>) {
    if let Some(item_id) = state.item_id.take() {
        events.push(ResponseEvent::OutputItemDone(ResponseItem::Message {
            id: Some(item_id),
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: std::mem::take(&mut state.text),
            }],
            phase: None,
        }));
    }
}

fn finish_tool_calls(state: &mut ChatStreamState, events: &mut Vec<ResponseEvent>) {
    for (_, tool_call) in std::mem::take(&mut state.tool_calls) {
        if let Some(item_id) = tool_call.item_id {
            events.push(ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                id: Some(item_id),
                name: tool_call.name,
                namespace: None,
                arguments: tool_call.arguments,
                call_id: tool_call
                    .call_id
                    .unwrap_or_else(|| "chat-completions-tool-call".to_string()),
            }));
        }
    }
}

fn ensure_tool_call_item(
    state: &mut ChatStreamState,
    index: usize,
    delta: &ChatToolCallDelta,
    events: &mut Vec<ResponseEvent>,
) {
    let tool_call = state.tool_calls.entry(index).or_default();
    if let Some(id) = delta.id.as_ref() {
        tool_call.call_id = Some(id.clone());
    }
    if let Some(function) = delta.function.as_ref() {
        if let Some(name) = function.name.as_ref() {
            tool_call.name = name.clone();
        }
    }
    if tool_call.item_id.is_none() {
        let call_id = tool_call.call_id.clone().unwrap_or_else(|| {
            format!(
                "{}-tool-{index}",
                state.response_id.as_deref().unwrap_or("chat-completions")
            )
        });
        tool_call.item_id = Some(call_id.clone());
        events.push(ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall {
            id: Some(call_id.clone()),
            name: tool_call.name.clone(),
            namespace: None,
            arguments: tool_call.arguments.clone(),
            call_id,
        }));
    }
}

fn process_tool_call_deltas(
    state: &mut ChatStreamState,
    deltas: Vec<ChatToolCallDelta>,
    events: &mut Vec<ResponseEvent>,
) {
    if deltas.is_empty() {
        return;
    }

    if state.item_id.is_some() {
        finish_text_item(state, events);
    }

    for delta in deltas {
        ensure_tool_call_item(state, delta.index, &delta, events);
        let tool_call = state
            .tool_calls
            .get_mut(&delta.index)
            .expect("tool call state exists");
        if let Some(function) = delta.function
            && let Some(arguments) = function.arguments
            && !arguments.is_empty()
        {
            tool_call.arguments.push_str(&arguments);
            if let Some(item_id) = tool_call.item_id.clone() {
                events.push(ResponseEvent::ToolCallInputDelta {
                    item_id,
                    call_id: tool_call.call_id.clone(),
                    delta: arguments,
                });
            }
        }
    }
}

fn finish_all_items(state: &mut ChatStreamState, events: &mut Vec<ResponseEvent>) {
    finish_text_item(state, events);
    finish_tool_calls(state, events);
}

fn choice_is_terminal(reason: Option<&str>) -> bool {
    matches!(
        reason,
        Some("stop" | "tool_calls" | "length" | "content_filter" | "function_call")
    )
}

fn choice_ends_with_tool_calls(reason: Option<&str>) -> bool {
    matches!(reason, Some("tool_calls" | "function_call"))
}

fn choice_ends_with_text(reason: Option<&str>) -> bool {
    matches!(reason, Some("stop" | "length" | "content_filter"))
}

fn finalize_choice(
    state: &mut ChatStreamState,
    reason: Option<&str>,
    events: &mut Vec<ResponseEvent>,
) {
    if choice_ends_with_tool_calls(reason) {
        finish_text_item(state, events);
        finish_tool_calls(state, events);
    } else if choice_ends_with_text(reason) {
        finish_text_item(state, events);
    }
}

fn has_open_items(state: &ChatStreamState) -> bool {
    state.item_id.is_some() || !state.tool_calls.is_empty()
}

fn ensure_text_item(state: &mut ChatStreamState, events: &mut Vec<ResponseEvent>) {
    if state.item_id.is_none() {
        let item_id = format!(
            "{}-message",
            state.response_id.as_deref().unwrap_or("chat-completions")
        );
        state.item_id = Some(item_id.clone());
        events.push(ResponseEvent::OutputItemAdded(ResponseItem::Message {
            id: Some(item_id),
            role: "assistant".to_string(),
            content: Vec::new(),
            phase: None,
        }));
    }
}

fn process_text_delta(
    state: &mut ChatStreamState,
    content: String,
    events: &mut Vec<ResponseEvent>,
) {
    if content.is_empty() {
        return;
    }
    ensure_text_item(state, events);
    state.text.push_str(&content);
    events.push(ResponseEvent::OutputTextDelta(content));
}

fn ensure_terminal_completion(state: &mut ChatStreamState, events: &mut Vec<ResponseEvent>) {
    if has_open_items(state) {
        finish_all_items(state, events);
    }
}

fn default_response_id(state: &ChatStreamState) -> String {
    state
        .response_id
        .clone()
        .unwrap_or_else(|| "chat-completions".to_string())
}

fn completed_event(state: &mut ChatStreamState) -> ResponseEvent {
    ResponseEvent::Completed {
        response_id: default_response_id(state),
        token_usage: state.usage.take(),
        end_turn: Some(true),
    }
}

fn start_response_id(state: &mut ChatStreamState, chunk: &ChatStreamChunk) {
    if state.response_id.is_none() {
        state.response_id = chunk.id.clone();
    }
}

fn update_usage(state: &mut ChatStreamState, chunk: ChatStreamChunk) -> ChatStreamChunk {
    if let Some(usage) = chunk.usage.as_ref() {
        state.usage = Some(TokenUsage {
            input_tokens: usage.prompt_tokens,
            cached_input_tokens: 0,
            output_tokens: usage.completion_tokens,
            reasoning_output_tokens: 0,
            total_tokens: usage.total_tokens,
        });
    }
    chunk
}

fn map_provider_error(chunk: ChatStreamChunk) -> Result<ChatStreamChunk, ApiError> {
    if let Some(error) = chunk.error.as_ref() {
        return Err(ApiError::Stream(
            error
                .message
                .clone()
                .unwrap_or_else(|| "Chat Completions stream error".to_string()),
        ));
    }
    Ok(chunk)
}

fn parse_chunk(data: &str) -> Result<ChatStreamChunk, ApiError> {
    serde_json::from_str(data).map_err(|e| {
        debug!("failed to parse Chat Completions SSE event: {e}, data: {data}");
        ApiError::Stream(format!(
            "failed to parse chat completions stream event: {e}"
        ))
    })
}

fn done_events(state: &mut ChatStreamState) -> Vec<ResponseEvent> {
    state.saw_done = true;
    let mut events = Vec::new();
    ensure_terminal_completion(state, &mut events);
    events.push(completed_event(state));
    events
}

fn chunk_events(mut chunk: ChatStreamChunk, state: &mut ChatStreamState) -> Vec<ResponseEvent> {
    start_response_id(state, &chunk);
    chunk = update_usage(state, chunk);
    let mut events = Vec::new();
    for choice in chunk.choices {
        process_tool_call_deltas(state, choice.delta.tool_calls, &mut events);
        if let Some(content) = choice.delta.content {
            process_text_delta(state, content, &mut events);
        }
        if choice_is_terminal(choice.finish_reason.as_deref()) {
            finalize_choice(state, choice.finish_reason.as_deref(), &mut events);
        }
    }
    events
}

fn parse_and_process_chunk(
    data: &str,
    state: &mut ChatStreamState,
) -> Result<Vec<ResponseEvent>, ApiError> {
    let chunk = parse_chunk(data)?;
    let chunk = map_provider_error(chunk)?;
    Ok(chunk_events(chunk, state))
}

fn done_or_chunk_events(
    data: &str,
    state: &mut ChatStreamState,
) -> Result<Vec<ResponseEvent>, ApiError> {
    if data.trim() == "[DONE]" {
        return Ok(done_events(state));
    }
    parse_and_process_chunk(data, state)
}

fn process_chat_completions_event(
    data: &str,
    state: &mut ChatStreamState,
) -> Result<Vec<ResponseEvent>, ApiError> {
    done_or_chunk_events(data, state)
}

#[cfg(test)]
fn text_from_done_message(item: &ResponseItem) -> Option<&str> {
    match item {
        ResponseItem::Message { content, .. } => match content.first() {
            Some(ContentItem::OutputText { text }) => Some(text.as_str()),
            _ => None,
        },
        _ => None,
    }
}

pub async fn process_chat_completions_sse(
    stream: ByteStream,
    tx_event: mpsc::Sender<Result<ResponseEvent, ApiError>>,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
) {
    let mut stream = stream.eventsource();
    let mut state = ChatStreamState::default();

    loop {
        let start = Instant::now();
        let response = timeout(idle_timeout, stream.next()).await;
        if let Some(t) = telemetry.as_ref() {
            t.on_sse_poll(&response, start.elapsed());
        }
        let sse = match response {
            Ok(Some(Ok(sse))) => sse,
            Ok(Some(Err(e))) => {
                debug!("Chat Completions SSE error: {e:#}");
                let _ = tx_event.send(Err(ApiError::Stream(e.to_string()))).await;
                return;
            }
            Ok(None) => {
                let _ = tx_event
                    .send(Err(ApiError::Stream(
                        "stream closed before chat completions [DONE]".into(),
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

        trace!("Chat Completions SSE event: {}", &sse.data);

        match process_chat_completions_event(&sse.data, &mut state) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_delta_maps_to_response_events() {
        let mut state = ChatStreamState::default();
        let events = process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{"content":"Hello"}}]}"#,
            &mut state,
        )
        .unwrap();

        assert!(matches!(events[0], ResponseEvent::OutputItemAdded(_)));
        assert!(matches!(events[1], ResponseEvent::OutputTextDelta(ref text) if text == "Hello"));
    }

    #[test]
    fn done_emits_completed() {
        let mut state = ChatStreamState::default();
        process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{"content":"Hello"}}]}"#,
            &mut state,
        )
        .unwrap();
        let events = process_chat_completions_event("[DONE]", &mut state).unwrap();

        assert!(matches!(events[0], ResponseEvent::OutputItemDone(_)));
        assert!(
            matches!(events[1], ResponseEvent::Completed { ref response_id, .. } if response_id == "chatcmpl-1")
        );
    }

    #[test]
    fn provider_error_maps_to_stream_error() {
        let mut state = ChatStreamState::default();
        let err =
            process_chat_completions_event(r#"{"error":{"message":"bad request"}}"#, &mut state)
                .unwrap_err();

        assert!(matches!(err, ApiError::Stream(message) if message == "bad request"));
    }

    #[test]
    fn tool_call_delta_maps_to_function_call_events() {
        let mut state = ChatStreamState::default();
        let events = process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"Bash","arguments":"{\"command\":"}}]}}]}"#,
            &mut state,
        )
        .unwrap();

        assert!(
            matches!(events[0], ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall { ref call_id, ref name, .. }) if call_id == "call_1" && name == "Bash")
        );
        assert!(
            matches!(events[1], ResponseEvent::ToolCallInputDelta { ref item_id, call_id: Some(ref call_id), ref delta } if item_id == "call_1" && call_id == "call_1" && delta == "{\"command\":")
        );

        let done = process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            &mut state,
        )
        .unwrap();
        assert!(
            matches!(done[0], ResponseEvent::OutputItemDone(ResponseItem::FunctionCall { ref call_id, ref name, ref arguments, .. }) if call_id == "call_1" && name == "Bash" && arguments == "{\"command\":")
        );
    }

    #[test]
    fn commentary_then_tool_calls_finishes_text_before_tool_call() {
        let mut state = ChatStreamState::default();
        let text_events = process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{"content":"Let me check the repo."}}]}"#,
            &mut state,
        )
        .unwrap();
        assert!(matches!(
            text_events[0],
            ResponseEvent::OutputItemAdded(ResponseItem::Message { .. })
        ));
        assert!(
            matches!(text_events[1], ResponseEvent::OutputTextDelta(ref text) if text == "Let me check the repo.")
        );

        let tool_events = process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"Bash","arguments":"{\"command\":\"pwd\"}"}}]},"finish_reason":"tool_calls"}]}"#,
            &mut state,
        )
        .unwrap();

        assert!(
            matches!(tool_events[0], ResponseEvent::OutputItemDone(ref item) if text_from_done_message(item) == Some("Let me check the repo."))
        );
        assert!(
            matches!(tool_events[1], ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall { ref call_id, ref name, .. }) if call_id == "call_1" && name == "Bash")
        );
        assert!(
            matches!(tool_events[2], ResponseEvent::ToolCallInputDelta { ref delta, .. } if delta == "{\"command\":\"pwd\"}")
        );
        assert!(
            matches!(tool_events[3], ResponseEvent::OutputItemDone(ResponseItem::FunctionCall { ref call_id, ref arguments, .. }) if call_id == "call_1" && arguments == "{\"command\":\"pwd\"}")
        );

        let done = process_chat_completions_event("[DONE]", &mut state).unwrap();
        assert!(
            matches!(done.last(), Some(ResponseEvent::Completed { response_id, .. }) if response_id == "chatcmpl-1")
        );
    }

    #[test]
    fn multiple_tool_calls_across_chunks_are_aggregated_by_index() {
        let mut state = ChatStreamState::default();

        let first = process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"Bash","arguments":"{\"command\":\"p"}},{"index":1,"id":"call_2","function":{"name":"Read","arguments":"{\"file_path\":\"/tmp/"}}]}}]}"#,
            &mut state,
        )
        .unwrap();
        assert!(
            matches!(first[0], ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall { ref call_id, ref name, .. }) if call_id == "call_1" && name == "Bash")
        );
        assert!(
            matches!(first[1], ResponseEvent::ToolCallInputDelta { call_id: Some(ref call_id), ref delta, .. } if call_id == "call_1" && delta == "{\"command\":\"p")
        );
        assert!(
            matches!(first[2], ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall { ref call_id, ref name, .. }) if call_id == "call_2" && name == "Read")
        );
        assert!(
            matches!(first[3], ResponseEvent::ToolCallInputDelta { call_id: Some(ref call_id), ref delta, .. } if call_id == "call_2" && delta == "{\"file_path\":\"/tmp/")
        );

        let second = process_chat_completions_event(
            r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"wd\"}"}},{"index":1,"function":{"arguments":"foo.txt\"}"}}]},"finish_reason":"tool_calls"}]}"#,
            &mut state,
        )
        .unwrap();
        assert!(
            matches!(second[0], ResponseEvent::ToolCallInputDelta { call_id: Some(ref call_id), ref delta, .. } if call_id == "call_1" && delta == "wd\"}")
        );
        assert!(
            matches!(second[1], ResponseEvent::ToolCallInputDelta { call_id: Some(ref call_id), ref delta, .. } if call_id == "call_2" && delta == "foo.txt\"}")
        );
        assert!(
            matches!(second[2], ResponseEvent::OutputItemDone(ResponseItem::FunctionCall { ref call_id, ref arguments, .. }) if call_id == "call_1" && arguments == "{\"command\":\"pwd\"}")
        );
        assert!(
            matches!(second[3], ResponseEvent::OutputItemDone(ResponseItem::FunctionCall { ref call_id, ref arguments, .. }) if call_id == "call_2" && arguments == "{\"file_path\":\"/tmp/foo.txt\"}")
        );
    }

    #[test]
    fn commentary_then_multiple_tool_calls_finishes_text_once() {
        let mut state = ChatStreamState::default();
        process_chat_completions_event(
            r#"{"id":"chatcmpl-2","choices":[{"delta":{"content":"I’ll inspect two files."}}]}"#,
            &mut state,
        )
        .unwrap();

        let events = process_chat_completions_event(
            r#"{"id":"chatcmpl-2","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_a","function":{"name":"Read","arguments":"{\"file_path\":\"a.rs\"}"}},{"index":1,"id":"call_b","function":{"name":"Read","arguments":"{\"file_path\":\"b.rs\"}"}}]},"finish_reason":"tool_calls"}]}"#,
            &mut state,
        )
        .unwrap();

        assert!(
            matches!(events[0], ResponseEvent::OutputItemDone(ref item) if text_from_done_message(item) == Some("I’ll inspect two files."))
        );
        assert!(
            matches!(events[1], ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall { ref call_id, .. }) if call_id == "call_a")
        );
        assert!(
            matches!(events[2], ResponseEvent::ToolCallInputDelta { call_id: Some(ref call_id), .. } if call_id == "call_a")
        );
        assert!(
            matches!(events[3], ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall { ref call_id, .. }) if call_id == "call_b")
        );
        assert!(
            matches!(events[4], ResponseEvent::ToolCallInputDelta { call_id: Some(ref call_id), .. } if call_id == "call_b")
        );
        assert!(
            matches!(events[5], ResponseEvent::OutputItemDone(ResponseItem::FunctionCall { ref call_id, .. }) if call_id == "call_a")
        );
        assert!(
            matches!(events[6], ResponseEvent::OutputItemDone(ResponseItem::FunctionCall { ref call_id, .. }) if call_id == "call_b")
        );
    }
}
