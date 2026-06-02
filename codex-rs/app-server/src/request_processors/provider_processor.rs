use std::collections::HashMap;
use std::time::Duration;

use crate::config_manager::ConfigManager;
use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit;
use codex_app_server_protocol::ConfigLayerSource;
use codex_app_server_protocol::ConfigReadParams;
use codex_app_server_protocol::ConfigValueWriteParams;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::ProviderAuthStyle;
use codex_app_server_protocol::ProviderConfigParams;
use codex_app_server_protocol::ProviderConnectionCheck;
use codex_app_server_protocol::ProviderCreateParams;
use codex_app_server_protocol::ProviderCreateResponse;
use codex_app_server_protocol::ProviderDeleteParams;
use codex_app_server_protocol::ProviderDeleteResponse;
use codex_app_server_protocol::ProviderPreferencesReadParams;
use codex_app_server_protocol::ProviderPreferencesReadResponse;
use codex_app_server_protocol::ProviderPreferencesScope;
use codex_app_server_protocol::ProviderPreferencesUpdateParams;
use codex_app_server_protocol::ProviderPreferencesUpdateResponse;
use codex_app_server_protocol::ProviderReadParams;
use codex_app_server_protocol::ProviderReadResponse;
use codex_app_server_protocol::ProviderTestConnectionCheckResult;
use codex_app_server_protocol::ProviderTestConnectionChecks;
use codex_app_server_protocol::ProviderTestConnectionErrorCode;
use codex_app_server_protocol::ProviderTestConnectionParams;
use codex_app_server_protocol::ProviderTestConnectionResponse;
use codex_app_server_protocol::ProviderTestConnectionTarget;
use codex_app_server_protocol::ProviderUpdateParams;
use codex_app_server_protocol::ProviderUpdateResponse;
use codex_app_server_protocol::ProviderWireApi;
use codex_core::config::ConfigOverrides;
use codex_login::AuthManager;
use codex_model_provider::create_model_provider;
use codex_protocol::error::CodexErr;
use reqwest::StatusCode;
use reqwest::Url;
use reqwest::header::ACCEPT;
use reqwest::header::HeaderValue;
use serde::Serialize;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

use super::ConfigRequestProcessor;

const BUILTIN_PROVIDER_IDS: [&str; 4] = ["openai", "amazon-bedrock", "ollama", "lmstudio"];
const TEST_CONNECTION_TIMEOUT_SECS: u64 = 20;
const BASIC_TEST_PROMPT: &str = "Say hello in one sentence.";
const TOOL_TEST_PROMPT: &str = "Use the ping tool if tool calling is available.";

#[derive(Clone)]
pub(crate) struct ProviderRequestProcessor {
    config_manager: ConfigManager,
    config_processor: ConfigRequestProcessor,
    auth_manager: Arc<AuthManager>,
}

impl ProviderRequestProcessor {
    pub(crate) fn new(
        config_manager: ConfigManager,
        config_processor: ConfigRequestProcessor,
        auth_manager: Arc<AuthManager>,
    ) -> Self {
        Self {
            config_manager,
            config_processor,
            auth_manager,
        }
    }

    pub(crate) async fn create(
        &self,
        params: ProviderCreateParams,
    ) -> Result<ProviderCreateResponse, codex_app_server_protocol::JSONRPCErrorError> {
        self.ensure_custom_provider_id(&params.id)?;
        let config = self.load_latest_config().await?;
        if config.model_providers.contains_key(&params.id) {
            return Err(invalid_request(format!(
                "provider id already exists: {}",
                params.id
            )));
        }

        let provider_value = provider_input_to_json(&params.provider, /*existing*/ None)?;
        self.validate_draft_provider(&params.id, &provider_value, params.default_model.as_ref())
            .await?;

        let mut edits = vec![ConfigEdit {
            key_path: format!("model_providers.{}", params.id),
            value: provider_value,
            merge_strategy: MergeStrategy::Replace,
        }];
        let warnings = append_default_selection_edits(
            &mut edits,
            params.set_default,
            params.default_model.as_ref(),
            &params.id,
        );

        let _ = self
            .config_processor
            .batch_write(ConfigBatchWriteParams {
                edits,
                file_path: None,
                expected_version: None,
                reload_user_config: true,
            })
            .await?;

        let ProviderReadResponse { provider } = self
            .config_processor
            .provider_read(ProviderReadParams {
                id: params.id,
                cwd: None,
            })
            .await?;
        Ok(ProviderCreateResponse { provider, warnings })
    }

    pub(crate) async fn update(
        &self,
        params: ProviderUpdateParams,
    ) -> Result<ProviderUpdateResponse, codex_app_server_protocol::JSONRPCErrorError> {
        self.ensure_custom_provider_id(&params.id)?;
        let config = self.load_latest_config().await?;
        let existing_provider = config
            .model_providers
            .get(&params.id)
            .ok_or_else(|| invalid_request(format!("unknown provider id: {}", params.id)))?;
        let existing_value = serde_json::to_value(existing_provider)
            .map_err(|err| internal_error(format!("failed to serialize provider info: {err}")))?;

        let provider_value = provider_input_to_json(&params.provider, Some(&existing_value))?;
        self.validate_draft_provider(&params.id, &provider_value, params.default_model.as_ref())
            .await?;

        let mut edits = vec![ConfigEdit {
            key_path: format!("model_providers.{}", params.id),
            value: provider_value,
            merge_strategy: MergeStrategy::Replace,
        }];
        let warnings = append_default_selection_edits(
            &mut edits,
            params.set_default,
            params.default_model.as_ref(),
            &params.id,
        );

        let _ = self
            .config_processor
            .batch_write(ConfigBatchWriteParams {
                edits,
                file_path: None,
                expected_version: None,
                reload_user_config: true,
            })
            .await?;

        let ProviderReadResponse { provider } = self
            .config_processor
            .provider_read(ProviderReadParams {
                id: params.id,
                cwd: None,
            })
            .await?;
        Ok(ProviderUpdateResponse { provider, warnings })
    }

    pub(crate) async fn delete(
        &self,
        params: ProviderDeleteParams,
    ) -> Result<ProviderDeleteResponse, codex_app_server_protocol::JSONRPCErrorError> {
        self.ensure_custom_provider_id(&params.id)?;
        let config = self.load_latest_config().await?;
        if !config.model_providers.contains_key(&params.id) {
            return Err(invalid_request(format!(
                "unknown provider id: {}",
                params.id
            )));
        }
        if config.model_provider_id == params.id {
            return Err(invalid_request(format!(
                "cannot delete currently selected provider: {}",
                params.id
            )));
        }

        let _ = self
            .config_processor
            .value_write(ConfigValueWriteParams {
                key_path: format!("model_providers.{}", params.id),
                value: JsonValue::Null,
                merge_strategy: MergeStrategy::Replace,
                file_path: None,
                expected_version: None,
            })
            .await?;

        let warning = self
            .config_processor
            .provider_read(ProviderReadParams {
                id: params.id,
                cwd: None,
            })
            .await
            .ok()
            .map(|_| "Provider is still defined in a lower-precedence config layer.".to_string());
        Ok(ProviderDeleteResponse {
            deleted: true,
            warning,
        })
    }

    pub(crate) async fn test_connection(
        &self,
        params: ProviderTestConnectionParams,
    ) -> Result<ProviderTestConnectionResponse, codex_app_server_protocol::JSONRPCErrorError> {
        let checks = normalize_checks(params.checks);
        let (config, provider_id) = match &params.target {
            ProviderTestConnectionTarget::Saved { id } => {
                (self.load_latest_config().await?, id.clone())
            }
            ProviderTestConnectionTarget::Draft { id, provider } => (
                self.resolve_draft_config(id, provider, &params.model)
                    .await?,
                id.clone(),
            ),
        };

        let provider_info = config
            .model_providers
            .get(&provider_id)
            .cloned()
            .ok_or_else(|| invalid_request(format!("unknown provider id: {provider_id}")))?;
        let runtime_provider =
            create_model_provider(provider_info.clone(), Some(self.auth_manager.clone()));

        if provider_info.requires_openai_auth && runtime_provider.auth().await.is_none() {
            return Ok(failed_connection_response(
                &checks,
                ProviderTestConnectionErrorCode::FailAuth,
                "OpenAI auth is required but no account is currently logged in.".to_string(),
                None,
            ));
        }

        let api_provider = match runtime_provider.api_provider().await {
            Ok(provider) => provider,
            Err(err) => {
                return Ok(failed_connection_response(
                    &checks,
                    provider_error_code(&err),
                    err.to_string(),
                    None,
                ));
            }
        };
        let api_auth = match runtime_provider.api_auth().await {
            Ok(auth) => auth,
            Err(err) => {
                return Ok(failed_connection_response(
                    &checks,
                    provider_error_code(&err),
                    err.to_string(),
                    None,
                ));
            }
        };
        let wire_api = provider_wire_api(&provider_info)?;

        let mut results = Vec::new();
        for check in requested_checks(&checks) {
            results.push(
                run_provider_probe(&api_provider, &api_auth, wire_api, &params.model, check).await,
            );
        }

        let ok = results.iter().all(|result| result.ok);
        let summary = if ok {
            "All requested checks passed.".to_string()
        } else {
            let failures = results.iter().filter(|result| !result.ok).count();
            format!("{failures} requested check(s) failed.")
        };
        Ok(ProviderTestConnectionResponse {
            ok,
            summary,
            results,
        })
    }

    pub(crate) async fn preferences_read(
        &self,
        params: ProviderPreferencesReadParams,
    ) -> Result<ProviderPreferencesReadResponse, codex_app_server_protocol::JSONRPCErrorError> {
        let config = self
            .config_manager
            .load_latest_config(params.cwd.as_ref().map(PathBuf::from))
            .await
            .map_err(|err| internal_error(format!("failed to load latest config: {err}")))?;
        let read = self
            .config_processor
            .read(ConfigReadParams {
                include_layers: true,
                cwd: params.cwd,
            })
            .await?;

        Ok(ProviderPreferencesReadResponse {
            default_provider: config.model_provider_id,
            default_model: config.model,
            config_scope: preferences_scope_from_read(&read),
        })
    }

    pub(crate) async fn preferences_update(
        &self,
        params: ProviderPreferencesUpdateParams,
    ) -> Result<ProviderPreferencesUpdateResponse, codex_app_server_protocol::JSONRPCErrorError>
    {
        let config = self.load_latest_config().await?;
        if !config
            .model_providers
            .contains_key(&params.default_provider)
        {
            return Err(invalid_request(format!(
                "unknown provider id: {}",
                params.default_provider
            )));
        }

        let file_path = match params.config_scope {
            ProviderPreferencesScope::Global => None,
            ProviderPreferencesScope::Project => {
                let cwd = params
                    .cwd
                    .as_ref()
                    .ok_or_else(|| invalid_request("cwd is required for project scope"))?;
                let path = project_config_path_for_cwd(self, cwd).await?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).await.map_err(|err| {
                        internal_error(format!("failed to create project config directory: {err}"))
                    })?;
                }
                Some(path.display().to_string())
            }
        };

        let mut edits = vec![ConfigEdit {
            key_path: "model_provider".to_string(),
            value: JsonValue::String(params.default_provider.clone()),
            merge_strategy: MergeStrategy::Replace,
        }];
        edits.push(ConfigEdit {
            key_path: "model".to_string(),
            value: params
                .default_model
                .clone()
                .map(JsonValue::String)
                .unwrap_or(JsonValue::Null),
            merge_strategy: MergeStrategy::Replace,
        });

        let _ = self
            .config_processor
            .batch_write(ConfigBatchWriteParams {
                edits,
                file_path,
                expected_version: None,
                reload_user_config: true,
            })
            .await?;

        Ok(ProviderPreferencesUpdateResponse {
            default_provider: params.default_provider,
            default_model: params.default_model,
            config_scope: params.config_scope,
        })
    }

    async fn load_latest_config(
        &self,
    ) -> Result<codex_core::config::Config, codex_app_server_protocol::JSONRPCErrorError> {
        self.config_manager
            .load_latest_config(/*fallback_cwd*/ None)
            .await
            .map_err(|err| internal_error(format!("failed to load latest config: {err}")))
    }

    async fn resolve_draft_config(
        &self,
        id: &str,
        provider: &ProviderConfigParams,
        model: &str,
    ) -> Result<codex_core::config::Config, codex_app_server_protocol::JSONRPCErrorError> {
        self.ensure_custom_provider_id(id)?;
        let provider_value = provider_input_to_json(provider, /*existing*/ None)?;
        let overrides = HashMap::from([
            ("model".to_string(), JsonValue::String(model.to_string())),
            (
                "model_provider".to_string(),
                JsonValue::String(id.to_string()),
            ),
            (format!("model_providers.{id}"), provider_value),
        ]);
        self.config_manager
            .load_with_overrides(Some(overrides), ConfigOverrides::default())
            .await
            .map_err(|err| invalid_request(format!("invalid provider config: {err}")))
    }

    async fn validate_draft_provider(
        &self,
        id: &str,
        provider_value: &JsonValue,
        default_model: Option<&String>,
    ) -> Result<(), codex_app_server_protocol::JSONRPCErrorError> {
        let model = default_model
            .map(std::string::String::as_str)
            .unwrap_or("provider-test-model");
        let overrides = HashMap::from([
            ("model".to_string(), JsonValue::String(model.to_string())),
            (
                "model_provider".to_string(),
                JsonValue::String(id.to_string()),
            ),
            (format!("model_providers.{id}"), provider_value.clone()),
        ]);
        self.config_manager
            .load_with_overrides(Some(overrides), ConfigOverrides::default())
            .await
            .map(|_| ())
            .map_err(|err| invalid_request(format!("invalid provider config: {err}")))
    }

    fn ensure_custom_provider_id(
        &self,
        id: &str,
    ) -> Result<(), codex_app_server_protocol::JSONRPCErrorError> {
        if BUILTIN_PROVIDER_IDS.contains(&id) {
            return Err(invalid_request(format!(
                "builtin provider ids are immutable: {id}"
            )));
        }
        if id.trim().is_empty() {
            return Err(invalid_request("provider id must not be empty"));
        }
        Ok(())
    }
}

fn append_default_selection_edits(
    edits: &mut Vec<ConfigEdit>,
    set_default: bool,
    default_model: Option<&String>,
    provider_id: &str,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if set_default {
        edits.push(ConfigEdit {
            key_path: "model_provider".to_string(),
            value: JsonValue::String(provider_id.to_string()),
            merge_strategy: MergeStrategy::Replace,
        });
        if let Some(default_model) = default_model {
            edits.push(ConfigEdit {
                key_path: "model".to_string(),
                value: JsonValue::String(default_model.clone()),
                merge_strategy: MergeStrategy::Replace,
            });
        } else {
            warnings.push(
                "Default provider updated, but default model was left unchanged.".to_string(),
            );
        }
    }
    warnings
}

async fn project_config_path_for_cwd(
    processor: &ProviderRequestProcessor,
    cwd: &str,
) -> Result<PathBuf, codex_app_server_protocol::JSONRPCErrorError> {
    let read = processor
        .config_processor
        .read(ConfigReadParams {
            include_layers: true,
            cwd: Some(cwd.to_string()),
        })
        .await?;
    if let Some(layers) = read.layers {
        for layer in layers {
            if let ConfigLayerSource::Project { dot_codex_folder } = layer.name {
                return Ok(dot_codex_folder.as_path().join("config.toml"));
            }
        }
    }
    Ok(PathBuf::from(cwd).join(".codex").join("config.toml"))
}

fn preferences_scope_from_read(
    read: &codex_app_server_protocol::ConfigReadResponse,
) -> ProviderPreferencesScope {
    let model_origin = read.origins.get("model");
    let provider_origin = read.origins.get("model_provider");
    if matches!(
        model_origin.map(|origin| &origin.name),
        Some(ConfigLayerSource::Project { .. })
    ) || matches!(
        provider_origin.map(|origin| &origin.name),
        Some(ConfigLayerSource::Project { .. })
    ) {
        ProviderPreferencesScope::Project
    } else {
        ProviderPreferencesScope::Global
    }
}

fn normalize_checks(checks: Option<ProviderTestConnectionChecks>) -> ProviderTestConnectionChecks {
    let checks = checks.unwrap_or(ProviderTestConnectionChecks {
        basic: true,
        streaming: false,
        tool_calling: false,
    });
    if !checks.basic && !checks.streaming && !checks.tool_calling {
        ProviderTestConnectionChecks {
            basic: true,
            streaming: false,
            tool_calling: false,
        }
    } else {
        checks
    }
}

fn requested_checks(checks: &ProviderTestConnectionChecks) -> Vec<ProviderConnectionCheck> {
    let mut requested = Vec::new();
    if checks.basic {
        requested.push(ProviderConnectionCheck::Basic);
    }
    if checks.streaming {
        requested.push(ProviderConnectionCheck::Streaming);
    }
    if checks.tool_calling {
        requested.push(ProviderConnectionCheck::ToolCalling);
    }
    requested
}

fn failed_connection_response(
    checks: &ProviderTestConnectionChecks,
    error_code: ProviderTestConnectionErrorCode,
    message: String,
    http_status: Option<u16>,
) -> ProviderTestConnectionResponse {
    let results = requested_checks(checks)
        .into_iter()
        .map(|check| ProviderTestConnectionCheckResult {
            check,
            ok: false,
            error_code: Some(error_code),
            message: Some(message.clone()),
            http_status,
        })
        .collect::<Vec<_>>();
    ProviderTestConnectionResponse {
        ok: false,
        summary: message,
        results,
    }
}

async fn run_provider_probe(
    api_provider: &codex_api::Provider,
    api_auth: &codex_api::SharedAuthProvider,
    wire_api: ProviderWireApi,
    model: &str,
    check: ProviderConnectionCheck,
) -> ProviderTestConnectionCheckResult {
    let path = match wire_api {
        ProviderWireApi::Responses => "responses",
        ProviderWireApi::AnthropicMessages => "messages",
        ProviderWireApi::ChatCompletions => "chat/completions",
    };
    let url = api_provider.url_for_path(path);
    if let Err(err) = Url::parse(&url) {
        return ProviderTestConnectionCheckResult {
            check,
            ok: false,
            error_code: Some(ProviderTestConnectionErrorCode::InvalidBaseUrl),
            message: Some(err.to_string()),
            http_status: None,
        };
    }

    let mut headers = api_provider.headers.clone();
    headers.extend(api_auth.to_auth_headers());
    if matches!(check, ProviderConnectionCheck::Streaming) {
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
    }

    let body = probe_request_body(wire_api, model, check);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TEST_CONNECTION_TIMEOUT_SECS))
        .build();
    let client = match client {
        Ok(client) => client,
        Err(err) => {
            return ProviderTestConnectionCheckResult {
                check,
                ok: false,
                error_code: Some(ProviderTestConnectionErrorCode::FailNetwork),
                message: Some(err.to_string()),
                http_status: None,
            };
        }
    };

    let response = client.post(url).headers(headers).json(&body).send().await;
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            return ProviderTestConnectionCheckResult {
                check,
                ok: false,
                error_code: Some(classify_transport_error(&err)),
                message: Some(err.to_string()),
                http_status: None,
            };
        }
    };

    let status = response.status();
    let body_text = response.text().await.unwrap_or_default();
    if status.is_success() {
        if matches!(check, ProviderConnectionCheck::Streaming) && body_text.trim().is_empty() {
            return ProviderTestConnectionCheckResult {
                check,
                ok: false,
                error_code: Some(ProviderTestConnectionErrorCode::FailStream),
                message: Some("streaming response returned an empty body".to_string()),
                http_status: Some(status.as_u16()),
            };
        }
        return ProviderTestConnectionCheckResult {
            check,
            ok: true,
            error_code: None,
            message: None,
            http_status: Some(status.as_u16()),
        };
    }

    ProviderTestConnectionCheckResult {
        check,
        ok: false,
        error_code: Some(classify_http_failure(status, &body_text, check)),
        message: Some(body_text),
        http_status: Some(status.as_u16()),
    }
}

fn classify_transport_error(err: &reqwest::Error) -> ProviderTestConnectionErrorCode {
    let lower = err.to_string().to_ascii_lowercase();
    if lower.contains("dns") || lower.contains("lookup") {
        ProviderTestConnectionErrorCode::FailDns
    } else if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        ProviderTestConnectionErrorCode::FailTls
    } else if lower.contains("builder error") || lower.contains("relative url") {
        ProviderTestConnectionErrorCode::InvalidBaseUrl
    } else {
        ProviderTestConnectionErrorCode::FailNetwork
    }
}

fn classify_http_failure(
    status: StatusCode,
    body: &str,
    check: ProviderConnectionCheck,
) -> ProviderTestConnectionErrorCode {
    let lower = body.to_ascii_lowercase();
    match status {
        StatusCode::UNAUTHORIZED => ProviderTestConnectionErrorCode::FailAuth,
        StatusCode::FORBIDDEN => {
            if lower.contains("model") || lower.contains("region") || lower.contains("available") {
                ProviderTestConnectionErrorCode::FailModel
            } else {
                ProviderTestConnectionErrorCode::FailAuth
            }
        }
        StatusCode::NOT_FOUND => ProviderTestConnectionErrorCode::FailEndpoint,
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            if matches!(check, ProviderConnectionCheck::ToolCalling)
                && (lower.contains("tool") || lower.contains("function"))
            {
                ProviderTestConnectionErrorCode::FailTool
            } else if lower.contains("model") {
                ProviderTestConnectionErrorCode::FailModel
            } else {
                ProviderTestConnectionErrorCode::FailSchema
            }
        }
        _ => ProviderTestConnectionErrorCode::FailNetwork,
    }
}

fn provider_input_to_json(
    input: &ProviderConfigParams,
    existing: Option<&JsonValue>,
) -> Result<JsonValue, codex_app_server_protocol::JSONRPCErrorError> {
    if input.display_name.trim().is_empty() {
        return Err(invalid_request("provider displayName must not be empty"));
    }
    if input.base_url.trim().is_empty() {
        return Err(invalid_request("provider baseUrl must not be empty"));
    }
    if input
        .api_key
        .as_ref()
        .is_some_and(|value| !value.is_empty())
        && input
            .env_key
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(invalid_request(
            "provider envKey and apiKey cannot both be set",
        ));
    }

    let mut object = existing
        .and_then(JsonValue::as_object)
        .cloned()
        .unwrap_or_default();
    for key in [
        "name",
        "base_url",
        "env_key",
        "env_key_instructions",
        "experimental_bearer_token",
        "wire_api",
        "auth_style",
        "auth",
        "aws",
        "query_params",
        "http_headers",
        "env_http_headers",
        "request_max_retries",
        "stream_max_retries",
        "stream_idle_timeout_ms",
        "websocket_connect_timeout_ms",
        "requires_openai_auth",
        "supports_websockets",
    ] {
        object.remove(key);
    }

    object.insert(
        "name".to_string(),
        JsonValue::String(input.display_name.clone()),
    );
    object.insert(
        "base_url".to_string(),
        JsonValue::String(input.base_url.clone()),
    );
    object.insert(
        "wire_api".to_string(),
        JsonValue::String(
            match input.wire_api {
                ProviderWireApi::Responses => "responses",
                ProviderWireApi::AnthropicMessages => "anthropic_messages",
                ProviderWireApi::ChatCompletions => "chat_completions",
            }
            .to_string(),
        ),
    );
    object.insert(
        "auth_style".to_string(),
        JsonValue::String(
            match input.auth_style {
                ProviderAuthStyle::Bearer => "bearer",
                ProviderAuthStyle::XApiKey => "x_api_key",
            }
            .to_string(),
        ),
    );
    object.insert(
        "requires_openai_auth".to_string(),
        JsonValue::Bool(input.requires_openai_auth),
    );
    object.insert(
        "supports_websockets".to_string(),
        JsonValue::Bool(input.supports_websockets),
    );

    if let Some(env_key) = input
        .env_key
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        object.insert("env_key".to_string(), JsonValue::String(env_key.clone()));
    }
    match input.api_key.as_deref() {
        Some("") => {}
        Some(api_key) => {
            object.insert(
                "experimental_bearer_token".to_string(),
                JsonValue::String(api_key.to_string()),
            );
        }
        None => {
            if input
                .env_key
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                if existing
                    .and_then(JsonValue::as_object)
                    .and_then(|object| object.get("env_key"))
                    .and_then(JsonValue::as_str)
                    == input.env_key.as_deref()
                    && let Some(existing_token) = existing
                        .and_then(JsonValue::as_object)
                        .and_then(|object| object.get("experimental_bearer_token"))
                {
                    object.insert(
                        "experimental_bearer_token".to_string(),
                        existing_token.clone(),
                    );
                }
            } else if let Some(existing_token) = existing
                .and_then(JsonValue::as_object)
                .and_then(|object| object.get("experimental_bearer_token"))
            {
                object.insert(
                    "experimental_bearer_token".to_string(),
                    existing_token.clone(),
                );
            }
        }
    }

    if let Some(value) = input.request_max_retries {
        object.insert(
            "request_max_retries".to_string(),
            JsonValue::Number(value.into()),
        );
    }
    if let Some(value) = input.stream_max_retries {
        object.insert(
            "stream_max_retries".to_string(),
            JsonValue::Number(value.into()),
        );
    }
    if let Some(value) = input.stream_idle_timeout_ms {
        object.insert(
            "stream_idle_timeout_ms".to_string(),
            JsonValue::Number(value.into()),
        );
    }
    if let Some(value) = input.websocket_connect_timeout_ms {
        object.insert(
            "websocket_connect_timeout_ms".to_string(),
            JsonValue::Number(value.into()),
        );
    }
    if let Some(headers) = &input.headers
        && !headers.is_empty()
    {
        object.insert(
            "http_headers".to_string(),
            JsonValue::Object(
                headers
                    .iter()
                    .map(|header| (header.name.clone(), JsonValue::String(header.value.clone())))
                    .collect::<JsonMap<String, JsonValue>>(),
            ),
        );
    }
    if let Some(headers) = &input.env_headers
        && !headers.is_empty()
    {
        object.insert(
            "env_http_headers".to_string(),
            JsonValue::Object(
                headers
                    .iter()
                    .map(|header| {
                        (
                            header.name.clone(),
                            JsonValue::String(header.env_var.clone()),
                        )
                    })
                    .collect::<JsonMap<String, JsonValue>>(),
            ),
        );
    }

    Ok(JsonValue::Object(object))
}

fn provider_error_code(err: &CodexErr) -> ProviderTestConnectionErrorCode {
    match err {
        CodexErr::EnvVar(_) => ProviderTestConnectionErrorCode::MissingApiKey,
        CodexErr::InvalidRequest(_) => ProviderTestConnectionErrorCode::FailSchema,
        CodexErr::UnsupportedOperation(_) => ProviderTestConnectionErrorCode::UnsupportedWireApi,
        _ => ProviderTestConnectionErrorCode::FailNetwork,
    }
}

fn provider_wire_api(
    provider: &impl Serialize,
) -> Result<ProviderWireApi, codex_app_server_protocol::JSONRPCErrorError> {
    let wire_api = serialized_provider_string_field(provider, "wire_api")?;
    match wire_api.as_str() {
        "responses" => Ok(ProviderWireApi::Responses),
        "anthropic_messages" => Ok(ProviderWireApi::AnthropicMessages),
        "chat_completions" => Ok(ProviderWireApi::ChatCompletions),
        _ => Err(internal_error(format!(
            "unexpected provider wire_api serialization: {wire_api}"
        ))),
    }
}

fn serialized_provider_string_field(
    provider: &impl Serialize,
    field_name: &str,
) -> Result<String, codex_app_server_protocol::JSONRPCErrorError> {
    let provider_value = serde_json::to_value(provider)
        .map_err(|err| internal_error(format!("failed to serialize provider info: {err}")))?;
    provider_value
        .get(field_name)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            internal_error(format!(
                "provider serialization omitted required field `{field_name}`"
            ))
        })
}

fn probe_request_body(
    wire_api: ProviderWireApi,
    model: &str,
    check: ProviderConnectionCheck,
) -> JsonValue {
    let stream = matches!(check, ProviderConnectionCheck::Streaming);
    match wire_api {
        ProviderWireApi::Responses => {
            let mut body = serde_json::json!({
                "model": model,
                "input": if matches!(check, ProviderConnectionCheck::ToolCalling) {
                    TOOL_TEST_PROMPT
                } else {
                    BASIC_TEST_PROMPT
                },
                "stream": stream,
                "store": false
            });
            if matches!(check, ProviderConnectionCheck::ToolCalling) {
                body["tools"] = serde_json::json!([{
                    "type": "function",
                    "name": "ping",
                    "description": "Ping tool",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }
                }]);
                body["tool_choice"] = JsonValue::String("auto".to_string());
                body["parallel_tool_calls"] = JsonValue::Bool(false);
            }
            body
        }
        ProviderWireApi::AnthropicMessages => {
            let mut body = serde_json::json!({
                "model": model,
                "max_tokens": 64,
                "system": "",
                "messages": [{
                    "role": "user",
                    "content": [{
                        "type": "text",
                        "text": if matches!(check, ProviderConnectionCheck::ToolCalling) {
                            TOOL_TEST_PROMPT
                        } else {
                            BASIC_TEST_PROMPT
                        }
                    }]
                }],
                "stream": stream
            });
            if matches!(check, ProviderConnectionCheck::ToolCalling) {
                body["tools"] = serde_json::json!([{
                    "name": "ping",
                    "description": "Ping tool",
                    "input_schema": {
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }
                }]);
            }
            body
        }
        ProviderWireApi::ChatCompletions => {
            let mut body = serde_json::json!({
                "model": model,
                "messages": [{
                    "role": "user",
                    "content": if matches!(check, ProviderConnectionCheck::ToolCalling) {
                        TOOL_TEST_PROMPT
                    } else {
                        BASIC_TEST_PROMPT
                    }
                }],
                "stream": stream
            });
            if matches!(check, ProviderConnectionCheck::ToolCalling) {
                body["tools"] = serde_json::json!([{
                    "type": "function",
                    "function": {
                        "name": "ping",
                        "description": "Ping tool",
                        "parameters": {
                            "type": "object",
                            "properties": {},
                            "additionalProperties": false
                        }
                    }
                }]);
                body["tool_choice"] = JsonValue::String("auto".to_string());
            }
            body
        }
    }
}
