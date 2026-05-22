use super::ApprovalsReviewer;
use super::AskForApproval;
use super::PermissionGrantScope;
use super::RequestPermissionProfile;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowAgentRuntime {
    Codex,
    Claude,
    Hermes,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowAgentStatus {
    Available,
    Missing,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgent {
    pub id: String,
    pub name: String,
    pub runtime: DevflowAgentRuntime,
    pub roles: Vec<String>,
    pub root_path: Option<String>,
    pub launch_command: Option<String>,
    pub status: DevflowAgentStatus,
    pub capabilities: Vec<String>,
    pub diagnostics: Vec<String>,
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentDetectParams {
    #[ts(optional = nullable)]
    pub codex_root: Option<String>,
    #[ts(optional = nullable)]
    pub claude_root: Option<String>,
    #[ts(optional = nullable)]
    pub hermes_root: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentDetectResponse {
    pub agents: Vec<DevflowAgent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentListParams {
    #[ts(optional = nullable)]
    pub runtimes: Option<Vec<DevflowAgentRuntime>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentListResponse {
    pub data: Vec<DevflowAgent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentReadResponse {
    pub agent: DevflowAgent,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentCapabilitiesReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentCapabilitiesReadResponse {
    pub id: String,
    pub capabilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentDiagnoseParams {
    pub id: String,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentDiagnoseResponse {
    pub agent: DevflowAgent,
    pub command: String,
    pub ok: bool,
    #[ts(type = "number | null")]
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentStartParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentStartResponse {
    pub agent: DevflowAgent,
    pub started: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentStopParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentStopResponse {
    pub agent: DevflowAgent,
    pub stopped: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentRestartParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentRestartResponse {
    pub agent: DevflowAgent,
    pub restarted: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectMemoryReadParams {
    pub project_root: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectMemoryReadResponse {
    pub project_id: String,
    pub path: String,
    #[ts(type = "string | null")]
    pub summary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectMemoryWriteParams {
    pub project_root: String,
    pub summary: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectMemoryWriteResponse {
    pub project_id: String,
    pub path: String,
    pub summary: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProject {
    pub id: String,
    pub name: String,
    pub root_path: String,
    #[ts(type = "string | null")]
    pub git_remote: Option<String>,
    #[ts(type = "string | null")]
    pub default_branch: Option<String>,
    #[ts(type = "string | null")]
    pub current_branch: Option<String>,
    pub is_trusted: bool,
    pub test_commands: Vec<String>,
    pub detected_docs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectDiagnoseParams {
    pub project_root: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectDiagnoseResponse {
    pub project: DevflowProject,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowSupportBundle {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub mime_type: String,
    pub summary: String,
    pub diagnostics: Vec<String>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowSupportBundleCreateParams {
    pub project_root: String,
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowSupportBundleCreateResponse {
    pub bundle: DevflowSupportBundle,
    pub project: DevflowProject,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowReleasePrepStatus {
    Ready,
    Blocked,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowReleasePrepCreateParams {
    pub project_root: String,
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowReleasePrepCreateResponse {
    pub status: DevflowReleasePrepStatus,
    pub summary: String,
    pub blockers: Vec<String>,
    pub commit_message_artifact: DevflowArtifact,
    pub pr_body_artifact: DevflowArtifact,
    pub release_note_artifact: DevflowArtifact,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectListParams {
    #[ts(optional = nullable)]
    pub project_roots: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectListResponse {
    pub data: Vec<DevflowProject>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectReadParams {
    pub project_root: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectReadResponse {
    pub project: DevflowProject,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectOpenParams {
    pub project_root: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectOpenResponse {
    pub project: DevflowProject,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectTestCommandsListParams {
    pub project_root: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectTestCommandsListResponse {
    pub project_id: String,
    pub commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectTrustParams {
    pub project_root: String,
    pub trusted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowProjectTrustResponse {
    pub project: DevflowProject,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowApprovalKind {
    CommandExecution,
    FileChange,
    Permissions,
    QualityGateWaive,
    ArtifactDelivery,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowApprovalStatus {
    Pending,
    Responded,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowApprovalDecision {
    Accept,
    AcceptForSession,
    AcceptForTask,
    AcceptForProject,
    Decline,
    Cancel,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApproval {
    pub id: String,
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    #[ts(type = "string | null")]
    pub quality_gate_id: Option<String>,
    pub request_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub kind: DevflowApprovalKind,
    pub status: DevflowApprovalStatus,
    #[ts(type = "string | null")]
    pub reason: Option<String>,
    #[ts(type = "string | null")]
    pub command: Option<String>,
    #[ts(type = "string | null")]
    pub cwd: Option<String>,
    pub file_paths: Vec<String>,
    #[ts(type = "RequestPermissionProfile | null")]
    pub requested_permissions: Option<RequestPermissionProfile>,
    #[ts(type = "bigint | null")]
    pub responded_at: Option<i64>,
    #[ts(type = "DevflowApprovalDecision | null")]
    pub decision: Option<DevflowApprovalDecision>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalPolicy {
    pub low_risk_approval_policy: AskForApproval,
    pub medium_risk_approval_policy: AskForApproval,
    pub high_risk_approval_policy: AskForApproval,
    pub approvals_reviewer: ApprovalsReviewer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalListParams {
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
    #[ts(optional = nullable)]
    pub run_id: Option<String>,
    #[ts(optional = nullable)]
    pub status: Option<DevflowApprovalStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalListResponse {
    pub data: Vec<DevflowApproval>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalRespondParams {
    pub id: String,
    pub decision: DevflowApprovalDecision,
    #[ts(optional = nullable)]
    pub scope: Option<PermissionGrantScope>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalRespondResponse {
    pub approval: DevflowApproval,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalPolicyReadParams {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalPolicyReadResponse {
    pub policy: DevflowApprovalPolicy,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalPolicyUpdateParams {
    pub policy: DevflowApprovalPolicy,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalPolicyUpdateResponse {
    pub policy: DevflowApprovalPolicy,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowTaskKind {
    Implementation,
    Review,
    Report,
    Diagnostic,
    Automation,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowTaskRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowPackStatus {
    Available,
    Missing,
    Disabled,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowPolicyPack {
    pub id: String,
    pub name: String,
    #[ts(type = "string | null")]
    pub source_path: Option<String>,
    pub status: DevflowPackStatus,
    pub policies: Vec<String>,
    pub applies_to_risk_levels: Vec<DevflowTaskRiskLevel>,
    pub diagnostics: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowPolicyPackListParams {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_disabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowPolicyPackListResponse {
    pub data: Vec<DevflowPolicyPack>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowPolicyPackReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowPolicyPackReadResponse {
    pub pack: DevflowPolicyPack,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowPolicyPackApplyParams {
    pub id: String,
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
    #[ts(optional = nullable)]
    pub risk_level: Option<DevflowTaskRiskLevel>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowPolicyPackApplyResponse {
    pub pack: DevflowPolicyPack,
    pub applied: bool,
    pub required_artifacts: Vec<String>,
    pub diagnostics: Vec<String>,
    pub artifact: Option<DevflowArtifact>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowCapabilityPack {
    pub id: String,
    pub name: String,
    #[ts(type = "string | null")]
    pub source_path: Option<String>,
    pub status: DevflowPackStatus,
    pub capabilities: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowCapabilityPackListParams {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_disabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowCapabilityPackListResponse {
    pub data: Vec<DevflowCapabilityPack>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowCapabilityPackReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowCapabilityPackReadResponse {
    pub pack: DevflowCapabilityPack,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowCapabilityPackRunStatus {
    Completed,
    Failed,
    Skipped,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowCapabilityPackRunParams {
    pub id: String,
    #[ts(optional = nullable)]
    pub capability: Option<String>,
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
    #[ts(optional = nullable)]
    pub project_root: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowCapabilityPackRunResponse {
    pub pack: DevflowCapabilityPack,
    pub status: DevflowCapabilityPackRunStatus,
    pub summary: String,
    #[ts(type = "DevflowArtifact | null")]
    pub artifact: Option<DevflowArtifact>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowWatchdogStatus {
    Idle,
    Running,
    NoProgress,
    TimedOut,
    Recovering,
    Quarantined,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowWatchdogAlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWatchdogAlert {
    pub id: String,
    pub status: DevflowWatchdogStatus,
    pub severity: DevflowWatchdogAlertSeverity,
    #[ts(type = "string | null")]
    pub project_id: Option<String>,
    #[ts(type = "string | null")]
    pub task_id: Option<String>,
    #[ts(type = "string | null")]
    pub run_id: Option<String>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWatchdogReadParams {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWatchdogReadResponse {
    pub status: DevflowWatchdogStatus,
    pub alerts: Vec<DevflowWatchdogAlert>,
    pub checked_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWatchdogAlertsParams {
    #[ts(optional = nullable)]
    pub status: Option<DevflowWatchdogStatus>,
    #[ts(optional = nullable)]
    pub severity: Option<DevflowWatchdogAlertSeverity>,
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWatchdogAlertsResponse {
    pub data: Vec<DevflowWatchdogAlert>,
    #[ts(type = "string | null")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowTaskStatus {
    Planned,
    Running,
    Paused,
    ReadyForReview,
    ReadyToMerge,
    Failed,
    Blocked,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTask {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub objective: String,
    #[ts(type = "string | null")]
    pub trigger_source: Option<String>,
    pub status: DevflowTaskStatus,
    pub kind: DevflowTaskKind,
    pub risk_level: DevflowTaskRiskLevel,
    pub dependencies: Vec<String>,
    pub assigned_agent_id: Option<String>,
    pub worktree_id: Option<String>,
    pub context_pack_id: Option<String>,
    pub run_ids: Vec<String>,
    pub artifact_ids: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskCreateParams {
    pub project_root: String,
    pub title: String,
    pub objective: String,
    pub kind: DevflowTaskKind,
    pub risk_level: DevflowTaskRiskLevel,
    #[ts(optional = nullable)]
    pub trigger_source: Option<String>,
    #[ts(optional = nullable)]
    pub dependencies: Option<Vec<String>>,
    #[ts(optional = nullable)]
    pub assigned_agent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskCreateResponse {
    pub task: DevflowTask,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskPlanParams {
    pub project_root: String,
    pub title: String,
    pub objective: String,
    pub risk_level: DevflowTaskRiskLevel,
    #[ts(optional = nullable)]
    pub max_tasks: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskPlanResponse {
    pub data: Vec<DevflowTask>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskDispatchParams {
    #[ts(optional = nullable)]
    pub project_id: Option<String>,
    #[ts(optional = nullable)]
    pub task_ids: Option<Vec<String>>,
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskDispatchSkipped {
    pub task_id: String,
    pub title: String,
    pub status: DevflowTaskStatus,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskDispatchBlocked {
    pub task_id: String,
    pub title: String,
    pub dependencies: Vec<String>,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskDispatchStarted {
    pub task: DevflowTask,
    pub run: DevflowRun,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskReadResponse {
    pub task: DevflowTask,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskListParams {
    #[ts(optional = nullable)]
    pub project_id: Option<String>,
    #[ts(optional = nullable)]
    pub status: Option<DevflowTaskStatus>,
    #[ts(optional = nullable)]
    pub assigned_agent_id: Option<String>,
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskListResponse {
    pub data: Vec<DevflowTask>,
    #[ts(type = "string | null")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskAssignParams {
    pub id: String,
    #[ts(optional = nullable)]
    pub assigned_agent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskAssignResponse {
    pub task: DevflowTask,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskDependenciesUpdateParams {
    pub id: String,
    pub dependencies: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskDependenciesUpdateResponse {
    pub task: DevflowTask,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowWorktreeStatus {
    Active,
    Dirty,
    Cleaned,
    Missing,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktree {
    pub id: String,
    pub task_id: String,
    pub project_id: String,
    pub repo_root: String,
    pub root_path: String,
    pub cwd_path: String,
    pub branch: String,
    #[ts(type = "string | null")]
    pub base_branch: Option<String>,
    pub base_commit: String,
    #[ts(type = "string | null")]
    pub head_commit: Option<String>,
    pub managed: bool,
    pub status: DevflowWorktreeStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeCreateParams {
    pub task_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeCreateResponse {
    pub worktree: DevflowWorktree,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeListParams {
    #[ts(optional = nullable)]
    pub project_id: Option<String>,
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
    #[ts(optional = nullable)]
    pub status: Option<DevflowWorktreeStatus>,
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeListResponse {
    pub data: Vec<DevflowWorktree>,
    #[ts(type = "string | null")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeReadResponse {
    pub worktree: DevflowWorktree,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeDiffParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeDiffResponse {
    pub worktree: DevflowWorktree,
    pub diff: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeMergeParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeMergeResponse {
    pub merged: bool,
    pub worktree: DevflowWorktree,
    pub task: DevflowTask,
    pub conflicts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeCleanupParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeCleanupResponse {
    pub cleaned: bool,
    pub worktree: DevflowWorktree,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowRunStatus {
    Queued,
    Running,
    Cancelled,
    ReadyForReview,
    ReadyToMerge,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowRun {
    pub id: String,
    pub task_id: String,
    pub agent_id: String,
    #[ts(type = "string | null")]
    pub thread_id: Option<String>,
    #[ts(type = "string | null")]
    pub turn_id: Option<String>,
    pub status: DevflowRunStatus,
    pub started_at: i64,
    #[ts(type = "bigint | null")]
    pub completed_at: Option<i64>,
    pub input: String,
    #[ts(type = "string | null")]
    pub stream_summary: Option<String>,
    pub command_ids: Vec<String>,
    pub artifact_ids: Vec<String>,
    #[ts(type = "string | null")]
    pub exit_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowArtifactKind {
    ContextPack,
    Diff,
    DeliveryReceipt,
    OutputArchive,
    QualityGateOutput,
    Report,
    ReviewReport,
    RunSummary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifact {
    pub id: String,
    pub task_id: String,
    pub run_id: String,
    pub kind: DevflowArtifactKind,
    pub title: String,
    pub path: String,
    pub mime_type: String,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactListParams {
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
    #[ts(optional = nullable)]
    pub run_id: Option<String>,
    #[ts(optional = nullable)]
    pub kind: Option<DevflowArtifactKind>,
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactListResponse {
    pub data: Vec<DevflowArtifact>,
    #[ts(type = "string | null")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactReadResponse {
    pub artifact: DevflowArtifact,
    pub contents: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactOpenParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactOpenResponse {
    pub artifact: DevflowArtifact,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactExportParams {
    pub id: String,
    pub destination_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactExportResponse {
    pub artifact: DevflowArtifact,
    pub destination_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowArtifactDeliveryStatus {
    PendingApproval,
    Delivered,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactDeliverParams {
    pub id: String,
    pub target_agent_id: String,
    pub destination: String,
    #[ts(optional = nullable)]
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactDeliverResponse {
    pub artifact: DevflowArtifact,
    #[ts(type = "DevflowArtifact | null")]
    pub receipt_artifact: Option<DevflowArtifact>,
    #[ts(type = "DevflowApproval | null")]
    pub approval: Option<DevflowApproval>,
    pub target_agent_id: String,
    pub destination: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub status: DevflowArtifactDeliveryStatus,
    pub output_summary: String,
    #[ts(type = "bigint | null")]
    pub delivered_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskStartParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskStartResponse {
    pub task: DevflowTask,
    pub run: DevflowRun,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskDispatchResponse {
    pub started: Vec<DevflowTaskDispatchStarted>,
    pub skipped: Vec<DevflowTaskDispatchSkipped>,
    pub blocked: Vec<DevflowTaskDispatchBlocked>,
    #[ts(type = "DevflowArtifact | null")]
    pub integrator_artifact: Option<DevflowArtifact>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskPauseParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskPauseResponse {
    pub task: DevflowTask,
    pub run: DevflowRun,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskResumeParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskResumeResponse {
    pub task: DevflowTask,
    pub run: DevflowRun,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskCancelParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskCancelResponse {
    pub task: DevflowTask,
    #[ts(type = "DevflowRun | null")]
    pub run: Option<DevflowRun>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowQualityGateKind {
    Format,
    Lint,
    Typecheck,
    TargetedTest,
    IntegrationTest,
    Snapshot,
    Build,
    Review,
    GstackHealth,
    GstackBrowserQa,
    GstackBenchmark,
    GstackCanary,
    GstackWatchdog,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowQualityGateStatus {
    Queued,
    Running,
    Passed,
    Failed,
    Waived,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGate {
    pub id: String,
    pub task_id: String,
    pub run_id: String,
    pub kind: DevflowQualityGateKind,
    pub status: DevflowQualityGateStatus,
    pub command: String,
    pub cwd: String,
    #[ts(type = "number | null")]
    pub exit_code: Option<i32>,
    #[ts(type = "bigint | null")]
    pub duration_ms: Option<i64>,
    #[ts(type = "string | null")]
    pub summary: Option<String>,
    #[ts(type = "string | null")]
    pub artifact_id: Option<String>,
    #[ts(type = "string | null")]
    pub waived_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateListParams {
    #[ts(optional = nullable)]
    pub task_id: Option<String>,
    #[ts(optional = nullable)]
    pub run_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateListResponse {
    pub data: Vec<DevflowQualityGate>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateReadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateReadResponse {
    pub gate: DevflowQualityGate,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateRunParams {
    pub task_id: String,
    #[ts(optional = nullable)]
    pub kind: Option<DevflowQualityGateKind>,
    #[ts(optional = nullable)]
    pub command_override: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateRunResponse {
    pub gate: DevflowQualityGate,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateRerunParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateRerunResponse {
    pub gate: DevflowQualityGate,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateWaiveParams {
    pub id: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateWaiveResponse {
    pub gate: DevflowQualityGate,
    #[ts(type = "DevflowApproval | null")]
    pub approval: Option<DevflowApproval>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowRunStatusChangedNotification {
    pub project_id: String,
    pub task_id: String,
    pub run: DevflowRun,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum DevflowRunOutputSource {
    Assistant,
    CommandExecution,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowRunOutputDeltaNotification {
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    pub source: DevflowRunOutputSource,
    pub delta: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowRunCommandStartedNotification {
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    pub command_id: String,
    pub command: String,
    pub cwd: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowRunCommandCompletedNotification {
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    pub command_id: String,
    pub exit_code: Option<i32>,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub output_summary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowRunDiffUpdatedNotification {
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    pub artifact_id: String,
    pub diff: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeDiffUpdatedNotification {
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    #[ts(type = "string | null")]
    pub worktree_id: Option<String>,
    pub artifact_id: String,
    pub diff: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowArtifactCreatedNotification {
    pub project_id: String,
    pub artifact: DevflowArtifact,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowQualityGateCompletedNotification {
    pub project_id: String,
    pub gate: DevflowQualityGate,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWatchdogAlertCreatedNotification {
    pub project_id: String,
    pub alert: DevflowWatchdogAlert,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowWorktreeStatusChangedNotification {
    pub worktree: DevflowWorktree,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowTaskStatusChangedNotification {
    pub task: DevflowTask,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowAgentStatusChangedNotification {
    pub agent: DevflowAgent,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DevflowApprovalRequestedNotification {
    pub approval: DevflowApproval,
}
