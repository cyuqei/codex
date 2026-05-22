use std::time::Duration;

use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::ProviderAuthStyle;
use codex_app_server_protocol::ProviderConfigParams;
use codex_app_server_protocol::ProviderConnectionCheck;
use codex_app_server_protocol::ProviderCreateParams;
use codex_app_server_protocol::ProviderCreateResponse;
use codex_app_server_protocol::ProviderDeleteParams;
use codex_app_server_protocol::ProviderDeleteResponse;
use codex_app_server_protocol::ProviderEnvHeaderInput;
use codex_app_server_protocol::ProviderHeaderInput;
use codex_app_server_protocol::ProviderReadParams;
use codex_app_server_protocol::ProviderReadResponse;
use codex_app_server_protocol::ProviderSource;
use codex_app_server_protocol::ProviderTestConnectionChecks;
use codex_app_server_protocol::ProviderTestConnectionErrorCode;
use codex_app_server_protocol::ProviderTestConnectionParams;
use codex_app_server_protocol::ProviderTestConnectionResponse;
use codex_app_server_protocol::ProviderTestConnectionTarget;
use codex_app_server_protocol::ProviderUpdateParams;
use codex_app_server_protocol::ProviderUpdateResponse;
use codex_app_server_protocol::ProviderWireApi;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;
const NO_PROXY_ENV_OVERRIDES: &[(&str, Option<&str>)] = &[
    ("HTTP_PROXY", None),
    ("HTTPS_PROXY", None),
    ("ALL_PROXY", None),
    ("NO_PROXY", Some("127.0.0.1,localhost")),
];

fn write_base_config_toml(codex_home: &TempDir) -> std::io::Result<()> {
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"
"#,
    )
}

fn test_provider_input(base_url: String, api_key: Option<String>) -> ProviderConfigParams {
    ProviderConfigParams {
        display_name: "OpenRouter Custom".to_string(),
        base_url,
        wire_api: ProviderWireApi::Responses,
        auth_style: ProviderAuthStyle::Bearer,
        env_key: None,
        api_key,
        requires_openai_auth: false,
        supports_websockets: false,
        request_max_retries: Some(4),
        stream_max_retries: Some(5),
        stream_idle_timeout_ms: Some(300000),
        websocket_connect_timeout_ms: None,
        headers: Some(vec![ProviderHeaderInput {
            name: "X-Title".to_string(),
            value: "Codex Local".to_string(),
        }]),
        env_headers: Some(vec![ProviderEnvHeaderInput {
            name: "X-Trace".to_string(),
            env_var: "TRACE_ID".to_string(),
        }]),
    }
}

#[tokio::test]
async fn provider_create_update_and_delete_round_trip() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_base_config_toml(&codex_home)?;
    let mut mcp = McpProcess::new_with_env(codex_home.path(), NO_PROXY_ENV_OVERRIDES).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_id = mcp
        .send_provider_create_request(ProviderCreateParams {
            id: "openrouter-custom".to_string(),
            provider: test_provider_input(
                "https://openrouter.ai/api/v1".to_string(),
                Some("test-token".to_string()),
            ),
            set_default: false,
            default_model: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_id)),
    )
    .await??;
    let ProviderCreateResponse { provider, warnings } =
        to_response::<ProviderCreateResponse>(create_response)?;
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(provider.id, "openrouter-custom");
    assert_eq!(provider.source, ProviderSource::Custom);
    assert!(provider.has_api_key);

    let update_id = mcp
        .send_provider_update_request(ProviderUpdateParams {
            id: "openrouter-custom".to_string(),
            provider: test_provider_input("https://openrouter.ai/api/v2".to_string(), None),
            set_default: false,
            default_model: None,
        })
        .await?;
    let update_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(update_id)),
    )
    .await??;
    let ProviderUpdateResponse { provider, warnings } =
        to_response::<ProviderUpdateResponse>(update_response)?;
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(
        provider.base_url.as_deref(),
        Some("https://openrouter.ai/api/v2")
    );
    assert!(
        provider.has_api_key,
        "stored api key should be preserved on update"
    );

    let read_id = mcp
        .send_provider_read_request(ProviderReadParams {
            id: "openrouter-custom".to_string(),
            cwd: None,
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ProviderReadResponse { provider } = to_response::<ProviderReadResponse>(read_response)?;
    assert_eq!(
        provider.base_url.as_deref(),
        Some("https://openrouter.ai/api/v2")
    );
    assert!(provider.has_api_key);

    let delete_id = mcp
        .send_provider_delete_request(ProviderDeleteParams {
            id: "openrouter-custom".to_string(),
        })
        .await?;
    let delete_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(delete_id)),
    )
    .await??;
    let ProviderDeleteResponse { deleted, warning } =
        to_response::<ProviderDeleteResponse>(delete_response)?;
    assert!(deleted);
    assert_eq!(warning, None);

    let read_missing_id = mcp
        .send_provider_read_request(ProviderReadParams {
            id: "openrouter-custom".to_string(),
            cwd: None,
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(read_missing_id)),
    )
    .await??;
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        error.error.message,
        "unknown provider id: openrouter-custom"
    );
    Ok(())
}

#[tokio::test]
async fn provider_delete_rejects_current_default_provider() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_base_config_toml(&codex_home)?;
    let mut mcp = McpProcess::new_with_env(codex_home.path(), NO_PROXY_ENV_OVERRIDES).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_id = mcp
        .send_provider_create_request(ProviderCreateParams {
            id: "deepseek".to_string(),
            provider: test_provider_input(
                "https://api.deepseek.com/v1".to_string(),
                Some("token".to_string()),
            ),
            set_default: true,
            default_model: Some("deepseek-chat".to_string()),
        })
        .await?;
    let _: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_id)),
    )
    .await??;

    let delete_id = mcp
        .send_provider_delete_request(ProviderDeleteParams {
            id: "deepseek".to_string(),
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(delete_id)),
    )
    .await??;
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        error.error.message,
        "cannot delete currently selected provider: deepseek"
    );
    Ok(())
}

#[tokio::test]
async fn provider_test_connection_draft_runs_requested_checks() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_base_config_toml(&codex_home)?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: {}\n\ndata: [DONE]\n\n"))
        .mount(&server)
        .await;

    let mut mcp = McpProcess::new_with_env(codex_home.path(), NO_PROXY_ENV_OVERRIDES).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_test_connection_request(ProviderTestConnectionParams {
            target: ProviderTestConnectionTarget::Draft {
                id: "draft-provider".to_string(),
                provider: Box::new(test_provider_input(
                    format!("{}/v1", server.uri()),
                    Some("test-token".to_string()),
                )),
            },
            model: "gpt-5.4".to_string(),
            checks: Some(ProviderTestConnectionChecks {
                basic: true,
                streaming: true,
                tool_calling: true,
            }),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let ProviderTestConnectionResponse { ok, results, .. } =
        to_response::<ProviderTestConnectionResponse>(response)?;
    assert!(ok);
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|result| result.ok));
    assert_eq!(
        results
            .iter()
            .map(|result| result.check)
            .collect::<Vec<_>>(),
        vec![
            ProviderConnectionCheck::Basic,
            ProviderConnectionCheck::Streaming,
            ProviderConnectionCheck::ToolCalling,
        ]
    );
    Ok(())
}

#[tokio::test]
async fn provider_test_connection_saved_classifies_endpoint_failure() -> Result<()> {
    let codex_home = TempDir::new()?;
    let server = MockServer::start().await;
    std::fs::write(
        codex_home.path().join("config.toml"),
        format!(
            r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "custom"

[model_providers.custom]
name = "Custom"
base_url = "{}/v1"
wire_api = "responses"
experimental_bearer_token = "test-token"
supports_websockets = false
"#,
            server.uri()
        ),
    )?;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;
    let mut mcp = McpProcess::new_with_env(codex_home.path(), NO_PROXY_ENV_OVERRIDES).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_test_connection_request(ProviderTestConnectionParams {
            target: ProviderTestConnectionTarget::Saved {
                id: "custom".to_string(),
            },
            model: "gpt-5.4".to_string(),
            checks: Some(ProviderTestConnectionChecks {
                basic: true,
                streaming: false,
                tool_calling: false,
            }),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let ProviderTestConnectionResponse { ok, results, .. } =
        to_response::<ProviderTestConnectionResponse>(response)?;
    assert!(!ok);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].error_code,
        Some(ProviderTestConnectionErrorCode::FailEndpoint)
    );
    Ok(())
}
