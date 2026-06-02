use std::path::Path;
use std::path::PathBuf;

use codex_app_server_protocol::ApprovalsReviewer;
use codex_app_server_protocol::AskForApproval;
use codex_app_server_protocol::DevflowApprovalPolicy;
use codex_app_server_protocol::DevflowTaskRiskLevel;
use tokio::fs;

pub(crate) fn default_approval_policy() -> DevflowApprovalPolicy {
    DevflowApprovalPolicy {
        low_risk_approval_policy: AskForApproval::Never,
        medium_risk_approval_policy: AskForApproval::OnFailure,
        high_risk_approval_policy: AskForApproval::OnRequest,
        approvals_reviewer: ApprovalsReviewer::User,
    }
}

pub(crate) async fn load_approval_policy(
    codex_home: &Path,
) -> Result<DevflowApprovalPolicy, String> {
    let path = approval_policy_path(codex_home);
    match fs::read_to_string(&path).await {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|err| format!("invalid devflow approval policy file: {err}")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(default_approval_policy()),
        Err(err) => Err(format!("failed to read devflow approval policy: {err}")),
    }
}

pub(crate) async fn save_approval_policy(
    codex_home: &Path,
    policy: &DevflowApprovalPolicy,
) -> Result<(), String> {
    let path = approval_policy_path(codex_home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create devflow policy dir: {err}"))?;
    }
    let contents = serde_json::to_string_pretty(policy)
        .map_err(|err| format!("failed to serialize devflow approval policy: {err}"))?;
    fs::write(&path, contents)
        .await
        .map_err(|err| format!("failed to write devflow approval policy: {err}"))
}

pub(crate) fn approval_policy_for_risk(
    policy: &DevflowApprovalPolicy,
    risk_level: DevflowTaskRiskLevel,
) -> AskForApproval {
    match risk_level {
        DevflowTaskRiskLevel::Low => policy.low_risk_approval_policy,
        DevflowTaskRiskLevel::Medium => policy.medium_risk_approval_policy,
        DevflowTaskRiskLevel::High => policy.high_risk_approval_policy,
    }
}

fn approval_policy_path(codex_home: &Path) -> PathBuf {
    codex_home.join("devflow").join("approval-policy.json")
}
