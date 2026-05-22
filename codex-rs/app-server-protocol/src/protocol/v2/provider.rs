use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

const OPENAI_PROVIDER_ID: &str = "openai";
const AMAZON_BEDROCK_PROVIDER_ID: &str = "amazon-bedrock";
const OLLAMA_OSS_PROVIDER_ID: &str = "ollama";
const LMSTUDIO_OSS_PROVIDER_ID: &str = "lmstudio";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ProviderSource {
    Builtin,
    Custom,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum ProviderWireApi {
    Responses,
    AnthropicMessages,
    ChatCompletions,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum ProviderAuthStyle {
    Bearer,
    XApiKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderHeaderSummary {
    pub name: String,
    pub value_source: ProviderHeaderValueSource,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ProviderHeaderValueSource {
    Literal,
    Env,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct Provider {
    pub id: String,
    pub display_name: String,
    pub source: ProviderSource,
    pub builtin_kind: Option<String>,
    pub base_url: Option<String>,
    pub wire_api: ProviderWireApi,
    pub auth_style: ProviderAuthStyle,
    pub env_key: Option<String>,
    pub has_api_key: bool,
    pub supports_websockets: bool,
    pub requires_openai_auth: bool,
    pub request_max_retries: Option<u64>,
    pub stream_max_retries: Option<u64>,
    pub stream_idle_timeout_ms: Option<u64>,
    pub websocket_connect_timeout_ms: Option<u64>,
    pub headers: Vec<ProviderHeaderSummary>,
    pub is_default: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderListParams {
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderListResponse {
    pub data: Vec<Provider>,
    pub default_provider: String,
    pub default_model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderReadParams {
    pub id: String,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderReadResponse {
    pub provider: Provider,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderHeaderInput {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderEnvHeaderInput {
    pub name: String,
    pub env_var: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderConfigParams {
    pub display_name: String,
    pub base_url: String,
    pub wire_api: ProviderWireApi,
    pub auth_style: ProviderAuthStyle,
    #[ts(optional = nullable)]
    pub env_key: Option<String>,
    #[ts(optional = nullable)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub requires_openai_auth: bool,
    #[serde(default)]
    pub supports_websockets: bool,
    #[ts(optional = nullable)]
    pub request_max_retries: Option<u64>,
    #[ts(optional = nullable)]
    pub stream_max_retries: Option<u64>,
    #[ts(optional = nullable)]
    pub stream_idle_timeout_ms: Option<u64>,
    #[ts(optional = nullable)]
    pub websocket_connect_timeout_ms: Option<u64>,
    #[ts(optional = nullable)]
    pub headers: Option<Vec<ProviderHeaderInput>>,
    #[ts(optional = nullable)]
    pub env_headers: Option<Vec<ProviderEnvHeaderInput>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderCreateParams {
    pub id: String,
    pub provider: ProviderConfigParams,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub set_default: bool,
    #[ts(optional = nullable)]
    pub default_model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderCreateResponse {
    pub provider: Provider,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderUpdateParams {
    pub id: String,
    pub provider: ProviderConfigParams,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub set_default: bool,
    #[ts(optional = nullable)]
    pub default_model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderUpdateResponse {
    pub provider: Provider,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderDeleteParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderDeleteResponse {
    pub deleted: bool,
    pub warning: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderTestConnectionChecks {
    #[serde(default = "default_enabled")]
    pub basic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub streaming: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tool_calling: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum ProviderTestConnectionTarget {
    Saved {
        id: String,
    },
    Draft {
        id: String,
        provider: Box<ProviderConfigParams>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderTestConnectionParams {
    pub target: ProviderTestConnectionTarget,
    pub model: String,
    #[ts(optional = nullable)]
    pub checks: Option<ProviderTestConnectionChecks>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ProviderConnectionCheck {
    Basic,
    Streaming,
    ToolCalling,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[ts(export_to = "v2/")]
pub enum ProviderTestConnectionErrorCode {
    #[serde(rename = "MISSING_API_KEY")]
    #[ts(rename = "MISSING_API_KEY")]
    MissingApiKey,
    #[serde(rename = "INVALID_BASE_URL")]
    #[ts(rename = "INVALID_BASE_URL")]
    InvalidBaseUrl,
    #[serde(rename = "FAIL_AUTH")]
    #[ts(rename = "FAIL_AUTH")]
    FailAuth,
    #[serde(rename = "FAIL_NETWORK")]
    #[ts(rename = "FAIL_NETWORK")]
    FailNetwork,
    #[serde(rename = "FAIL_DNS")]
    #[ts(rename = "FAIL_DNS")]
    FailDns,
    #[serde(rename = "FAIL_TLS")]
    #[ts(rename = "FAIL_TLS")]
    FailTls,
    #[serde(rename = "FAIL_ENDPOINT")]
    #[ts(rename = "FAIL_ENDPOINT")]
    FailEndpoint,
    #[serde(rename = "FAIL_MODEL")]
    #[ts(rename = "FAIL_MODEL")]
    FailModel,
    #[serde(rename = "FAIL_STREAM")]
    #[ts(rename = "FAIL_STREAM")]
    FailStream,
    #[serde(rename = "FAIL_SCHEMA")]
    #[ts(rename = "FAIL_SCHEMA")]
    FailSchema,
    #[serde(rename = "FAIL_TOOL")]
    #[ts(rename = "FAIL_TOOL")]
    FailTool,
    #[serde(rename = "UNSUPPORTED_WIRE_API")]
    #[ts(rename = "UNSUPPORTED_WIRE_API")]
    UnsupportedWireApi,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderTestConnectionCheckResult {
    pub check: ProviderConnectionCheck,
    pub ok: bool,
    pub error_code: Option<ProviderTestConnectionErrorCode>,
    pub message: Option<String>,
    pub http_status: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderTestConnectionResponse {
    pub ok: bool,
    pub summary: String,
    pub results: Vec<ProviderTestConnectionCheckResult>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ProviderPreferencesScope {
    Global,
    Project,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderPreferencesReadParams {
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderPreferencesReadResponse {
    pub default_provider: String,
    pub default_model: Option<String>,
    pub config_scope: ProviderPreferencesScope,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderPreferencesUpdateParams {
    pub default_provider: String,
    pub default_model: Option<String>,
    pub config_scope: ProviderPreferencesScope,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderPreferencesUpdateResponse {
    pub default_provider: String,
    pub default_model: Option<String>,
    pub config_scope: ProviderPreferencesScope,
}

pub fn builtin_kind_for_provider_id(id: &str) -> Option<String> {
    match id {
        OPENAI_PROVIDER_ID => Some("openai".to_string()),
        AMAZON_BEDROCK_PROVIDER_ID => Some("amazon-bedrock".to_string()),
        OLLAMA_OSS_PROVIDER_ID => Some("ollama".to_string()),
        LMSTUDIO_OSS_PROVIDER_ID => Some("lmstudio".to_string()),
        _ => None,
    }
}
