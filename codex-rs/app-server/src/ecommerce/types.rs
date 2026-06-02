use std::collections::HashMap;

pub(crate) use codex_app_server_protocol::EcommerceAgentSubmitParams;
pub(crate) use codex_app_server_protocol::EcommerceAgentSubmitResponse;
pub(crate) use codex_app_server_protocol::EcommerceAgentStepResult;
pub(crate) use codex_app_server_protocol::EcommerceAgentType;

pub(crate) type WorkflowContext = HashMap<String, serde_json::Value>;
