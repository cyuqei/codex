use async_trait::async_trait;

use super::llm_client::LlmClient;
use super::types::WorkflowContext;
use crate::ecommerce::types::EcommerceAgentStepResult;
use codex_app_server_protocol::JSONRPCErrorError;

#[derive(Debug)]
pub(crate) struct StepOutput {
    pub step_name: String,
    pub model: String,
    pub output: serde_json::Value,
}

#[async_trait]
pub(crate) trait WorkflowStep: Send + Sync {
    fn name(&self) -> &'static str;
    fn model(&self) -> &'static str;
    async fn execute(
        &self,
        ctx: &mut WorkflowContext,
        llm: &LlmClient,
    ) -> Result<StepOutput, JSONRPCErrorError>;
}

pub(crate) fn make_step_result(step: &StepOutput, duration_ms: i64) -> EcommerceAgentStepResult {
    EcommerceAgentStepResult {
        step_name: step.step_name.clone(),
        model: Some(step.model.to_string()),
        duration_ms: Some(duration_ms),
        output_preview: Some(
            serde_json::to_string(&step.output)
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect(),
        ),
    }
}
