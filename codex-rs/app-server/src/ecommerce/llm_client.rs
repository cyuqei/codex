use codex_app_server_protocol::JSONRPCErrorError;
use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

/// Maximum retry attempts for transient errors.
const MAX_RETRIES: u32 = 3;
/// Initial backoff delay in milliseconds.
const INITIAL_BACKOFF_MS: u64 = 2_000;

/// Supported LLM protocol.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LlmProtocol {
    /// OpenAI-compatible chat completions API (POST /chat/completions)
    OpenAiCompat,
    /// Anthropic messages API (POST /messages)
    Anthropic,
}

/// Resolved LLM configuration from the codex config.
#[derive(Debug)]
pub(crate) struct LlmConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub protocol: LlmProtocol,
    pub default_model: String,
}

impl LlmConfig {
    /// Resolve configuration from multiple sources in priority order:
    /// 1. Explicit env vars (LLM_API_KEY, LLM_BASE_URL, etc.) — highest priority
    /// 2. ~/.codex/config.toml + auth.json — user's codex model provider config
    /// 3. No hardcoded defaults — returns None if nothing is configured
    pub fn resolve() -> Option<Self> {
        // Priority 1: explicit env overrides
        if env::var("LLM_API_KEY").is_ok() || env::var("LLM_BASE_URL").is_ok() {
            let api_key = env::var("LLM_API_KEY").ok();
            let base_url = env::var("LLM_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
            let protocol = Self::protocol_from_env();
            let default_model = env::var("DEFAULT_MODEL")
                .unwrap_or_else(|_| "gpt-4o".to_string());
            return Some(Self {
                api_key,
                base_url,
                protocol,
                default_model,
            });
        }

        // Priority 2: read from ~/.codex/config.toml
        if let Some(config) = Self::from_codex_config() {
            return Some(config);
        }

        None
    }

    fn protocol_from_env() -> LlmProtocol {
        let protocol_str = env::var("LLM_PROTOCOL")
            .unwrap_or_else(|_| "openai_compat".to_string());
        if protocol_str.eq_ignore_ascii_case("anthropic") {
            LlmProtocol::Anthropic
        } else {
            LlmProtocol::OpenAiCompat
        }
    }

    fn from_codex_config() -> Option<Self> {
        let codex_home = dirs::home_dir()?.join(".codex");
        let config_path = codex_home.join("config.toml");
        let auth_path = codex_home.join("auth.json");

        let config_text = std::fs::read_to_string(&config_path).ok()?;
        let config: toml::Table = config_text.parse().ok()?;

        // Get active model provider name
        let provider_name = config.get("model_provider")?.as_str()?;
        let default_model = config.get("model").and_then(|v| v.as_str());

        // Get provider config section
        let providers = config.get("model_providers")?.as_table()?;
        let provider = providers.get(provider_name)?.as_table()?;

        // Extract base_url
        let base_url = provider
            .get("base_url")
            .and_then(|v| v.as_str())?
            .to_string();

        // Determine protocol from wire_api
        let protocol = match provider
            .get("wire_api")
            .and_then(|v| v.as_str())
            .unwrap_or("chat_completions")
        {
            "chat_completions" => LlmProtocol::OpenAiCompat,
            "messages" => LlmProtocol::Anthropic,
            _ => LlmProtocol::OpenAiCompat,
        };

        // Resolve API key: first try env_key from provider config, then auth.json
        let api_key = Self::resolve_api_key(provider, &auth_path);

        Some(Self {
            api_key,
            base_url,
            protocol,
            default_model: default_model.unwrap_or(provider_name).to_string(),
        })
    }

    fn resolve_api_key(provider: &toml::Table, auth_path: &PathBuf) -> Option<String> {
        // Try env_key from provider config
        if let Some(env_key) = provider.get("env_key").and_then(|v| v.as_str()) {
            if let Ok(key) = env::var(env_key) {
                return Some(key);
            }
        }

        // Fallback: read from auth.json — try all keys
        if let Ok(auth_text) = std::fs::read_to_string(auth_path) {
            if let Ok(auth) = serde_json::from_str::<serde_json::Value>(&auth_text) {
                // Try provider's env_key first, then common key names
                let key_names = [
                    provider
                        .get("env_key")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                    "OPENAI_API_KEY",
                    "ANTHROPIC_API_KEY",
                    "LLM_API_KEY",
                    "INFERAICHAT_API_KEY",
                ];
                for name in key_names {
                    if !name.is_empty() {
                        if let Some(key) = auth.get(name).and_then(|v| v.as_str()) {
                            return Some(key.to_string());
                        }
                    }
                }
            }
        }

        None
    }
}

/// A generic LLM HTTP client for ecommerce agent calls.
///
/// Configuration is resolved automatically from:
/// 1. Environment variables (LLM_API_KEY, LLM_BASE_URL, LLM_PROTOCOL, DEFAULT_MODEL)
/// 2. ~/.codex/config.toml + auth.json (user's codex model provider config)
pub(crate) struct LlmClient {
    config: LlmConfig,
}

impl LlmClient {
    pub fn new() -> Self {
        let config = LlmConfig::resolve().expect(
            "LLM configuration not found. \
             Set LLM_API_KEY + LLM_BASE_URL env vars, or ensure ~/.codex/config.toml \
             has a valid model_provider with base_url and auth.",
        );
        Self { config }
    }

    pub async fn chat(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, JSONRPCErrorError> {
        let model = if model.is_empty() {
            &self.config.default_model
        } else {
            model
        };

        let api_key = self.config.api_key.as_ref().ok_or_else(|| JSONRPCErrorError {
            code: -32001,
            message: "No API key configured. Set LLM_API_KEY or ensure ~/.codex/auth.json has a key".to_string(),
            data: None,
        })?;

        let client = reqwest::Client::new();

        // Retry loop with exponential backoff for transient errors
        let mut backoff = INITIAL_BACKOFF_MS;
        let mut attempt = 0;

        loop {
            let result = match self.config.protocol {
                LlmProtocol::Anthropic => {
                    self.call_anthropic_once(&client, model, system_prompt, user_message, api_key)
                        .await
                }
                LlmProtocol::OpenAiCompat => {
                    self.call_openai_compat_once(&client, model, system_prompt, user_message, api_key)
                        .await
                }
            };

            match result {
                Ok(content) => return Ok(content),
                Err(e) => {
                    let is_retryable = self.is_retryable_error(&e);
                    attempt += 1;
                    if is_retryable && attempt < MAX_RETRIES {
                        eprintln!(
                            "[LlmClient] Attempt {}/{} failed ({}). Retrying in {}ms...",
                            attempt, MAX_RETRIES, e.message, backoff
                        );
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
                        backoff *= 2; // Exponential backoff
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Check if an error is transient and worth retrying.
    fn is_retryable_error(&self, err: &JSONRPCErrorError) -> bool {
        let msg = err.message.to_lowercase();
        // 429 Too Many Requests
        if msg.contains("429") || msg.contains("too many requests") || msg.contains("rate limit") {
            return true;
        }
        // 5xx server errors
        if msg.contains("502") || msg.contains("503") || msg.contains("504") || msg.contains("524") {
            return true;
        }
        // Network/timeout errors
        if msg.contains("failed") || msg.contains("timeout") || msg.contains("connection") {
            return true;
        }
        false
    }

    async fn call_anthropic_once(
        &self,
        client: &reqwest::Client,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        api_key: &str,
    ) -> Result<String, JSONRPCErrorError> {
        let body = json!({
            "model": model,
            "system": system_prompt,
            "messages": [
                {"role": "user", "content": user_message}
            ],
            "max_tokens": 4096,
        });

        let response = client
            .post(format!("{}/messages", self.config.base_url.trim_end_matches('/')))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| JSONRPCErrorError {
                code: -32002,
                message: format!("Anthropic request failed: {}", e),
                data: None,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(JSONRPCErrorError {
                code: -32003,
                message: format!("LLM API error {}: {}", status, text),
                data: None,
            });
        }

        let data: Value = response.json().await.map_err(|e| JSONRPCErrorError {
            code: -32004,
            message: format!("Failed to parse LLM response: {}", e),
            data: None,
        })?;

        let content = data
            .get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        Ok(content)
    }

    async fn call_openai_compat_once(
        &self,
        client: &reqwest::Client,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        api_key: &str,
    ) -> Result<String, JSONRPCErrorError> {
        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message}
            ],
            "temperature": 0.7,
            "max_tokens": 4096
        });

        let response = client
            .post(format!("{}/chat/completions", self.config.base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| JSONRPCErrorError {
                code: -32002,
                message: format!("OpenAI-compat request failed: {}", e),
                data: None,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(JSONRPCErrorError {
                code: -32003,
                message: format!("LLM API error {}: {}", status, text),
                data: None,
            });
        }

        let data: Value = response.json().await.map_err(|e| JSONRPCErrorError {
            code: -32004,
            message: format!("Failed to parse LLM response: {}", e),
            data: None,
        })?;

        let content = data
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        Ok(content)
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}
