use std::time::Duration;

use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::Provider;
use codex_app_server_protocol::ProviderAuthStyle;
use codex_app_server_protocol::ProviderHeaderSummary;
use codex_app_server_protocol::ProviderHeaderValueSource;
use codex_app_server_protocol::ProviderListParams;
use codex_app_server_protocol::ProviderListResponse;
use codex_app_server_protocol::ProviderReadParams;
use codex_app_server_protocol::ProviderReadResponse;
use codex_app_server_protocol::ProviderSource;
use codex_app_server_protocol::ProviderWireApi;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;

fn write_provider_config_toml(codex_home: &TempDir) -> std::io::Result<()> {
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "deepseek-chat"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "deepseek"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
env_key = "DEEPSEEK_API_KEY"
wire_api = "chat_completions"
supports_websockets = false
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000

[model_providers.deepseek.http_headers]
X-Title = "Codex Local"

[model_providers.deepseek.env_http_headers]
X-Trace = "DEEPSEEK_TRACE_ID"
"#,
    )
}

fn expected_deepseek_provider() -> Provider {
    Provider {
        id: "deepseek".to_string(),
        display_name: "DeepSeek".to_string(),
        source: ProviderSource::Custom,
        builtin_kind: None,
        base_url: Some("https://api.deepseek.com/v1".to_string()),
        wire_api: ProviderWireApi::ChatCompletions,
        auth_style: ProviderAuthStyle::Bearer,
        env_key: Some("DEEPSEEK_API_KEY".to_string()),
        has_api_key: true,
        supports_websockets: false,
        requires_openai_auth: false,
        request_max_retries: Some(4),
        stream_max_retries: Some(5),
        stream_idle_timeout_ms: Some(300000),
        websocket_connect_timeout_ms: None,
        headers: vec![
            ProviderHeaderSummary {
                name: "X-Title".to_string(),
                value_source: ProviderHeaderValueSource::Literal,
                value: "Codex Local".to_string(),
            },
            ProviderHeaderSummary {
                name: "X-Trace".to_string(),
                value_source: ProviderHeaderValueSource::Env,
                value: "DEEPSEEK_TRACE_ID".to_string(),
            },
        ],
        is_default: true,
    }
}

#[tokio::test]
async fn provider_list_returns_builtin_and_custom_providers() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_provider_config_toml(&codex_home)?;
    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[
            ("DEEPSEEK_API_KEY", Some("sk-test")),
            ("DEEPSEEK_TRACE_ID", Some("trace-test")),
        ],
    )
    .await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_list_request(ProviderListParams { cwd: None })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let ProviderListResponse {
        data,
        default_provider,
        default_model,
    } = to_response::<ProviderListResponse>(response)?;

    assert_eq!(default_provider, "deepseek".to_string());
    assert_eq!(default_model, Some("deepseek-chat".to_string()));
    assert!(data.iter().any(|provider| provider.id == "openai"));
    assert!(data.iter().any(|provider| provider.id == "ollama"));
    assert!(data.iter().any(|provider| provider.id == "lmstudio"));

    let deepseek = data
        .into_iter()
        .find(|provider| provider.id == "deepseek")
        .expect("deepseek provider should be present");
    assert_eq!(deepseek, expected_deepseek_provider());
    Ok(())
}

#[tokio::test]
async fn provider_read_returns_custom_provider_details() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_provider_config_toml(&codex_home)?;
    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[
            ("DEEPSEEK_API_KEY", Some("sk-test")),
            ("DEEPSEEK_TRACE_ID", Some("trace-test")),
        ],
    )
    .await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_read_request(ProviderReadParams {
            id: "deepseek".to_string(),
            cwd: None,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let ProviderReadResponse { provider } = to_response::<ProviderReadResponse>(response)?;

    assert_eq!(provider, expected_deepseek_provider());
    Ok(())
}

#[tokio::test]
async fn provider_read_rejects_unknown_provider_id() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_provider_config_toml(&codex_home)?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_read_request(ProviderReadParams {
            id: "missing".to_string(),
            cwd: None,
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.id, RequestId::Integer(request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(error.error.message, "unknown provider id: missing");
    Ok(())
}
