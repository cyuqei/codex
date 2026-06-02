use std::sync::Arc;

use codex_agent_identity::AgentIdentityKey;
use codex_agent_identity::AgentTaskAuthorizationTarget;
use codex_agent_identity::authorization_header_for_agent_task;
use codex_api::AuthProvider;
use codex_api::SharedAuthProvider;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_provider_info::AuthStyle;
use codex_model_provider_info::ModelProviderInfo;
use http::HeaderMap;
use http::HeaderName;
use http::HeaderValue;

use crate::bearer_auth_provider::BearerAuthProvider;

#[derive(Clone, Debug)]
struct AgentIdentityAuthProvider {
    auth: codex_login::auth::AgentIdentityAuth,
}

impl AuthProvider for AgentIdentityAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        let record = self.auth.record();
        let header_value = authorization_header_for_agent_task(
            AgentIdentityKey {
                agent_runtime_id: &record.agent_runtime_id,
                private_key_pkcs8_base64: &record.agent_private_key,
            },
            AgentTaskAuthorizationTarget {
                agent_runtime_id: &record.agent_runtime_id,
                task_id: self.auth.process_task_id(),
            },
        )
        .map_err(std::io::Error::other);

        if let Ok(header_value) = header_value
            && let Ok(header) = HeaderValue::from_str(&header_value)
        {
            let _ = headers.insert(http::header::AUTHORIZATION, header);
        }

        if let Ok(header) = HeaderValue::from_str(self.auth.account_id()) {
            let _ = headers.insert("ChatGPT-Account-ID", header);
        }

        if self.auth.is_fedramp_account() {
            let _ = headers.insert("X-OpenAI-Fedramp", HeaderValue::from_static("true"));
        }
    }
}

// Some providers are meant to send no auth headers. Examples include local OSS
// providers and custom test providers with `requires_openai_auth = false`.
#[derive(Clone, Debug)]
struct UnauthenticatedAuthProvider;

impl AuthProvider for UnauthenticatedAuthProvider {
    fn add_auth_headers(&self, _headers: &mut HeaderMap) {}
}

#[derive(Clone, Debug)]
struct HeaderAuthProvider {
    header_name: HeaderName,
    token: String,
}

impl HeaderAuthProvider {
    fn new(header_name: HeaderName, token: String) -> Self {
        Self { header_name, token }
    }
}

impl AuthProvider for HeaderAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        if let Ok(header) = HeaderValue::from_str(&self.token) {
            let _ = headers.insert(self.header_name.clone(), header);
        }
    }
}

pub fn unauthenticated_auth_provider() -> SharedAuthProvider {
    Arc::new(UnauthenticatedAuthProvider)
}

/// Returns the provider-scoped auth manager when this provider uses command-backed auth.
///
/// Providers without custom auth continue using the caller-supplied base manager, when present.
pub(crate) fn auth_manager_for_provider(
    auth_manager: Option<Arc<AuthManager>>,
    provider: &ModelProviderInfo,
) -> Option<Arc<AuthManager>> {
    match provider.auth.clone() {
        Some(config) => Some(AuthManager::external_bearer_only(config)),
        None => auth_manager,
    }
}

pub(crate) fn resolve_provider_auth(
    auth: Option<&CodexAuth>,
    provider: &ModelProviderInfo,
) -> codex_protocol::error::Result<SharedAuthProvider> {
    if let Some(auth) = api_key_auth_for_provider(provider)? {
        return Ok(auth);
    }

    Ok(match auth {
        Some(auth) => auth_provider_from_auth(auth),
        None => unauthenticated_auth_provider(),
    })
}

fn api_key_auth_for_provider(
    provider: &ModelProviderInfo,
) -> codex_protocol::error::Result<Option<SharedAuthProvider>> {
    let token = provider
        .api_key()?
        .or_else(|| provider.experimental_bearer_token.clone());

    let Some(token) = token else {
        return Ok(None);
    };

    let auth: SharedAuthProvider = match provider.auth_style {
        AuthStyle::Bearer => Arc::new(BearerAuthProvider::new(token)),
        AuthStyle::XApiKey => Arc::new(HeaderAuthProvider::new(
            HeaderName::from_static("x-api-key"),
            token,
        )),
    };
    Ok(Some(auth))
}

/// Builds request-header auth for a first-party Codex auth snapshot.
pub fn auth_provider_from_auth(auth: &CodexAuth) -> SharedAuthProvider {
    match auth {
        CodexAuth::AgentIdentity(auth) => {
            Arc::new(AgentIdentityAuthProvider { auth: auth.clone() })
        }
        CodexAuth::ApiKey(_) | CodexAuth::Chatgpt(_) | CodexAuth::ChatgptAuthTokens(_) => {
            Arc::new(BearerAuthProvider {
                token: auth.get_token().ok(),
                account_id: auth.get_account_id(),
                is_fedramp_account: auth.is_fedramp_account(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use codex_model_provider_info::WireApi;
    use codex_model_provider_info::create_oss_provider_with_base_url;

    use super::*;

    #[test]
    fn unauthenticated_auth_provider_adds_no_headers() {
        let provider =
            create_oss_provider_with_base_url("http://localhost:11434/v1", WireApi::Responses);
        let auth = resolve_provider_auth(/*auth*/ None, &provider).expect("auth should resolve");

        assert!(auth.to_auth_headers().is_empty());
    }

    #[test]
    fn x_api_key_auth_style_adds_x_api_key_header() {
        let provider = ModelProviderInfo {
            auth_style: codex_model_provider_info::AuthStyle::XApiKey,
            experimental_bearer_token: Some("test-token".to_string()),
            ..create_oss_provider_with_base_url("http://localhost:11434/v1", WireApi::Responses)
        };
        let auth = resolve_provider_auth(/*auth*/ None, &provider).expect("auth should resolve");
        let headers = auth.to_auth_headers();

        assert_eq!(
            headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok()),
            Some("test-token")
        );
        assert!(!headers.contains_key(http::header::AUTHORIZATION));
    }
}
