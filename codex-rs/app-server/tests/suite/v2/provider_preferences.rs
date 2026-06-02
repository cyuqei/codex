use std::time::Duration;

use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::ProviderPreferencesReadParams;
use codex_app_server_protocol::ProviderPreferencesReadResponse;
use codex_app_server_protocol::ProviderPreferencesScope;
use codex_app_server_protocol::ProviderPreferencesUpdateParams;
use codex_app_server_protocol::ProviderPreferencesUpdateResponse;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn provider_preferences_read_reports_global_scope() -> Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "openai"
"#,
    )?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_preferences_read_request(ProviderPreferencesReadParams { cwd: None })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let preferences = to_response::<ProviderPreferencesReadResponse>(response)?;

    assert_eq!(
        preferences,
        ProviderPreferencesReadResponse {
            default_provider: "openai".to_string(),
            default_model: Some("gpt-5.4".to_string()),
            config_scope: ProviderPreferencesScope::Global,
        }
    );
    Ok(())
}

#[tokio::test]
async fn provider_preferences_update_global_writes_user_config() -> Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "openai"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "chat_completions"
experimental_bearer_token = "token"
supports_websockets = false
"#,
    )?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_preferences_update_request(ProviderPreferencesUpdateParams {
            default_provider: "deepseek".to_string(),
            default_model: Some("deepseek-chat".to_string()),
            config_scope: ProviderPreferencesScope::Global,
            cwd: None,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let updated = to_response::<ProviderPreferencesUpdateResponse>(response)?;

    assert_eq!(
        updated,
        ProviderPreferencesUpdateResponse {
            default_provider: "deepseek".to_string(),
            default_model: Some("deepseek-chat".to_string()),
            config_scope: ProviderPreferencesScope::Global,
        }
    );
    let written = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(written.contains("model_provider = \"deepseek\""));
    assert!(written.contains("model = \"deepseek-chat\""));
    Ok(())
}

#[tokio::test]
async fn provider_preferences_update_project_writes_project_config() -> Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"
model = "gpt-5.4"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "openai"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "chat_completions"
experimental_bearer_token = "token"
supports_websockets = false
"#,
    )?;
    let project_root = codex_home.path().join("repo");
    let project_codex = project_root.join(".codex");
    let nested = project_root.join("nested");
    std::fs::create_dir_all(&project_codex)?;
    std::fs::create_dir_all(&nested)?;
    std::fs::create_dir(project_root.join(".git"))?;
    std::fs::write(
        project_codex.join("config.toml"),
        r#"
model = "project-model"
model_provider = "deepseek"
"#,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_provider_preferences_update_request(ProviderPreferencesUpdateParams {
            default_provider: "openai".to_string(),
            default_model: Some("gpt-5.4".to_string()),
            config_scope: ProviderPreferencesScope::Project,
            cwd: Some(nested.display().to_string()),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let updated = to_response::<ProviderPreferencesUpdateResponse>(response)?;

    assert_eq!(updated.config_scope, ProviderPreferencesScope::Project);
    let project_written = std::fs::read_to_string(project_codex.join("config.toml"))?;
    assert!(project_written.contains("model_provider = \"openai\""));
    assert!(project_written.contains("model = \"gpt-5.4\""));
    let user_written = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(user_written.contains("model_provider = \"openai\""));
    assert!(!user_written.contains("project-model"));
    Ok(())
}
