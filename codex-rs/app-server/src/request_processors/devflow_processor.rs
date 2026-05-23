use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use chrono::Utc;
use codex_analytics::AnalyticsEventsClient;
use codex_app_server_protocol::CommandExecutionApprovalDecision;
use codex_app_server_protocol::CommandExecutionRequestApprovalParams;
use codex_app_server_protocol::CommandExecutionRequestApprovalResponse;
use codex_app_server_protocol::DevflowAgent;
use codex_app_server_protocol::DevflowAgentCapabilitiesReadParams;
use codex_app_server_protocol::DevflowAgentCapabilitiesReadResponse;
use codex_app_server_protocol::DevflowAgentDetectParams;
use codex_app_server_protocol::DevflowAgentDetectResponse;
use codex_app_server_protocol::DevflowAgentDiagnoseParams;
use codex_app_server_protocol::DevflowAgentDiagnoseResponse;
use codex_app_server_protocol::DevflowAgentLane;
use codex_app_server_protocol::DevflowAgentListParams;
use codex_app_server_protocol::DevflowAgentListResponse;
use codex_app_server_protocol::DevflowAgentReadParams;
use codex_app_server_protocol::DevflowAgentReadResponse;
use codex_app_server_protocol::DevflowAgentRestartParams;
use codex_app_server_protocol::DevflowAgentRestartResponse;
use codex_app_server_protocol::DevflowAgentRuntime;
use codex_app_server_protocol::DevflowAgentStartParams;
use codex_app_server_protocol::DevflowAgentStartResponse;
use codex_app_server_protocol::DevflowAgentStatus;
use codex_app_server_protocol::DevflowAgentStatusChangedNotification;
use codex_app_server_protocol::DevflowAgentStopParams;
use codex_app_server_protocol::DevflowAgentStopResponse;
use codex_app_server_protocol::DevflowApproval;
use codex_app_server_protocol::DevflowApprovalDecision;
use codex_app_server_protocol::DevflowApprovalKind;
use codex_app_server_protocol::DevflowApprovalListParams;
use codex_app_server_protocol::DevflowApprovalListResponse;
use codex_app_server_protocol::DevflowApprovalPolicyReadParams;
use codex_app_server_protocol::DevflowApprovalPolicyReadResponse;
use codex_app_server_protocol::DevflowApprovalPolicyUpdateParams;
use codex_app_server_protocol::DevflowApprovalPolicyUpdateResponse;
use codex_app_server_protocol::DevflowApprovalRequestedNotification;
use codex_app_server_protocol::DevflowApprovalRespondParams;
use codex_app_server_protocol::DevflowApprovalRespondResponse;
use codex_app_server_protocol::DevflowApprovalStatus;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowArtifactCreatedNotification;
use codex_app_server_protocol::DevflowArtifactDeliverParams;
use codex_app_server_protocol::DevflowArtifactDeliverResponse;
use codex_app_server_protocol::DevflowArtifactDeliveryStatus;
use codex_app_server_protocol::DevflowArtifactExportParams;
use codex_app_server_protocol::DevflowArtifactExportResponse;
use codex_app_server_protocol::DevflowArtifactKind;
use codex_app_server_protocol::DevflowArtifactListParams;
use codex_app_server_protocol::DevflowArtifactListResponse;
use codex_app_server_protocol::DevflowArtifactOpenParams;
use codex_app_server_protocol::DevflowArtifactOpenResponse;
use codex_app_server_protocol::DevflowArtifactReadParams;
use codex_app_server_protocol::DevflowArtifactReadResponse;
use codex_app_server_protocol::DevflowPolicyPack;
use codex_app_server_protocol::DevflowProjectDiagnoseParams;
use codex_app_server_protocol::DevflowProjectDiagnoseResponse;
use codex_app_server_protocol::DevflowProjectListParams;
use codex_app_server_protocol::DevflowProjectListResponse;
use codex_app_server_protocol::DevflowProjectMemoryReadParams;
use codex_app_server_protocol::DevflowProjectMemoryReadResponse;
use codex_app_server_protocol::DevflowProjectMemoryWriteParams;
use codex_app_server_protocol::DevflowProjectMemoryWriteResponse;
use codex_app_server_protocol::DevflowProjectOpenParams;
use codex_app_server_protocol::DevflowProjectOpenResponse;
use codex_app_server_protocol::DevflowProjectReadParams;
use codex_app_server_protocol::DevflowProjectReadResponse;
use codex_app_server_protocol::DevflowProjectTestCommandsListParams;
use codex_app_server_protocol::DevflowProjectTestCommandsListResponse;
use codex_app_server_protocol::DevflowProjectTrustParams;
use codex_app_server_protocol::DevflowProjectTrustResponse;
use codex_app_server_protocol::DevflowQualityGate;
use codex_app_server_protocol::DevflowQualityGateCompletedNotification;
use codex_app_server_protocol::DevflowQualityGateKind;
use codex_app_server_protocol::DevflowQualityGateListParams;
use codex_app_server_protocol::DevflowQualityGateListResponse;
use codex_app_server_protocol::DevflowQualityGateReadParams;
use codex_app_server_protocol::DevflowQualityGateReadResponse;
use codex_app_server_protocol::DevflowQualityGateRerunParams;
use codex_app_server_protocol::DevflowQualityGateRerunResponse;
use codex_app_server_protocol::DevflowQualityGateRunParams;
use codex_app_server_protocol::DevflowQualityGateRunResponse;
use codex_app_server_protocol::DevflowQualityGateStatus;
use codex_app_server_protocol::DevflowQualityGateWaiveParams;
use codex_app_server_protocol::DevflowQualityGateWaiveResponse;
use codex_app_server_protocol::DevflowReleasePrepCreateParams;
use codex_app_server_protocol::DevflowReleasePrepCreateResponse;
use codex_app_server_protocol::DevflowRun;
use codex_app_server_protocol::DevflowRunCommandCompletedNotification;
use codex_app_server_protocol::DevflowRunCommandStartedNotification;
use codex_app_server_protocol::DevflowRunDiffUpdatedNotification;
use codex_app_server_protocol::DevflowRunOutputDeltaNotification;
use codex_app_server_protocol::DevflowRunOutputSource;
use codex_app_server_protocol::DevflowRunStatus;
use codex_app_server_protocol::DevflowRunStatusChangedNotification;
use codex_app_server_protocol::DevflowSupportBundle;
use codex_app_server_protocol::DevflowSupportBundleCreateParams;
use codex_app_server_protocol::DevflowSupportBundleCreateResponse;
use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowTaskAssignParams;
use codex_app_server_protocol::DevflowTaskAssignResponse;
use codex_app_server_protocol::DevflowTaskCancelParams;
use codex_app_server_protocol::DevflowTaskCancelResponse;
use codex_app_server_protocol::DevflowTaskCreateParams;
use codex_app_server_protocol::DevflowTaskCreateResponse;
use codex_app_server_protocol::DevflowTaskDependenciesUpdateParams;
use codex_app_server_protocol::DevflowTaskDependenciesUpdateResponse;
use codex_app_server_protocol::DevflowTaskDispatchBlocked;
use codex_app_server_protocol::DevflowTaskDispatchParams;
use codex_app_server_protocol::DevflowTaskDispatchResponse;
use codex_app_server_protocol::DevflowTaskDispatchSkipped;
use codex_app_server_protocol::DevflowTaskDispatchStarted;
use codex_app_server_protocol::DevflowTaskKind;
use codex_app_server_protocol::DevflowTaskListParams;
use codex_app_server_protocol::DevflowTaskListResponse;
use codex_app_server_protocol::DevflowTaskPauseParams;
use codex_app_server_protocol::DevflowTaskPauseResponse;
use codex_app_server_protocol::DevflowTaskPlanParams;
use codex_app_server_protocol::DevflowTaskPlanResponse;
use codex_app_server_protocol::DevflowTaskReadParams;
use codex_app_server_protocol::DevflowTaskReadResponse;
use codex_app_server_protocol::DevflowTaskResumeParams;
use codex_app_server_protocol::DevflowTaskResumeResponse;
use codex_app_server_protocol::DevflowTaskRiskLevel;
use codex_app_server_protocol::DevflowTaskStartParams;
use codex_app_server_protocol::DevflowTaskStartResponse;
use codex_app_server_protocol::DevflowTaskStatus;
use codex_app_server_protocol::DevflowTaskStatusChangedNotification;
use codex_app_server_protocol::DevflowWatchdogAlert;
use codex_app_server_protocol::DevflowWatchdogAlertCreatedNotification;
use codex_app_server_protocol::DevflowWatchdogAlertSeverity;
use codex_app_server_protocol::DevflowWatchdogStatus;
use codex_app_server_protocol::DevflowWorktree;
use codex_app_server_protocol::DevflowWorktreeCleanupParams;
use codex_app_server_protocol::DevflowWorktreeCleanupResponse;
use codex_app_server_protocol::DevflowWorktreeCreateParams;
use codex_app_server_protocol::DevflowWorktreeCreateResponse;
use codex_app_server_protocol::DevflowWorktreeDiffParams;
use codex_app_server_protocol::DevflowWorktreeDiffResponse;
use codex_app_server_protocol::DevflowWorktreeDiffUpdatedNotification;
use codex_app_server_protocol::DevflowWorktreeListParams;
use codex_app_server_protocol::DevflowWorktreeListResponse;
use codex_app_server_protocol::DevflowWorktreeMergeParams;
use codex_app_server_protocol::DevflowWorktreeMergeResponse;
use codex_app_server_protocol::DevflowWorktreeReadParams;
use codex_app_server_protocol::DevflowWorktreeReadResponse;
use codex_app_server_protocol::DevflowWorktreeStatusChangedNotification;
use codex_app_server_protocol::FileChangeApprovalDecision;
use codex_app_server_protocol::FileChangeRequestApprovalParams;
use codex_app_server_protocol::FileChangeRequestApprovalResponse;
use codex_app_server_protocol::GrantedPermissionProfile;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::ItemStartedNotification;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::PermissionGrantScope;
use codex_app_server_protocol::PermissionsRequestApprovalParams;
use codex_app_server_protocol::PermissionsRequestApprovalResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::RequestPermissionProfile;
use codex_app_server_protocol::Result as JsonRpcResultValue;
use codex_app_server_protocol::ReviewOutput;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnDiffUpdatedNotification;
use codex_arg0::Arg0DispatchPaths;
use codex_core::ThreadManager;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_git_utils::get_git_repo_root;
use codex_protocol::ThreadId;
use codex_protocol::config_types::SandboxMode;
use codex_protocol::error::CodexErr;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ReviewRequest;
use codex_protocol::protocol::ReviewTarget;
use codex_protocol::user_input::UserInput as CoreUserInput;
use serde::Serialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::config_manager::ConfigManager;
use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::ObservedOutgoingMessage;
use crate::outgoing_message::OutgoingMessage;
use crate::outgoing_message::OutgoingMessageSender;
use crate::thread_state::ThreadStateManager;
use crate::thread_status::ThreadWatchManager;

use super::devflow_approval::approval_policy_for_risk;
use super::devflow_approval::default_approval_policy;
use super::devflow_approval::load_approval_policy;
use super::devflow_approval::save_approval_policy;
use super::devflow_external_agent::ExternalAgentExecution;
use super::devflow_external_agent::run_claude_report;
use super::devflow_external_agent::run_hermes_command;
use super::devflow_policy_requirements::policy_pack_required_artifacts;
use super::devflow_policy_requirements::required_quality_gates;
use super::devflow_project::diagnose_project;
use super::devflow_quality_gate::GateCommand;
use super::devflow_quality_gate::combine_gate_output;
use super::devflow_quality_gate::quality_gate_command;
use super::devflow_quality_gate::run_gate_command;
use super::devflow_release_prep::DevflowReleasePrepDraft;
use super::devflow_release_prep::DevflowReleasePrepInput;
use super::devflow_release_prep::create_devflow_release_prep;
use super::devflow_review_findings::build_review_finding_state;
use super::devflow_review_findings::render_review_artifact;
use super::devflow_review_findings::review_artifact_all_findings_addressed;
use super::devflow_review_findings::review_artifact_summary;
use super::devflow_root_cause::build_root_cause_state;
use super::devflow_root_cause::render_root_cause_artifact;
use super::devflow_root_cause::root_cause_artifact_summary;
use super::devflow_root_cause::task_requires_root_cause;
use super::devflow_store_persistence::DevflowStoreSnapshot;
use super::devflow_store_persistence::PersistedDevflowQualityGateRecord;
use super::devflow_store_persistence::PersistedDevflowRequestedStop;
use super::devflow_store_persistence::PersistedDevflowRunRecord;
use super::devflow_store_persistence::devflow_store_snapshot_path;
use super::devflow_store_persistence::load_devflow_store_snapshot;
use super::devflow_store_persistence::save_devflow_store_snapshot;
use super::devflow_support_bundle::DevflowSupportBundleInput;
use super::devflow_support_bundle::create_devflow_support_bundle;
use super::devflow_worktree::WorktreeMergeOutcome;
use super::devflow_worktree::cleanup_managed_worktree;
use super::devflow_worktree::create_managed_worktree;
use super::devflow_worktree::list_managed_worktrees;
use super::devflow_worktree::merge_managed_worktree;
use super::devflow_worktree::read_managed_worktree;
use super::devflow_worktree::worktree_diff;
use super::thread_lifecycle::ListenerTaskContext;
use super::thread_lifecycle::ensure_conversation_listener;

const DEFAULT_CLAUDE_ROOT: &str = "/Users/yuqei/claude-code";
const DEFAULT_HERMES_ROOT: &str = "/Users/yuqei/hermes-agent";
const INTERNAL_CONNECTION_ID_START: u64 = 1_000_000_000;
const STREAM_SUMMARY_LIMIT: usize = 4000;
const COMMAND_OUTPUT_SUMMARY_LIMIT: usize = 400;
const DIFF_SUMMARY_LIMIT: usize = 160;
const OUTPUT_ARCHIVE_THRESHOLD: usize = STREAM_SUMMARY_LIMIT;
const DEVFLOW_DISPATCH_DEFAULT_LIMIT: usize = 4;
const DEVFLOW_DISPATCH_MAX_LIMIT: usize = 16;
const STORE_SNAPSHOT_PERSIST_ALERT_ID: &str = "devflow-store-snapshot-persist-error";
const ARTIFACT_DELIVERY_COMMAND: &str =
    "hermes chat -Q --source devflow --max-turns 5 -q <devflow-artifact-delivery-prompt>";

struct StaticAgentDescriptor<'a> {
    id: &'a str,
    name: &'a str,
    runtime: DevflowAgentRuntime,
    lane: DevflowAgentLane,
    root: &'a str,
    launch_command: &'a str,
    roles: &'a [&'a str],
    capabilities: &'a [&'a str],
}

#[derive(Clone)]
struct DevflowRunRecord {
    run: DevflowRun,
    project_root: String,
    internal_connection_id: ConnectionId,
    diff_artifact_id: Option<String>,
    summary_artifact_id: Option<String>,
    output_archive_artifact_id: Option<String>,
    review_artifact_id: Option<String>,
    quality_gate_id: Option<String>,
    review_requested: bool,
    review_completed: bool,
    auto_repair_attempt: u32,
    auto_integrator_merge: bool,
    requested_stop: Option<DevflowRequestedStop>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DevflowRequestedStop {
    Pause,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DevflowIntegratorMergePolicy {
    Manual,
    AutoWhenReady,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DevflowQualityGateStartMode {
    Manual,
    Automatic,
}

struct DevflowQualityGateStartRequest {
    task_id: String,
    kind: DevflowQualityGateKind,
    command_override: Option<String>,
    mode: DevflowQualityGateStartMode,
}

impl From<PersistedDevflowRequestedStop> for DevflowRequestedStop {
    fn from(value: PersistedDevflowRequestedStop) -> Self {
        match value {
            PersistedDevflowRequestedStop::Pause => Self::Pause,
            PersistedDevflowRequestedStop::Cancel => Self::Cancel,
        }
    }
}

impl From<DevflowRequestedStop> for PersistedDevflowRequestedStop {
    fn from(value: DevflowRequestedStop) -> Self {
        match value {
            DevflowRequestedStop::Pause => Self::Pause,
            DevflowRequestedStop::Cancel => Self::Cancel,
        }
    }
}

#[derive(Clone)]
struct DevflowQualityGateRecord {
    gate: DevflowQualityGate,
    command: GateCommand,
}

#[derive(Clone)]
enum PendingDevflowApprovalRequest {
    CommandExecution {
        request_id: RequestId,
        params: CommandExecutionRequestApprovalParams,
    },
    FileChange {
        request_id: RequestId,
        params: FileChangeRequestApprovalParams,
    },
    Permissions {
        request_id: RequestId,
        params: PermissionsRequestApprovalParams,
    },
    QualityGateWaive {
        gate_id: String,
        waive_reason: String,
    },
    ArtifactDelivery {
        artifact_id: String,
        target_agent_id: String,
        destination: String,
        message: Option<String>,
    },
}

#[derive(Clone)]
struct DevflowApprovalRecord {
    approval: DevflowApproval,
    request: PendingDevflowApprovalRequest,
}

#[derive(Clone, PartialEq)]
struct DevflowApprovalGrant {
    project_id: String,
    task_id: Option<String>,
    kind: DevflowApprovalKind,
    command: Option<String>,
    cwd: Option<String>,
    file_paths: Vec<String>,
    requested_permissions: Option<RequestPermissionProfile>,
    decision: DevflowApprovalDecision,
}

struct DevflowApprovalProjection {
    request_id: RequestId,
    thread_id: String,
    kind: DevflowApprovalKind,
    reason: Option<String>,
    command: Option<String>,
    cwd: Option<String>,
    file_paths: Vec<String>,
    requested_permissions: Option<RequestPermissionProfile>,
    request: PendingDevflowApprovalRequest,
}

struct DevflowRunCommandCompletedEvent {
    task_id: String,
    run_id: String,
    command_id: String,
    exit_code: Option<i32>,
    status: String,
    duration_ms: Option<i64>,
    output_summary: Option<String>,
}

#[derive(Clone)]
pub(super) struct DevflowCapabilityPackTarget {
    pub(super) task_id: String,
    pub(super) run_id: String,
    pub(super) project_root: String,
    pub(super) cwd_path: PathBuf,
    pub(super) worktree_id: Option<String>,
}

pub(super) struct DevflowCapabilityPackGateOutcome {
    pub(super) kind: DevflowQualityGateKind,
    pub(super) capability: &'static str,
    pub(super) status: DevflowQualityGateStatus,
    pub(super) command: String,
    pub(super) exit_code: Option<i32>,
    pub(super) duration_ms: Option<i64>,
    pub(super) summary: String,
}

pub(super) struct DevflowPolicyPackApplication {
    pub(super) required_artifacts: Vec<String>,
    pub(super) diagnostics: Vec<String>,
    pub(super) artifact: Option<DevflowArtifact>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DevflowWatchdogQueueSnapshot {
    pub(super) status: DevflowWatchdogStatus,
    pub(super) counts: DevflowWatchdogQueueCounts,
    pub(super) running: Vec<DevflowWatchdogQueueItem>,
    pub(super) no_progress: Vec<DevflowWatchdogQueueItem>,
    pub(super) timed_out: Vec<DevflowWatchdogQueueItem>,
    pub(super) recovering: Vec<DevflowWatchdogQueueItem>,
    pub(super) blocked: Vec<DevflowWatchdogQueueItem>,
    pub(super) alerts: Vec<DevflowWatchdogAlert>,
    pub(super) checked_at: i64,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DevflowWatchdogQueueCounts {
    pub(super) running: usize,
    pub(super) no_progress: usize,
    pub(super) timed_out: usize,
    pub(super) recovering: usize,
    pub(super) blocked: usize,
    pub(super) alerts: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DevflowWatchdogQueueItem {
    pub(super) task_id: Option<String>,
    pub(super) run_id: Option<String>,
    pub(super) project_id: Option<String>,
    pub(super) title: Option<String>,
    pub(super) task_status: Option<DevflowTaskStatus>,
    pub(super) run_status: Option<DevflowRunStatus>,
    pub(super) agent_id: Option<String>,
    pub(super) updated_at: Option<i64>,
    pub(super) alert_id: Option<String>,
    pub(super) alert_severity: Option<DevflowWatchdogAlertSeverity>,
    pub(super) reason: String,
}

#[derive(Default)]
struct DevflowStore {
    tasks: HashMap<String, DevflowTask>,
    runs: HashMap<String, DevflowRunRecord>,
    quality_gates: HashMap<String, DevflowQualityGateRecord>,
    approvals: HashMap<String, DevflowApprovalRecord>,
    approval_history: HashMap<String, DevflowApproval>,
    approval_grants: Vec<DevflowApprovalGrant>,
    artifacts: HashMap<String, DevflowArtifact>,
    watchdog_alerts: Vec<DevflowWatchdogAlert>,
    thread_to_run: HashMap<String, String>,
}

fn devflow_store_from_snapshot(snapshot: DevflowStoreSnapshot) -> DevflowStore {
    let mut store = DevflowStore::default();
    let now = Utc::now().timestamp();
    store.tasks = snapshot
        .tasks
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect();
    let mut interrupted_task_ids = HashSet::new();
    store.runs = snapshot
        .runs
        .into_iter()
        .map(|mut record| {
            if matches!(
                record.run.status,
                DevflowRunStatus::Queued | DevflowRunStatus::Running
            ) {
                record.run.status = DevflowRunStatus::Failed;
                record.run.completed_at.get_or_insert(now);
                record.run.exit_reason.get_or_insert_with(|| {
                    "app-server restarted before this Devflow run completed; start a new run to recover"
                        .to_string()
                });
                interrupted_task_ids.insert(record.run.task_id.clone());
            }
            (
                record.run.id.clone(),
                DevflowRunRecord {
                    run: record.run,
                    project_root: record.project_root,
                    internal_connection_id: ConnectionId(0),
                    diff_artifact_id: record.diff_artifact_id,
                    summary_artifact_id: record.summary_artifact_id,
                    output_archive_artifact_id: record.output_archive_artifact_id,
                    review_artifact_id: record.review_artifact_id,
                    quality_gate_id: record.quality_gate_id,
                    review_requested: record.review_requested,
                    review_completed: record.review_completed,
                    auto_repair_attempt: record.auto_repair_attempt,
                    auto_integrator_merge: record.auto_integrator_merge,
                    requested_stop: record.requested_stop.map(DevflowRequestedStop::from),
                },
            )
        })
        .collect();
    for task_id in interrupted_task_ids {
        if let Some(task) = store.tasks.get_mut(&task_id)
            && task.status == DevflowTaskStatus::Running
        {
            task.status = DevflowTaskStatus::Blocked;
            task.updated_at = now;
        }
    }
    store.quality_gates = snapshot
        .quality_gates
        .into_iter()
        .map(|mut record| {
            if matches!(
                record.gate.status,
                DevflowQualityGateStatus::Queued | DevflowQualityGateStatus::Running
            ) {
                record.gate.status = DevflowQualityGateStatus::Failed;
                record.gate.updated_at = now;
                record.gate.summary.get_or_insert_with(|| {
                    "app-server restarted before this quality gate completed; rerun required"
                        .to_string()
                });
            }
            (
                record.gate.id.clone(),
                DevflowQualityGateRecord {
                    gate: record.gate,
                    command: record.command,
                },
            )
        })
        .collect();
    store.approval_history = snapshot
        .approvals
        .into_iter()
        .map(|mut approval| {
            if approval.status == DevflowApprovalStatus::Pending {
                approval.status = DevflowApprovalStatus::Responded;
                approval.responded_at = Some(now);
                approval.decision = Some(DevflowApprovalDecision::Cancel);
                approval.reason = Some(
                    approval
                        .reason
                        .map(|reason| {
                            format!(
                                "{reason}\n\nCancelled because app-server restarted before this Devflow approval was answered."
                            )
                        })
                        .unwrap_or_else(|| {
                            "Cancelled because app-server restarted before this Devflow approval was answered."
                                .to_string()
                        }),
                );
            }
            (approval.id.clone(), approval)
        })
        .collect();
    store.artifacts = snapshot
        .artifacts
        .into_iter()
        .map(|artifact| (artifact.id.clone(), artifact))
        .collect();
    store.watchdog_alerts = snapshot.watchdog_alerts;
    store
}

fn devflow_store_snapshot(store: &DevflowStore) -> DevflowStoreSnapshot {
    DevflowStoreSnapshot::new(
        store.tasks.values().cloned().collect(),
        store
            .runs
            .values()
            .map(|record| PersistedDevflowRunRecord {
                run: record.run.clone(),
                project_root: record.project_root.clone(),
                diff_artifact_id: record.diff_artifact_id.clone(),
                summary_artifact_id: record.summary_artifact_id.clone(),
                output_archive_artifact_id: record.output_archive_artifact_id.clone(),
                review_artifact_id: record.review_artifact_id.clone(),
                quality_gate_id: record.quality_gate_id.clone(),
                review_requested: record.review_requested,
                review_completed: record.review_completed,
                auto_repair_attempt: record.auto_repair_attempt,
                auto_integrator_merge: record.auto_integrator_merge,
                requested_stop: record
                    .requested_stop
                    .map(PersistedDevflowRequestedStop::from),
            })
            .collect(),
        store
            .quality_gates
            .values()
            .map(|record| PersistedDevflowQualityGateRecord {
                gate: record.gate.clone(),
                command: record.command.clone(),
            })
            .collect(),
        devflow_approval_snapshots(store),
        store.artifacts.values().cloned().collect(),
        store.watchdog_alerts.clone(),
    )
}

fn devflow_approval_snapshots(store: &DevflowStore) -> Vec<DevflowApproval> {
    let mut approvals = store.approval_history.clone();
    for record in store.approvals.values() {
        approvals.insert(record.approval.id.clone(), record.approval.clone());
    }
    approvals.into_values().collect()
}

#[derive(Clone)]
pub(crate) struct DevflowRequestProcessor {
    outgoing: Arc<OutgoingMessageSender>,
    arg0_paths: Arg0DispatchPaths,
    config: Arc<Config>,
    config_manager: ConfigManager,
    thread_manager: Arc<ThreadManager>,
    thread_state_manager: ThreadStateManager,
    pending_thread_unloads: Arc<Mutex<HashSet<ThreadId>>>,
    analytics_events_client: AnalyticsEventsClient,
    thread_watch_manager: ThreadWatchManager,
    thread_list_state_permit: Arc<Semaphore>,
    store: Arc<Mutex<DevflowStore>>,
    store_snapshot_load_error: Option<String>,
    store_snapshot_persist_error: Arc<Mutex<Option<String>>>,
    next_internal_connection_id: Arc<AtomicU64>,
}

impl DevflowRequestProcessor {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        outgoing: Arc<OutgoingMessageSender>,
        arg0_paths: Arg0DispatchPaths,
        config: Arc<Config>,
        config_manager: ConfigManager,
        thread_manager: Arc<ThreadManager>,
        thread_state_manager: ThreadStateManager,
        pending_thread_unloads: Arc<Mutex<HashSet<ThreadId>>>,
        analytics_events_client: AnalyticsEventsClient,
        thread_watch_manager: ThreadWatchManager,
        thread_list_state_permit: Arc<Semaphore>,
    ) -> Self {
        let mut store_snapshot_load_error = None;
        let store = match load_devflow_store_snapshot(config.codex_home.as_path()) {
            Ok(Some(snapshot)) => {
                let store = devflow_store_from_snapshot(snapshot);
                tracing::info!(
                    tasks = store.tasks.len(),
                    runs = store.runs.len(),
                    quality_gates = store.quality_gates.len(),
                    artifacts = store.artifacts.len(),
                    watchdog_alerts = store.watchdog_alerts.len(),
                    "loaded devflow store snapshot"
                );
                store
            }
            Ok(None) => DevflowStore::default(),
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "failed to load devflow store snapshot; starting with empty store"
                );
                store_snapshot_load_error = Some(err);
                let mut store = DevflowStore::default();
                store.watchdog_alerts.push(DevflowWatchdogAlert {
                    id: Uuid::new_v4().to_string(),
                    status: DevflowWatchdogStatus::Recovering,
                    severity: DevflowWatchdogAlertSeverity::Critical,
                    project_id: None,
                    task_id: None,
                    run_id: None,
                    message:
                        "Devflow store snapshot could not be restored; runtime indexes started empty"
                            .to_string(),
                    created_at: Utc::now().timestamp(),
                });
                store
            }
        };
        let processor = Self {
            outgoing,
            arg0_paths,
            config,
            config_manager,
            thread_manager,
            thread_state_manager,
            pending_thread_unloads,
            analytics_events_client,
            thread_watch_manager,
            thread_list_state_permit,
            store: Arc::new(Mutex::new(store)),
            store_snapshot_load_error,
            store_snapshot_persist_error: Arc::new(Mutex::new(None)),
            next_internal_connection_id: Arc::new(AtomicU64::new(INTERNAL_CONNECTION_ID_START)),
        };

        let receiver = processor.outgoing.subscribe_outgoing_messages();
        tokio::spawn(Self::observe_outgoing_messages(processor.clone(), receiver));

        processor
    }

    pub(crate) async fn agent_detect(
        &self,
        params: DevflowAgentDetectParams,
    ) -> Result<DevflowAgentDetectResponse, JSONRPCErrorError> {
        let agents = self.detect_agents(params);
        for agent in agents.iter().cloned() {
            self.send_agent_status_changed(agent).await;
        }
        Ok(DevflowAgentDetectResponse { agents })
    }

    pub(crate) async fn agent_list(
        &self,
        params: DevflowAgentListParams,
    ) -> Result<DevflowAgentListResponse, JSONRPCErrorError> {
        let mut agents = self.detect_agents(DevflowAgentDetectParams::default());
        if let Some(runtimes) = params.runtimes {
            agents.retain(|agent| runtimes.contains(&agent.runtime));
        }
        Ok(DevflowAgentListResponse { data: agents })
    }

    pub(crate) async fn agent_read(
        &self,
        params: DevflowAgentReadParams,
    ) -> Result<DevflowAgentReadResponse, JSONRPCErrorError> {
        Ok(DevflowAgentReadResponse {
            agent: self.agent_by_id(&params.id)?,
        })
    }

    pub(crate) async fn agent_capabilities_read(
        &self,
        params: DevflowAgentCapabilitiesReadParams,
    ) -> Result<DevflowAgentCapabilitiesReadResponse, JSONRPCErrorError> {
        let agent = self.agent_by_id(&params.id)?;
        Ok(DevflowAgentCapabilitiesReadResponse {
            id: agent.id,
            capabilities: agent.capabilities,
        })
    }

    pub(crate) async fn agent_diagnose(
        &self,
        params: DevflowAgentDiagnoseParams,
    ) -> Result<DevflowAgentDiagnoseResponse, JSONRPCErrorError> {
        let agent = self.agent_by_id(&params.id)?;
        let cwd = params
            .cwd
            .map(PathBuf::from)
            .or_else(|| agent.root_path.as_ref().map(PathBuf::from))
            .unwrap_or_else(|| self.config.codex_home.to_path_buf());
        let (command, execution) = match params.id.as_str() {
            "hermes-automation" => {
                let args = ["doctor"];
                let execution = run_hermes_command(&cwd, &args)
                    .await
                    .map_err(internal_error)?;
                ("hermes doctor".to_string(), execution)
            }
            "claude-writer" | "claude-reviewer" => {
                let execution = run_claude_report(
                    &cwd,
                    "Reply with a short diagnostic confirming the Claude Code CLI is available.",
                )
                .await
                .map_err(internal_error)?;
                (
                    "claude -p --output-format text --permission-mode plan --tools \"\""
                        .to_string(),
                    execution,
                )
            }
            "codex-main" => (
                "codex runtime self-check".to_string(),
                ExternalAgentExecution {
                    exit_code: Some(0),
                    stdout: "codex runtime ready".to_string(),
                    stderr: String::new(),
                },
            ),
            _ => {
                return Err(invalid_request(format!(
                    "unknown devflow agent id: {}",
                    params.id
                )));
            }
        };

        Ok(DevflowAgentDiagnoseResponse {
            agent,
            command,
            ok: execution.exit_code == Some(0),
            exit_code: execution.exit_code,
            stdout: execution.stdout,
            stderr: execution.stderr,
        })
    }

    pub(crate) async fn agent_start(
        &self,
        params: DevflowAgentStartParams,
    ) -> Result<DevflowAgentStartResponse, JSONRPCErrorError> {
        let agent = self.agent_by_id(&params.id)?;
        Ok(DevflowAgentStartResponse {
            agent,
            started: false,
            message: devflow_agent_lifecycle_noop_message("start"),
        })
    }

    pub(crate) async fn agent_stop(
        &self,
        params: DevflowAgentStopParams,
    ) -> Result<DevflowAgentStopResponse, JSONRPCErrorError> {
        let agent = self.agent_by_id(&params.id)?;
        Ok(DevflowAgentStopResponse {
            agent,
            stopped: false,
            message: devflow_agent_lifecycle_noop_message("stop"),
        })
    }

    pub(crate) async fn agent_restart(
        &self,
        params: DevflowAgentRestartParams,
    ) -> Result<DevflowAgentRestartResponse, JSONRPCErrorError> {
        let agent = self.agent_by_id(&params.id)?;
        Ok(DevflowAgentRestartResponse {
            agent,
            restarted: false,
            message: devflow_agent_lifecycle_noop_message("restart"),
        })
    }

    pub(crate) async fn project_memory_read(
        &self,
        params: DevflowProjectMemoryReadParams,
    ) -> Result<DevflowProjectMemoryReadResponse, JSONRPCErrorError> {
        if params.project_root.trim().is_empty() {
            return Err(invalid_request("project_root is required".to_string()));
        }
        let path = project_memory_path(&params.project_root);
        let summary = read_project_memory_summary(&path).await?;
        Ok(DevflowProjectMemoryReadResponse {
            project_id: params.project_root,
            path: path.display().to_string(),
            summary,
        })
    }

    pub(crate) async fn project_list(
        &self,
        params: DevflowProjectListParams,
    ) -> Result<DevflowProjectListResponse, JSONRPCErrorError> {
        let roots = params.project_roots.unwrap_or_default();
        let mut data = Vec::with_capacity(roots.len());
        for project_root in roots {
            data.push(diagnose_project(&self.config_manager, &project_root).await?);
        }
        Ok(DevflowProjectListResponse { data })
    }

    pub(crate) async fn project_read(
        &self,
        params: DevflowProjectReadParams,
    ) -> Result<DevflowProjectReadResponse, JSONRPCErrorError> {
        Ok(DevflowProjectReadResponse {
            project: diagnose_project(&self.config_manager, &params.project_root).await?,
        })
    }

    pub(crate) async fn project_open(
        &self,
        params: DevflowProjectOpenParams,
    ) -> Result<DevflowProjectOpenResponse, JSONRPCErrorError> {
        Ok(DevflowProjectOpenResponse {
            project: diagnose_project(&self.config_manager, &params.project_root).await?,
        })
    }

    pub(crate) async fn project_memory_write(
        &self,
        params: DevflowProjectMemoryWriteParams,
    ) -> Result<DevflowProjectMemoryWriteResponse, JSONRPCErrorError> {
        if params.project_root.trim().is_empty() {
            return Err(invalid_request("project_root is required".to_string()));
        }
        let path = project_memory_path(&params.project_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|err| {
                internal_error(format!("failed to create project memory dir: {err}"))
            })?;
        }
        fs::write(&path, &params.summary).await.map_err(|err| {
            internal_error(format!("failed to write project memory summary: {err}"))
        })?;
        Ok(DevflowProjectMemoryWriteResponse {
            project_id: params.project_root,
            path: path.display().to_string(),
            summary: params.summary,
        })
    }

    pub(crate) async fn project_diagnose(
        &self,
        params: DevflowProjectDiagnoseParams,
    ) -> Result<DevflowProjectDiagnoseResponse, JSONRPCErrorError> {
        let project = diagnose_project(&self.config_manager, &params.project_root).await?;
        Ok(DevflowProjectDiagnoseResponse { project })
    }

    pub(crate) async fn support_bundle_create(
        &self,
        params: DevflowSupportBundleCreateParams,
    ) -> Result<DevflowSupportBundleCreateResponse, JSONRPCErrorError> {
        if params.project_root.trim().is_empty() {
            return Err(invalid_request("project_root is required".to_string()));
        }

        let project = diagnose_project(&self.config_manager, &params.project_root).await?;
        let project_id = project.id.clone();
        let requested_project_root = params.project_root.clone();
        let task_id_filter = params.task_id.clone();
        let (approval_policy, approval_policy_load_error) =
            match load_approval_policy(self.config.codex_home.as_path()).await {
                Ok(policy) => (policy, None),
                Err(err) => (default_approval_policy(), Some(err)),
            };
        let worktrees = list_managed_worktrees(self.config.codex_home.as_path())
            .await
            .map_err(internal_error)?
            .into_iter()
            .filter(|worktree| {
                (worktree.project_id == project_id || worktree.project_id == requested_project_root)
                    && task_id_filter
                        .as_ref()
                        .is_none_or(|task_id| &worktree.task_id == task_id)
            })
            .collect::<Vec<_>>();
        let store_snapshot_persist_error = self.store_snapshot_persist_error.lock().await.clone();
        let watchdog_queue = serde_json::to_value(
            self.watchdog_queue_snapshot(Some(project_id.as_str()))
                .await,
        )
        .map_err(|err| internal_error(format!("failed to serialize watchdog queue: {err}")))?;

        let (tasks, runs, quality_gates, approvals, artifacts, mut watchdog_alerts) = {
            let store = self.store.lock().await;
            let matches_project =
                |candidate: &str| candidate == project_id || candidate == requested_project_root;
            let tasks = if let Some(task_id) = task_id_filter.as_ref() {
                let task = store.tasks.get(task_id).cloned().ok_or_else(|| {
                    invalid_request(format!("unknown devflow task id: {task_id}"))
                })?;
                if !matches_project(&task.project_id) {
                    return Err(invalid_request(format!(
                        "devflow task {task_id} does not belong to project {}",
                        project.id
                    )));
                }
                vec![task]
            } else {
                store
                    .tasks
                    .values()
                    .filter(|task| matches_project(&task.project_id))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            let task_ids = tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<HashSet<_>>();
            let run_ids_from_tasks = tasks
                .iter()
                .flat_map(|task| task.run_ids.iter().cloned())
                .collect::<HashSet<_>>();
            let runs = store
                .runs
                .values()
                .filter(|record| {
                    run_ids_from_tasks.contains(&record.run.id)
                        || (task_id_filter.is_none() && matches_project(&record.project_root))
                })
                .map(|record| record.run.clone())
                .collect::<Vec<_>>();
            let run_ids = runs
                .iter()
                .map(|run| run.id.clone())
                .collect::<HashSet<_>>();
            let quality_gates = store
                .quality_gates
                .values()
                .filter(|record| {
                    task_ids.contains(&record.gate.task_id) || run_ids.contains(&record.gate.run_id)
                })
                .map(|record| record.gate.clone())
                .collect::<Vec<_>>();
            let approvals = devflow_approval_snapshots(&store)
                .into_iter()
                .filter(|approval| {
                    matches_project(&approval.project_id)
                        && task_id_filter
                            .as_ref()
                            .is_none_or(|task_id| &approval.task_id == task_id)
                })
                .collect::<Vec<_>>();
            let artifacts = store
                .artifacts
                .values()
                .filter(|artifact| {
                    task_ids.contains(&artifact.task_id) || run_ids.contains(&artifact.run_id)
                })
                .cloned()
                .collect::<Vec<_>>();
            let watchdog_alerts = store
                .watchdog_alerts
                .iter()
                .filter(|alert| {
                    alert
                        .project_id
                        .as_ref()
                        .is_some_and(|project_id| matches_project(project_id))
                        || alert
                            .task_id
                            .as_ref()
                            .is_some_and(|task_id| task_ids.contains(task_id))
                        || alert
                            .run_id
                            .as_ref()
                            .is_some_and(|run_id| run_ids.contains(run_id))
                })
                .cloned()
                .collect::<Vec<_>>();

            (
                tasks,
                runs,
                quality_gates,
                approvals,
                artifacts,
                watchdog_alerts,
            )
        };
        if let Some(error) = store_snapshot_persist_error.as_deref() {
            watchdog_alerts.push(store_snapshot_persist_watchdog_alert(
                error,
                Utc::now().timestamp(),
            ));
        }

        let anchor_task_id = task_id_filter
            .clone()
            .or_else(|| tasks.first().map(|task| task.id.clone()));
        let bundle = create_devflow_support_bundle(DevflowSupportBundleInput {
            project: project.clone(),
            task_id: task_id_filter,
            tasks,
            runs,
            quality_gates,
            approvals,
            artifacts,
            worktrees,
            watchdog_alerts,
            watchdog_queue,
            approval_policy,
            approval_policy_load_error,
            store_snapshot_path: devflow_store_snapshot_path(self.config.codex_home.as_path())
                .display()
                .to_string(),
            store_snapshot_load_error: self.store_snapshot_load_error.clone(),
            store_snapshot_persist_error,
        })
        .await
        .map_err(internal_error)?;
        if let Some(artifact) = self
            .record_support_bundle_artifact(anchor_task_id.as_deref(), &bundle)
            .await?
        {
            self.send_artifact_created(artifact).await;
        }

        Ok(DevflowSupportBundleCreateResponse { bundle, project })
    }

    async fn record_support_bundle_artifact(
        &self,
        task_id: Option<&str>,
        bundle: &DevflowSupportBundle,
    ) -> Result<Option<DevflowArtifact>, JSONRPCErrorError> {
        let Some(task_id) = task_id else {
            return Ok(None);
        };
        let artifact = {
            let mut store = self.store.lock().await;
            let Some(task) = store.tasks.get(task_id).cloned() else {
                return Ok(None);
            };
            let run_id = task
                .run_ids
                .last()
                .cloned()
                .unwrap_or_else(|| format!("support-bundle-{}", bundle.id));
            let artifact = DevflowArtifact {
                id: bundle.id.clone(),
                task_id: task.id.clone(),
                run_id: run_id.clone(),
                kind: DevflowArtifactKind::Report,
                title: format!("Support bundle for {}", task.title),
                path: bundle.path.clone(),
                mime_type: bundle.mime_type.clone(),
                summary: bundle.summary.clone(),
                created_at: bundle.created_at,
            };
            if let Some(task) = store.tasks.get_mut(&task.id)
                && !task.artifact_ids.contains(&artifact.id)
            {
                task.artifact_ids.push(artifact.id.clone());
            }
            if let Some(record) = store.runs.get_mut(&run_id)
                && !record.run.artifact_ids.contains(&artifact.id)
            {
                record.run.artifact_ids.push(artifact.id.clone());
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
            artifact
        };
        Ok(Some(artifact))
    }

    pub(crate) async fn release_prep_create(
        &self,
        params: DevflowReleasePrepCreateParams,
    ) -> Result<DevflowReleasePrepCreateResponse, JSONRPCErrorError> {
        if params.project_root.trim().is_empty() {
            return Err(invalid_request("project_root is required".to_string()));
        }

        let project = diagnose_project(&self.config_manager, &params.project_root).await?;
        let project_id = project.id.clone();
        let requested_project_root = params.project_root.clone();
        let task_id_filter = params.task_id.clone();

        let (tasks, runs, quality_gates, prior_artifacts) = {
            let store = self.store.lock().await;
            let matches_project =
                |candidate: &str| candidate == project_id || candidate == requested_project_root;
            let mut tasks = if let Some(task_id) = task_id_filter.as_ref() {
                let task = store.tasks.get(task_id).cloned().ok_or_else(|| {
                    invalid_request(format!("unknown devflow task id: {task_id}"))
                })?;
                if !matches_project(&task.project_id) {
                    return Err(invalid_request(format!(
                        "devflow task {task_id} does not belong to project {}",
                        project.id
                    )));
                }
                vec![task]
            } else {
                store
                    .tasks
                    .values()
                    .filter(|task| matches_project(&task.project_id))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            if tasks.is_empty() {
                return Err(invalid_request(
                    "devflowReleasePrep/create requires at least one task in the project"
                        .to_string(),
                ));
            }
            tasks.sort_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then_with(|| a.id.cmp(&b.id))
            });
            let task_ids = tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<HashSet<_>>();
            let run_ids_from_tasks = tasks
                .iter()
                .flat_map(|task| task.run_ids.iter().cloned())
                .collect::<HashSet<_>>();
            let mut runs = store
                .runs
                .values()
                .filter(|record| {
                    run_ids_from_tasks.contains(&record.run.id)
                        || (task_id_filter.is_none() && matches_project(&record.project_root))
                })
                .map(|record| record.run.clone())
                .collect::<Vec<_>>();
            runs.sort_by(|a, b| {
                a.started_at
                    .cmp(&b.started_at)
                    .then_with(|| a.id.cmp(&b.id))
            });
            let run_ids = runs
                .iter()
                .map(|run| run.id.clone())
                .collect::<HashSet<_>>();
            let mut quality_gates = store
                .quality_gates
                .values()
                .filter(|record| {
                    task_ids.contains(&record.gate.task_id) || run_ids.contains(&record.gate.run_id)
                })
                .map(|record| record.gate.clone())
                .collect::<Vec<_>>();
            quality_gates.sort_by(|a, b| a.id.cmp(&b.id));
            let mut prior_artifacts = store
                .artifacts
                .values()
                .filter(|artifact| {
                    task_ids.contains(&artifact.task_id) || run_ids.contains(&artifact.run_id)
                })
                .cloned()
                .collect::<Vec<_>>();
            prior_artifacts.sort_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then_with(|| a.id.cmp(&b.id))
            });
            (tasks, runs, quality_gates, prior_artifacts)
        };

        let anchor_task = tasks
            .first()
            .cloned()
            .ok_or_else(|| internal_error("release prep task scope unexpectedly empty"))?;
        let run_id = anchor_task
            .run_ids
            .last()
            .cloned()
            .unwrap_or_else(|| format!("release-prep-{}", Uuid::new_v4()));
        let store_snapshot_persist_error = self.store_snapshot_persist_error.lock().await.clone();
        let draft = create_devflow_release_prep(DevflowReleasePrepInput {
            project,
            anchor_task: anchor_task.clone(),
            run_id: run_id.clone(),
            tasks,
            runs,
            quality_gates,
            artifacts: prior_artifacts,
            store_snapshot_load_error: self.store_snapshot_load_error.clone(),
            store_snapshot_persist_error,
        })
        .await
        .map_err(internal_error)?;

        self.record_release_prep_artifacts(&anchor_task.id, &run_id, &draft)
            .await;

        Ok(DevflowReleasePrepCreateResponse {
            status: draft.status,
            summary: draft.summary,
            blockers: draft.blockers,
            commit_message_artifact: draft.commit_message_artifact,
            pr_body_artifact: draft.pr_body_artifact,
            release_note_artifact: draft.release_note_artifact,
        })
    }

    async fn record_release_prep_artifacts(
        &self,
        task_id: &str,
        run_id: &str,
        draft: &DevflowReleasePrepDraft,
    ) {
        let artifacts = vec![
            draft.commit_message_artifact.clone(),
            draft.pr_body_artifact.clone(),
            draft.release_note_artifact.clone(),
        ];
        {
            let mut store = self.store.lock().await;
            if let Some(task) = store.tasks.get_mut(task_id) {
                for artifact in &artifacts {
                    if !task.artifact_ids.contains(&artifact.id) {
                        task.artifact_ids.push(artifact.id.clone());
                    }
                }
                task.updated_at = Utc::now().timestamp();
            }
            if let Some(record) = store.runs.get_mut(run_id) {
                for artifact in &artifacts {
                    if !record.run.artifact_ids.contains(&artifact.id) {
                        record.run.artifact_ids.push(artifact.id.clone());
                    }
                }
            }
            for artifact in &artifacts {
                store
                    .artifacts
                    .insert(artifact.id.clone(), artifact.clone());
            }
        }
        for artifact in artifacts {
            self.send_artifact_created(artifact).await;
        }
    }

    pub(crate) async fn project_test_commands_list(
        &self,
        params: DevflowProjectTestCommandsListParams,
    ) -> Result<DevflowProjectTestCommandsListResponse, JSONRPCErrorError> {
        let project = diagnose_project(&self.config_manager, &params.project_root).await?;
        Ok(DevflowProjectTestCommandsListResponse {
            project_id: project.id,
            commands: project.test_commands,
        })
    }

    pub(crate) async fn project_trust(
        &self,
        params: DevflowProjectTrustParams,
    ) -> Result<DevflowProjectTrustResponse, JSONRPCErrorError> {
        if params.project_root.trim().is_empty() {
            return Err(invalid_request("project_root is required".to_string()));
        }
        let trust_level = if params.trusted {
            codex_protocol::config_types::TrustLevel::Trusted
        } else {
            codex_protocol::config_types::TrustLevel::Untrusted
        };
        codex_core::config::set_project_trust_level(
            &self.config.codex_home,
            Path::new(&params.project_root),
            trust_level,
        )
        .map_err(|err| internal_error(format!("failed to persist project trust: {err}")))?;
        let project = diagnose_project(&self.config_manager, &params.project_root).await?;
        Ok(DevflowProjectTrustResponse { project })
    }

    pub(crate) async fn approval_policy_read(
        &self,
        _params: DevflowApprovalPolicyReadParams,
    ) -> Result<DevflowApprovalPolicyReadResponse, JSONRPCErrorError> {
        let policy = load_approval_policy(self.config.codex_home.as_path())
            .await
            .map_err(|err| {
                invalid_request(format!("failed to load devflow approval policy: {err}"))
            })?;
        Ok(DevflowApprovalPolicyReadResponse { policy })
    }

    pub(crate) async fn approval_policy_update(
        &self,
        params: DevflowApprovalPolicyUpdateParams,
    ) -> Result<DevflowApprovalPolicyUpdateResponse, JSONRPCErrorError> {
        save_approval_policy(self.config.codex_home.as_path(), &params.policy)
            .await
            .map_err(internal_error)?;
        Ok(DevflowApprovalPolicyUpdateResponse {
            policy: params.policy,
        })
    }

    pub(crate) async fn approval_list(
        &self,
        params: DevflowApprovalListParams,
    ) -> Result<DevflowApprovalListResponse, JSONRPCErrorError> {
        let mut data = {
            let store = self.store.lock().await;
            devflow_approval_snapshots(&store)
                .into_iter()
                .filter(|record| {
                    params
                        .task_id
                        .as_ref()
                        .is_none_or(|task_id| &record.task_id == task_id)
                        && params
                            .run_id
                            .as_ref()
                            .is_none_or(|run_id| &record.run_id == run_id)
                        && params
                            .status
                            .as_ref()
                            .is_none_or(|status| &record.status == status)
                })
                .collect::<Vec<_>>()
        };
        data.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(DevflowApprovalListResponse { data })
    }

    pub(crate) async fn approval_respond(
        &self,
        params: DevflowApprovalRespondParams,
    ) -> Result<DevflowApprovalRespondResponse, JSONRPCErrorError> {
        let record = {
            let store = self.store.lock().await;
            match store.approvals.get(&params.id).cloned() {
                Some(record) => record,
                None if store.approval_history.contains_key(&params.id) => {
                    return Err(invalid_request(format!(
                        "devflow approval is not pending: {}",
                        params.id
                    )));
                }
                None => {
                    return Err(invalid_request(format!(
                        "unknown devflow approval id: {}",
                        params.id
                    )));
                }
            }
        };
        if record.approval.status != DevflowApprovalStatus::Pending {
            return Err(invalid_request(format!(
                "devflow approval is not pending: {}",
                params.id
            )));
        }

        match &record.request {
            PendingDevflowApprovalRequest::QualityGateWaive {
                gate_id,
                waive_reason,
            } => {
                if approval_decision_creates_grant(params.decision) {
                    return Err(invalid_request(
                        "quality gate waive approvals cannot be granted for task or project policy"
                            .to_string(),
                    ));
                }
                if approval_decision_accepts(params.decision) {
                    self.finalize_quality_gate_waive(gate_id, waive_reason)
                        .await?;
                }
            }
            PendingDevflowApprovalRequest::ArtifactDelivery {
                artifact_id,
                target_agent_id,
                destination,
                message,
            } => {
                if approval_decision_creates_grant(params.decision) {
                    return Err(invalid_request(
                        "artifact delivery approvals cannot be granted for task or project policy"
                            .to_string(),
                    ));
                }
                if approval_decision_accepts(params.decision) {
                    let processor = self.clone();
                    let artifact_id = artifact_id.clone();
                    let target_agent_id = target_agent_id.clone();
                    let destination = destination.clone();
                    let message = message.clone();
                    tokio::spawn(async move {
                        let _ = processor
                            .finalize_artifact_delivery(
                                artifact_id,
                                target_agent_id,
                                destination,
                                message,
                            )
                            .await;
                    });
                }
            }
            PendingDevflowApprovalRequest::CommandExecution { request_id, .. }
            | PendingDevflowApprovalRequest::FileChange { request_id, .. }
            | PendingDevflowApprovalRequest::Permissions { request_id, .. } => {
                let result =
                    approval_response_value(&record.request, params.decision, params.scope)?;
                self.outgoing
                    .notify_client_response(request_id.clone(), result)
                    .await;
            }
        }

        let approval = {
            let mut store = self.store.lock().await;
            let mut record = store.approvals.remove(&params.id).ok_or_else(|| {
                invalid_request(format!("unknown devflow approval id: {}", params.id))
            })?;
            if let Some(grant) = devflow_approval_grant(&record.approval, params.decision)
                && !store.approval_grants.contains(&grant)
            {
                store.approval_grants.push(grant);
            }
            mark_approval_responded(&mut record.approval, params.decision);
            let approval = record.approval.clone();
            store
                .approval_history
                .insert(approval.id.clone(), approval.clone());
            store.approvals.insert(approval.id.clone(), record);
            approval
        };
        self.persist_store_best_effort().await;
        Ok(DevflowApprovalRespondResponse { approval })
    }

    pub(crate) async fn task_create(
        &self,
        params: DevflowTaskCreateParams,
    ) -> Result<DevflowTaskCreateResponse, JSONRPCErrorError> {
        if params.project_root.trim().is_empty() {
            return Err(invalid_request("project_root is required".to_string()));
        }
        if params.title.trim().is_empty() {
            return Err(invalid_request("title is required".to_string()));
        }
        if params.objective.trim().is_empty() {
            return Err(invalid_request("objective is required".to_string()));
        }

        let now = Utc::now().timestamp();
        let task = DevflowTask {
            id: Uuid::new_v4().to_string(),
            project_id: params.project_root,
            title: params.title,
            objective: params.objective,
            trigger_source: params
                .trigger_source
                .or_else(|| default_trigger_source(params.assigned_agent_id.as_deref())),
            status: DevflowTaskStatus::Planned,
            kind: params.kind,
            risk_level: params.risk_level,
            dependencies: params.dependencies.unwrap_or_default(),
            assigned_agent_id: params.assigned_agent_id,
            worktree_id: None,
            context_pack_id: None,
            run_ids: Vec::new(),
            artifact_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        {
            let mut store = self.store.lock().await;
            store.tasks.insert(task.id.clone(), task.clone());
        }

        self.send_task_status_changed(task.clone()).await;

        Ok(DevflowTaskCreateResponse { task })
    }

    pub(crate) async fn task_plan(
        &self,
        params: DevflowTaskPlanParams,
    ) -> Result<DevflowTaskPlanResponse, JSONRPCErrorError> {
        if params.project_root.trim().is_empty() {
            return Err(invalid_request("project_root is required".to_string()));
        }
        if params.title.trim().is_empty() {
            return Err(invalid_request("title is required".to_string()));
        }
        if params.objective.trim().is_empty() {
            return Err(invalid_request("objective is required".to_string()));
        }

        let now = Utc::now().timestamp();
        let plan_steps = build_task_plan_steps(&params.title, &params.objective, params.max_tasks);
        let implementation_tasks = plan_steps
            .iter()
            .enumerate()
            .map(|(index, step)| DevflowTask {
                id: Uuid::new_v4().to_string(),
                project_id: params.project_root.clone(),
                title: format!("{} - Workstream {}", params.title, index + 1),
                objective: step.clone(),
                trigger_source: None,
                status: DevflowTaskStatus::Planned,
                kind: DevflowTaskKind::Implementation,
                risk_level: params.risk_level,
                dependencies: Vec::new(),
                assigned_agent_id: Some("codex-main".to_string()),
                worktree_id: None,
                context_pack_id: None,
                run_ids: Vec::new(),
                artifact_ids: Vec::new(),
                created_at: now,
                updated_at: now,
            })
            .collect::<Vec<_>>();
        let review_task = DevflowTask {
            id: Uuid::new_v4().to_string(),
            project_id: params.project_root,
            title: format!("{} - Integration Review", params.title),
            objective: format!(
                "Review and summarize the completed workstreams for: {}",
                params.objective
            ),
            trigger_source: None,
            status: DevflowTaskStatus::Planned,
            kind: DevflowTaskKind::Review,
            risk_level: params.risk_level,
            dependencies: implementation_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect(),
            assigned_agent_id: Some("codex-main".to_string()),
            worktree_id: None,
            context_pack_id: None,
            run_ids: Vec::new(),
            artifact_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        let mut tasks = implementation_tasks;
        tasks.push(review_task);

        let mut plan_artifacts = Vec::new();
        for task in &mut tasks {
            if task.kind == DevflowTaskKind::Implementation
                && task_requires_plan_artifact(task.risk_level)
            {
                let run_id = format!("planner-{}", Uuid::new_v4());
                let path = artifact_file_path(&task.project_id, &run_id, "plan", "md");
                let content = format!(
                    "# Plan: {title}\n\n\
                     ## Objective\n\
                     {objective}\n\n\
                     ## Risk Level\n\
                     {risk_level}\n\n\
                     ## Execution Discipline\n\
                     - Start inside the managed worktree assigned by Devflow.\n\
                     - Produce diff, verification, and review evidence before release prep.\n\
                     - Keep blockers explicit so Integrator and Watchdog queues can route follow-up work.\n",
                    title = task.title,
                    objective = task.objective,
                    risk_level = task_risk_level_label(task.risk_level),
                );
                write_artifact_file(&path, &content).await.map_err(|err| {
                    internal_error(format!(
                        "failed to write devflow planner artifact for task {}: {err}",
                        task.id
                    ))
                })?;
                let artifact = DevflowArtifact {
                    id: Uuid::new_v4().to_string(),
                    task_id: task.id.clone(),
                    run_id,
                    kind: DevflowArtifactKind::Report,
                    title: format!("Plan for {}", task.title),
                    path: path.display().to_string(),
                    mime_type: "text/markdown".to_string(),
                    summary: "Planner-generated plan artifact required before starting medium/high risk work"
                        .to_string(),
                    created_at: now,
                };
                task.artifact_ids.push(artifact.id.clone());
                plan_artifacts.push(artifact);
            }
        }

        {
            let mut store = self.store.lock().await;
            for task in &tasks {
                store.tasks.insert(task.id.clone(), task.clone());
            }
            for artifact in &plan_artifacts {
                store
                    .artifacts
                    .insert(artifact.id.clone(), artifact.clone());
            }
        }

        for task in &tasks {
            self.send_task_status_changed(task.clone()).await;
        }
        for artifact in plan_artifacts {
            self.send_artifact_created(artifact).await;
        }

        Ok(DevflowTaskPlanResponse { data: tasks })
    }

    pub(crate) async fn task_dispatch(
        &self,
        params: DevflowTaskDispatchParams,
    ) -> Result<DevflowTaskDispatchResponse, JSONRPCErrorError> {
        let project_id = params
            .project_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if params.project_id.is_some() && project_id.is_none() {
            return Err(invalid_request("project_id must not be empty".to_string()));
        }

        let requested_task_ids = params
            .task_ids
            .as_ref()
            .map(|task_ids| {
                task_ids
                    .iter()
                    .map(|task_id| task_id.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|task_ids| !task_ids.is_empty());
        if params.task_ids.is_some() && requested_task_ids.is_none() {
            return Err(invalid_request(
                "task_ids must not be empty when provided".to_string(),
            ));
        }
        if let Some(task_ids) = requested_task_ids.as_ref()
            && task_ids.iter().any(String::is_empty)
        {
            return Err(invalid_request(
                "task_ids must not contain empty ids".to_string(),
            ));
        }
        if project_id.is_none() && requested_task_ids.is_none() {
            return Err(invalid_request(
                "project_id or task_ids is required for devflowTask/dispatch".to_string(),
            ));
        }

        let limit = params
            .limit
            .map(|limit| limit as usize)
            .unwrap_or(DEVFLOW_DISPATCH_DEFAULT_LIMIT);
        if limit == 0 {
            return Err(invalid_request("limit must be greater than 0".to_string()));
        }
        let limit = limit.min(DEVFLOW_DISPATCH_MAX_LIMIT);
        let requested_task_ids =
            requested_task_ids.map(|task_ids| task_ids.into_iter().collect::<HashSet<_>>());
        let now = Utc::now().timestamp();

        let (ready_tasks, mut skipped, blocked, blocked_notifications, anchor_project_id) = {
            let mut store = self.store.lock().await;
            if let Some(task_ids) = requested_task_ids.as_ref() {
                for task_id in task_ids {
                    if !store.tasks.contains_key(task_id) {
                        return Err(invalid_request(format!(
                            "unknown devflow task id: {task_id}"
                        )));
                    }
                }
            }

            let mut tasks = store
                .tasks
                .values()
                .filter(|task| project_id.as_deref().is_none_or(|id| task.project_id == id))
                .filter(|task| {
                    requested_task_ids
                        .as_ref()
                        .is_none_or(|task_ids| task_ids.contains(&task.id))
                })
                .cloned()
                .collect::<Vec<_>>();
            tasks.sort_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then_with(|| a.id.cmp(&b.id))
            });

            let mut ready_tasks = Vec::new();
            let mut skipped = Vec::new();
            let mut blocked = Vec::new();
            let mut blocked_notifications = Vec::new();
            for task in tasks {
                if task.kind != DevflowTaskKind::Implementation {
                    skipped.push(DevflowTaskDispatchSkipped {
                        task_id: task.id,
                        title: task.title,
                        status: task.status,
                        reason: "not an implementation task".to_string(),
                    });
                    continue;
                }

                match task.status {
                    DevflowTaskStatus::Planned => {
                        let dependencies = unresolved_dependencies(&store, &task);
                        if dependencies.is_empty() {
                            if task_requires_plan_artifact(task.risk_level)
                                && !task_has_plan_artifact(&store, &task)
                            {
                                blocked.push(DevflowTaskDispatchBlocked {
                                    task_id: task.id.clone(),
                                    title: task.title.clone(),
                                    dependencies: Vec::new(),
                                    reason: "missing required plan artifact".to_string(),
                                });
                                if let Some(task_view) = store.tasks.get_mut(&task.id) {
                                    task_view.status = DevflowTaskStatus::Blocked;
                                    task_view.updated_at = now;
                                    blocked_notifications.push(task_view.clone());
                                }
                            } else if ready_tasks.len() < limit {
                                ready_tasks.push(task);
                            } else {
                                skipped.push(DevflowTaskDispatchSkipped {
                                    task_id: task.id,
                                    title: task.title,
                                    status: task.status,
                                    reason: format!("dispatch limit reached ({limit})"),
                                });
                            }
                        } else {
                            blocked.push(DevflowTaskDispatchBlocked {
                                task_id: task.id.clone(),
                                title: task.title.clone(),
                                dependencies,
                                reason: "unresolved dependencies".to_string(),
                            });
                            if let Some(task_view) = store.tasks.get_mut(&task.id) {
                                task_view.status = DevflowTaskStatus::Blocked;
                                task_view.updated_at = now;
                                blocked_notifications.push(task_view.clone());
                            }
                        }
                    }
                    DevflowTaskStatus::Running => {
                        skipped.push(DevflowTaskDispatchSkipped {
                            task_id: task.id,
                            title: task.title,
                            status: task.status,
                            reason: "already running".to_string(),
                        });
                    }
                    DevflowTaskStatus::Blocked => {
                        let dependencies = unresolved_dependencies(&store, &task);
                        blocked.push(DevflowTaskDispatchBlocked {
                            task_id: task.id.clone(),
                            title: task.title,
                            dependencies,
                            reason: "already blocked; requires dependency resolution, approval, or conflict recovery"
                                .to_string(),
                        });
                    }
                    DevflowTaskStatus::Paused
                    | DevflowTaskStatus::ReadyForReview
                    | DevflowTaskStatus::ReadyToMerge
                    | DevflowTaskStatus::Failed
                    | DevflowTaskStatus::Cancelled => {
                        skipped.push(DevflowTaskDispatchSkipped {
                            task_id: task.id,
                            title: task.title,
                            status: task.status,
                            reason: "not ready for automatic dispatch".to_string(),
                        });
                    }
                }
            }

            let anchor_project_id = project_id.clone().or_else(|| {
                ready_tasks
                    .first()
                    .map(|task| task.project_id.clone())
                    .or_else(|| {
                        blocked
                            .first()
                            .and_then(|item| store.tasks.get(&item.task_id))
                            .map(|task| task.project_id.clone())
                    })
                    .or_else(|| {
                        skipped
                            .first()
                            .and_then(|item| store.tasks.get(&item.task_id))
                            .map(|task| task.project_id.clone())
                    })
            });

            (
                ready_tasks,
                skipped,
                blocked,
                blocked_notifications,
                anchor_project_id,
            )
        };

        for task in blocked_notifications {
            self.send_task_status_changed(task).await;
        }

        let mut started = Vec::new();
        for task in ready_tasks {
            match self
                .start_task_run(
                    &task.id,
                    None,
                    0,
                    DevflowIntegratorMergePolicy::AutoWhenReady,
                )
                .await
            {
                Ok(response) => started.push(DevflowTaskDispatchStarted {
                    task: response.task,
                    run: response.run,
                }),
                Err(err) => skipped.push(DevflowTaskDispatchSkipped {
                    task_id: task.id,
                    title: task.title,
                    status: task.status,
                    reason: err.message,
                }),
            }
        }

        let integrator_artifact = self
            .write_integrator_dispatch_artifact(
                anchor_project_id.as_deref(),
                limit,
                &started,
                &skipped,
                &blocked,
            )
            .await?;
        if let Some(artifact) = integrator_artifact.clone() {
            self.send_artifact_created(artifact).await;
        }

        Ok(DevflowTaskDispatchResponse {
            started,
            skipped,
            blocked,
            integrator_artifact,
        })
    }

    pub(crate) async fn worktree_create(
        &self,
        params: DevflowWorktreeCreateParams,
    ) -> Result<DevflowWorktreeCreateResponse, JSONRPCErrorError> {
        let worktree = self.ensure_managed_worktree(&params.task_id).await?;
        self.outgoing
            .send_server_notification(ServerNotification::DevflowWorktreeStatusChanged(
                DevflowWorktreeStatusChangedNotification {
                    worktree: worktree.clone(),
                },
            ))
            .await;
        Ok(DevflowWorktreeCreateResponse { worktree })
    }

    pub(crate) async fn worktree_list(
        &self,
        params: DevflowWorktreeListParams,
    ) -> Result<DevflowWorktreeListResponse, JSONRPCErrorError> {
        let mut data = list_managed_worktrees(self.config.codex_home.as_path())
            .await
            .map_err(invalid_request)?;
        data.retain(|worktree| {
            params
                .project_id
                .as_ref()
                .is_none_or(|project_id| &worktree.project_id == project_id)
                && params
                    .task_id
                    .as_ref()
                    .is_none_or(|task_id| &worktree.task_id == task_id)
                && params
                    .status
                    .as_ref()
                    .is_none_or(|status| &worktree.status == status)
        });
        let start = params
            .cursor
            .as_deref()
            .and_then(|cursor| cursor.parse::<usize>().ok())
            .unwrap_or(0);
        let limit = params.limit.unwrap_or(100).min(500) as usize;
        let page = data
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor =
            (start + page.len() < data.len()).then(|| (start + page.len()).to_string());
        Ok(DevflowWorktreeListResponse {
            data: page,
            next_cursor,
        })
    }

    pub(crate) async fn worktree_read(
        &self,
        params: DevflowWorktreeReadParams,
    ) -> Result<DevflowWorktreeReadResponse, JSONRPCErrorError> {
        let worktree = read_managed_worktree(self.config.codex_home.as_path(), &params.id)
            .await
            .map_err(invalid_request)?;
        Ok(DevflowWorktreeReadResponse { worktree })
    }

    pub(crate) async fn worktree_diff(
        &self,
        params: DevflowWorktreeDiffParams,
    ) -> Result<DevflowWorktreeDiffResponse, JSONRPCErrorError> {
        let (worktree, diff) = worktree_diff(self.config.codex_home.as_path(), &params.id)
            .await
            .map_err(invalid_request)?;
        Ok(DevflowWorktreeDiffResponse { worktree, diff })
    }

    pub(crate) async fn worktree_merge(
        &self,
        params: DevflowWorktreeMergeParams,
    ) -> Result<DevflowWorktreeMergeResponse, JSONRPCErrorError> {
        let outcome = merge_managed_worktree(self.config.codex_home.as_path(), &params.id)
            .await
            .map_err(invalid_request)?;
        let (task, artifact) = self.record_worktree_merge_outcome(&outcome).await?;

        self.send_task_status_changed(task.clone()).await;
        self.send_artifact_created(artifact).await;

        Ok(DevflowWorktreeMergeResponse {
            merged: outcome.merged,
            worktree: outcome.worktree,
            task,
            conflicts: outcome.conflicts,
        })
    }

    pub(crate) async fn worktree_cleanup(
        &self,
        params: DevflowWorktreeCleanupParams,
    ) -> Result<DevflowWorktreeCleanupResponse, JSONRPCErrorError> {
        let worktree = cleanup_managed_worktree(self.config.codex_home.as_path(), &params.id)
            .await
            .map_err(invalid_request)?;
        self.outgoing
            .send_server_notification(ServerNotification::DevflowWorktreeStatusChanged(
                DevflowWorktreeStatusChangedNotification {
                    worktree: worktree.clone(),
                },
            ))
            .await;
        Ok(DevflowWorktreeCleanupResponse {
            cleaned: true,
            worktree,
        })
    }

    pub(crate) async fn quality_gate_list(
        &self,
        params: DevflowQualityGateListParams,
    ) -> Result<DevflowQualityGateListResponse, JSONRPCErrorError> {
        let store = self.store.lock().await;
        let mut data = store
            .quality_gates
            .values()
            .map(|record| record.gate.clone())
            .filter(|gate| {
                params
                    .task_id
                    .as_ref()
                    .is_none_or(|task_id| &gate.task_id == task_id)
                    && params
                        .run_id
                        .as_ref()
                        .is_none_or(|run_id| &gate.run_id == run_id)
            })
            .collect::<Vec<_>>();
        data.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(DevflowQualityGateListResponse { data })
    }

    pub(crate) async fn quality_gate_read(
        &self,
        params: DevflowQualityGateReadParams,
    ) -> Result<DevflowQualityGateReadResponse, JSONRPCErrorError> {
        let store = self.store.lock().await;
        let gate = store
            .quality_gates
            .get(&params.id)
            .map(|record| record.gate.clone())
            .ok_or_else(|| {
                invalid_request(format!("unknown devflow quality gate id: {}", params.id))
            })?;
        Ok(DevflowQualityGateReadResponse { gate })
    }

    pub(crate) async fn quality_gate_run(
        &self,
        params: DevflowQualityGateRunParams,
    ) -> Result<DevflowQualityGateRunResponse, JSONRPCErrorError> {
        let gate = self
            .start_quality_gate(DevflowQualityGateStartRequest {
                task_id: params.task_id,
                kind: params.kind.unwrap_or(DevflowQualityGateKind::TargetedTest),
                command_override: params.command_override,
                mode: DevflowQualityGateStartMode::Manual,
            })
            .await?;
        Ok(DevflowQualityGateRunResponse { gate })
    }

    pub(crate) async fn quality_gate_rerun(
        &self,
        params: DevflowQualityGateRerunParams,
    ) -> Result<DevflowQualityGateRerunResponse, JSONRPCErrorError> {
        let (task_id, kind, command_override) = {
            let store = self.store.lock().await;
            let record = store.quality_gates.get(&params.id).ok_or_else(|| {
                invalid_request(format!("unknown devflow quality gate id: {}", params.id))
            })?;
            (
                record.gate.task_id.clone(),
                record.gate.kind,
                Some(record.command.command.clone()),
            )
        };
        let gate = self
            .start_quality_gate(DevflowQualityGateStartRequest {
                task_id,
                kind,
                command_override,
                mode: DevflowQualityGateStartMode::Manual,
            })
            .await?;
        Ok(DevflowQualityGateRerunResponse { gate })
    }

    pub(crate) async fn quality_gate_waive(
        &self,
        params: DevflowQualityGateWaiveParams,
    ) -> Result<DevflowQualityGateWaiveResponse, JSONRPCErrorError> {
        if params.reason.trim().is_empty() {
            return Err(invalid_request("waive reason is required".to_string()));
        }
        let (gate, approval) = self
            .request_quality_gate_waive_approval(&params.id, &params.reason)
            .await?;
        Ok(DevflowQualityGateWaiveResponse {
            gate,
            approval: Some(approval),
        })
    }

    async fn request_quality_gate_waive_approval(
        &self,
        gate_id: &str,
        waive_reason: &str,
    ) -> Result<(DevflowQualityGate, DevflowApproval), JSONRPCErrorError> {
        let existing_approval = {
            let store = self.store.lock().await;
            store
                .approvals
                .values()
                .find(|record| {
                    record.approval.quality_gate_id.as_deref() == Some(gate_id)
                        && matches!(
                            record.request,
                            PendingDevflowApprovalRequest::QualityGateWaive { .. }
                        )
                        && record.approval.status == DevflowApprovalStatus::Pending
                })
                .map(|record| record.approval.clone())
        };
        if let Some(approval) = existing_approval {
            let gate = {
                let store = self.store.lock().await;
                store
                    .quality_gates
                    .get(gate_id)
                    .map(|record| record.gate.clone())
                    .ok_or_else(|| {
                        invalid_request(format!("unknown devflow quality gate id: {gate_id}"))
                    })?
            };
            return Ok((gate, approval));
        }

        let (gate, project_root, thread_id, turn_id) = {
            let store = self.store.lock().await;
            let record = store.quality_gates.get(gate_id).ok_or_else(|| {
                invalid_request(format!("unknown devflow quality gate id: {gate_id}"))
            })?;
            if record.gate.status == DevflowQualityGateStatus::Waived {
                return Err(invalid_request(format!(
                    "devflow quality gate is already waived: {gate_id}"
                )));
            }
            if record.gate.status != DevflowQualityGateStatus::Failed {
                return Err(invalid_request(format!(
                    "devflow quality gate must be failed before waive: {gate_id}"
                )));
            }
            let run = store.runs.get(&record.gate.run_id).ok_or_else(|| {
                invalid_request(format!(
                    "unknown devflow run for quality gate: {}",
                    record.gate.run_id
                ))
            })?;
            (
                record.gate.clone(),
                run.project_root.clone(),
                run.run.thread_id.clone().unwrap_or_default(),
                run.run.turn_id.clone().unwrap_or_default(),
            )
        };

        let approval = DevflowApproval {
            id: Uuid::new_v4().to_string(),
            project_id: project_root,
            task_id: gate.task_id.clone(),
            run_id: gate.run_id.clone(),
            quality_gate_id: Some(gate.id.clone()),
            request_id: format!("quality-gate-waive:{}", gate.id),
            thread_id,
            turn_id,
            item_id: String::new(),
            kind: DevflowApprovalKind::QualityGateWaive,
            status: DevflowApprovalStatus::Pending,
            reason: Some(waive_reason.to_string()),
            command: Some(gate.command.clone()),
            cwd: Some(gate.cwd.clone()),
            file_paths: Vec::new(),
            requested_permissions: None,
            responded_at: None,
            decision: None,
            created_at: Utc::now().timestamp(),
        };
        let approval_record = DevflowApprovalRecord {
            approval: approval.clone(),
            request: PendingDevflowApprovalRequest::QualityGateWaive {
                gate_id: gate.id.clone(),
                waive_reason: waive_reason.to_string(),
            },
        };
        {
            let mut store = self.store.lock().await;
            store
                .approval_history
                .insert(approval.id.clone(), approval.clone());
            store
                .approvals
                .insert(approval_record.approval.id.clone(), approval_record.clone());
        }
        self.send_approval_requested(approval.clone()).await;
        Ok((gate, approval))
    }

    async fn finalize_quality_gate_waive(
        &self,
        gate_id: &str,
        waive_reason: &str,
    ) -> Result<(), JSONRPCErrorError> {
        let (gate, task_id, run_id) = {
            let mut store = self.store.lock().await;
            let record = store.quality_gates.get_mut(gate_id).ok_or_else(|| {
                invalid_request(format!("unknown devflow quality gate id: {gate_id}"))
            })?;
            record.gate.status = DevflowQualityGateStatus::Waived;
            record.gate.waived_reason = Some(waive_reason.to_string());
            record.gate.updated_at = Utc::now().timestamp();
            (
                record.gate.clone(),
                record.gate.task_id.clone(),
                record.gate.run_id.clone(),
            )
        };
        self.send_quality_gate_completed(gate.clone()).await;
        self.start_review_for_run(&task_id, &run_id).await?;
        Ok(())
    }

    pub(crate) async fn artifact_list(
        &self,
        params: DevflowArtifactListParams,
    ) -> Result<DevflowArtifactListResponse, JSONRPCErrorError> {
        let mut data = {
            let store = self.store.lock().await;
            store
                .artifacts
                .values()
                .filter(|artifact| {
                    params
                        .task_id
                        .as_ref()
                        .is_none_or(|task_id| &artifact.task_id == task_id)
                        && params
                            .run_id
                            .as_ref()
                            .is_none_or(|run_id| &artifact.run_id == run_id)
                        && params
                            .kind
                            .as_ref()
                            .is_none_or(|kind| &artifact.kind == kind)
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        data.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        let start = params
            .cursor
            .as_deref()
            .and_then(|cursor| cursor.parse::<usize>().ok())
            .unwrap_or(0);
        let limit = params.limit.unwrap_or(100).min(500) as usize;
        let page = data
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor =
            (start + page.len() < data.len()).then(|| (start + page.len()).to_string());
        Ok(DevflowArtifactListResponse {
            data: page,
            next_cursor,
        })
    }

    pub(crate) async fn artifact_read(
        &self,
        params: DevflowArtifactReadParams,
    ) -> Result<DevflowArtifactReadResponse, JSONRPCErrorError> {
        let artifact = {
            let store = self.store.lock().await;
            store.artifacts.get(&params.id).cloned().ok_or_else(|| {
                invalid_request(format!("unknown devflow artifact id: {}", params.id))
            })?
        };
        let contents = fs::read_to_string(&artifact.path)
            .await
            .map_err(|err| internal_error(format!("failed to read artifact contents: {err}")))?;
        Ok(DevflowArtifactReadResponse { artifact, contents })
    }

    pub(crate) async fn artifact_open(
        &self,
        params: DevflowArtifactOpenParams,
    ) -> Result<DevflowArtifactOpenResponse, JSONRPCErrorError> {
        let artifact = {
            let store = self.store.lock().await;
            store.artifacts.get(&params.id).cloned().ok_or_else(|| {
                invalid_request(format!("unknown devflow artifact id: {}", params.id))
            })?
        };
        Ok(DevflowArtifactOpenResponse { artifact })
    }

    pub(crate) async fn artifact_export(
        &self,
        params: DevflowArtifactExportParams,
    ) -> Result<DevflowArtifactExportResponse, JSONRPCErrorError> {
        let artifact = {
            let store = self.store.lock().await;
            store.artifacts.get(&params.id).cloned().ok_or_else(|| {
                invalid_request(format!("unknown devflow artifact id: {}", params.id))
            })?
        };
        if let Some(parent) = Path::new(&params.destination_path).parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|err| internal_error(format!("failed to create export dir: {err}")))?;
        }
        fs::copy(&artifact.path, &params.destination_path)
            .await
            .map_err(|err| internal_error(format!("failed to export artifact: {err}")))?;
        Ok(DevflowArtifactExportResponse {
            artifact,
            destination_path: params.destination_path,
        })
    }

    pub(crate) async fn artifact_deliver(
        &self,
        params: DevflowArtifactDeliverParams,
    ) -> Result<DevflowArtifactDeliverResponse, JSONRPCErrorError> {
        let DevflowArtifactDeliverParams {
            id,
            target_agent_id,
            destination,
            message,
        } = params;

        if target_agent_id != "hermes-automation" {
            return Err(invalid_request(format!(
                "devflow artifact delivery only supports hermes-automation in the current MVP: {target_agent_id}"
            )));
        }
        let destination = destination.trim().to_string();
        if destination.is_empty() {
            return Err(invalid_request(
                "devflow artifact delivery destination must not be empty".to_string(),
            ));
        }

        let (artifact, project_root) = self.artifact_delivery_context(&id).await?;
        if destination == "local" || destination.starts_with("local:") {
            return self
                .run_artifact_delivery(
                    artifact,
                    project_root,
                    target_agent_id,
                    destination,
                    message,
                )
                .await;
        }

        let approval = self
            .request_artifact_delivery_approval(
                &artifact,
                &project_root,
                &target_agent_id,
                &destination,
                message,
            )
            .await?;

        Ok(DevflowArtifactDeliverResponse {
            artifact,
            receipt_artifact: None,
            approval: Some(approval),
            target_agent_id,
            destination,
            command: ARTIFACT_DELIVERY_COMMAND.to_string(),
            exit_code: None,
            status: DevflowArtifactDeliveryStatus::PendingApproval,
            output_summary: "waiting for devflow approval before external Hermes delivery"
                .to_string(),
            delivered_at: None,
        })
    }

    async fn request_artifact_delivery_approval(
        &self,
        artifact: &DevflowArtifact,
        project_root: &str,
        target_agent_id: &str,
        destination: &str,
        message: Option<String>,
    ) -> Result<DevflowApproval, JSONRPCErrorError> {
        let existing_approval = {
            let store = self.store.lock().await;
            store
                .approvals
                .values()
                .find(|record| {
                    if record.approval.status != DevflowApprovalStatus::Pending {
                        return false;
                    }
                    match &record.request {
                        PendingDevflowApprovalRequest::ArtifactDelivery {
                            artifact_id,
                            target_agent_id: existing_target_agent_id,
                            destination: existing_destination,
                            message: existing_message,
                        } => {
                            artifact_id == &artifact.id
                                && existing_target_agent_id == target_agent_id
                                && existing_destination == destination
                                && existing_message == &message
                        }
                        PendingDevflowApprovalRequest::CommandExecution { .. }
                        | PendingDevflowApprovalRequest::FileChange { .. }
                        | PendingDevflowApprovalRequest::Permissions { .. }
                        | PendingDevflowApprovalRequest::QualityGateWaive { .. } => false,
                    }
                })
                .map(|record| record.approval.clone())
        };
        if let Some(approval) = existing_approval {
            return Ok(approval);
        }

        let (thread_id, turn_id) = {
            let store = self.store.lock().await;
            let record = store.runs.get(&artifact.run_id).ok_or_else(|| {
                invalid_request(format!(
                    "unknown devflow run id for artifact delivery approval: {}",
                    artifact.run_id
                ))
            })?;
            (
                record.run.thread_id.clone().unwrap_or_default(),
                record.run.turn_id.clone().unwrap_or_default(),
            )
        };
        let approval = DevflowApproval {
            id: Uuid::new_v4().to_string(),
            project_id: project_root.to_string(),
            task_id: artifact.task_id.clone(),
            run_id: artifact.run_id.clone(),
            quality_gate_id: None,
            request_id: format!("artifact-delivery:{}", Uuid::new_v4()),
            thread_id,
            turn_id,
            item_id: String::new(),
            kind: DevflowApprovalKind::ArtifactDelivery,
            status: DevflowApprovalStatus::Pending,
            reason: Some(format!(
                "Deliver Devflow artifact {} to external Hermes destination {destination}",
                artifact.id
            )),
            command: Some(ARTIFACT_DELIVERY_COMMAND.to_string()),
            cwd: Some(project_root.to_string()),
            file_paths: vec![artifact.path.clone()],
            requested_permissions: None,
            responded_at: None,
            decision: None,
            created_at: Utc::now().timestamp(),
        };
        let approval_record = DevflowApprovalRecord {
            approval: approval.clone(),
            request: PendingDevflowApprovalRequest::ArtifactDelivery {
                artifact_id: artifact.id.clone(),
                target_agent_id: target_agent_id.to_string(),
                destination: destination.to_string(),
                message,
            },
        };
        {
            let mut store = self.store.lock().await;
            store
                .approval_history
                .insert(approval.id.clone(), approval.clone());
            store
                .approvals
                .insert(approval_record.approval.id.clone(), approval_record.clone());
        }
        self.send_approval_requested(approval.clone()).await;
        Ok(approval)
    }

    async fn finalize_artifact_delivery(
        &self,
        artifact_id: String,
        target_agent_id: String,
        destination: String,
        message: Option<String>,
    ) -> Result<DevflowArtifactDeliverResponse, JSONRPCErrorError> {
        let (artifact, project_root) = self.artifact_delivery_context(&artifact_id).await?;
        self.run_artifact_delivery(
            artifact,
            project_root,
            target_agent_id,
            destination,
            message,
        )
        .await
    }

    async fn artifact_delivery_context(
        &self,
        artifact_id: &str,
    ) -> Result<(DevflowArtifact, String), JSONRPCErrorError> {
        let (artifact, project_root) = {
            let store = self.store.lock().await;
            let artifact = store.artifacts.get(artifact_id).cloned().ok_or_else(|| {
                invalid_request(format!("unknown devflow artifact id: {artifact_id}"))
            })?;
            let project_root = store
                .tasks
                .get(&artifact.task_id)
                .map(|task| task.project_id.clone())
                .ok_or_else(|| {
                    internal_error(format!(
                        "unknown devflow task id for artifact delivery: {}",
                        artifact.task_id
                    ))
                })?;
            (artifact, project_root)
        };
        Ok((artifact, project_root))
    }

    async fn run_artifact_delivery(
        &self,
        artifact: DevflowArtifact,
        project_root: String,
        target_agent_id: String,
        destination: String,
        message: Option<String>,
    ) -> Result<DevflowArtifactDeliverResponse, JSONRPCErrorError> {
        let delivery_message = message.unwrap_or_else(|| {
            "Deliver this Devflow artifact and return a concise delivery receipt.".to_string()
        });
        let prompt = format!(
            "You are Hermes handling a Devflow artifact delivery request.\n\nDestination: {destination}\nMessage: {delivery_message}\nArtifact ID: {}\nArtifact title: {}\nArtifact kind: {:?}\nArtifact path: {}\n\nRead the artifact file if needed, deliver it to the requested destination using the safest configured Hermes messaging path, and return a concise delivery receipt. If the destination is local, do not send an external message; print the receipt only.",
            artifact.id, artifact.title, artifact.kind, artifact.path
        );
        let command = ARTIFACT_DELIVERY_COMMAND.to_string();
        let command_args = [
            "chat".to_string(),
            "-Q".to_string(),
            "--source".to_string(),
            "devflow".to_string(),
            "--max-turns".to_string(),
            "5".to_string(),
            "-q".to_string(),
            prompt,
        ];
        let command_arg_refs = command_args.iter().map(String::as_str).collect::<Vec<_>>();
        let execution_result =
            run_hermes_command(Path::new(&project_root), &command_arg_refs).await;
        let (exit_code, output_summary) = match execution_result {
            Ok(execution) => (
                execution.exit_code,
                truncate(
                    &combine_external_agent_output(&execution),
                    COMMAND_OUTPUT_SUMMARY_LIMIT,
                ),
            ),
            Err(err) => (
                None,
                truncate(
                    &format!("failed to run Hermes artifact delivery: {err}"),
                    COMMAND_OUTPUT_SUMMARY_LIMIT,
                ),
            ),
        };
        let status = if exit_code == Some(0) {
            DevflowArtifactDeliveryStatus::Delivered
        } else {
            DevflowArtifactDeliveryStatus::Failed
        };
        let delivered_at = Utc::now().timestamp();
        let status_label = match status {
            DevflowArtifactDeliveryStatus::PendingApproval => "pending approval",
            DevflowArtifactDeliveryStatus::Delivered => "delivered",
            DevflowArtifactDeliveryStatus::Failed => "failed",
        };
        let receipt_id = Uuid::new_v4().to_string();
        let receipt_artifact = DevflowArtifact {
            id: receipt_id.clone(),
            task_id: artifact.task_id.clone(),
            run_id: artifact.run_id.clone(),
            kind: DevflowArtifactKind::DeliveryReceipt,
            title: format!("Delivery receipt for {}", artifact.title),
            path: artifact_file_path(
                &project_root,
                &artifact.run_id,
                &format!("delivery-{receipt_id}"),
                "md",
            )
            .display()
            .to_string(),
            mime_type: "text/markdown".to_string(),
            summary: format!(
                "Hermes artifact delivery {status_label} for destination {destination}"
            ),
            created_at: delivered_at,
        };
        let receipt = format!(
            "# {}\n\n- Artifact ID: {}\n- Artifact title: {}\n- Target agent: {}\n- Destination: {}\n- Status: {}\n- Command: `{}`\n- Exit code: {}\n- Delivered at: {}\n\n## Hermes output\n\n```text\n{}\n```\n",
            receipt_artifact.title,
            artifact.id,
            artifact.title,
            target_agent_id,
            destination,
            status_label,
            command,
            exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "none".to_string()),
            delivered_at,
            output_summary
        );
        write_artifact_file(Path::new(&receipt_artifact.path), &receipt)
            .await
            .map_err(|err| {
                internal_error(format!(
                    "failed to write devflow artifact delivery receipt: {err}"
                ))
            })?;

        {
            let mut store = self.store.lock().await;
            let Some(record) = store.runs.get_mut(&artifact.run_id) else {
                return Err(internal_error(format!(
                    "unknown devflow run id for artifact delivery: {}",
                    artifact.run_id
                )));
            };
            if !record.run.artifact_ids.contains(&receipt_artifact.id) {
                record.run.artifact_ids.push(receipt_artifact.id.clone());
            }
            let Some(task) = store.tasks.get_mut(&artifact.task_id) else {
                return Err(internal_error(format!(
                    "unknown devflow task id for artifact delivery: {}",
                    artifact.task_id
                )));
            };
            if !task.artifact_ids.contains(&receipt_artifact.id) {
                task.artifact_ids.push(receipt_artifact.id.clone());
            }
            store
                .artifacts
                .insert(receipt_artifact.id.clone(), receipt_artifact.clone());
        }
        self.send_artifact_created(receipt_artifact.clone()).await;

        Ok(DevflowArtifactDeliverResponse {
            artifact,
            receipt_artifact: Some(receipt_artifact),
            approval: None,
            target_agent_id,
            destination,
            command,
            exit_code,
            status,
            output_summary,
            delivered_at: Some(delivered_at),
        })
    }

    async fn start_quality_gate(
        &self,
        request: DevflowQualityGateStartRequest,
    ) -> Result<DevflowQualityGate, JSONRPCErrorError> {
        let requested_task_id = request.task_id.clone();
        let (task, run_record) = {
            let store = self.store.lock().await;
            let task = store
                .tasks
                .get(&requested_task_id)
                .cloned()
                .ok_or_else(|| {
                    invalid_request(format!("unknown devflow task id: {requested_task_id}"))
                })?;
            let run_id = task.run_ids.last().cloned().ok_or_else(|| {
                invalid_request(format!("task has no devflow run: {requested_task_id}"))
            })?;
            let run_record = store
                .runs
                .get(&run_id)
                .cloned()
                .ok_or_else(|| invalid_request(format!("unknown devflow run id: {run_id}")))?;
            (task, run_record)
        };
        let execution_cwd = if let Some(worktree_id) = task.worktree_id.as_deref() {
            read_managed_worktree(self.config.codex_home.as_path(), worktree_id)
                .await
                .map_err(invalid_request)?
                .cwd_path
        } else {
            task.project_id.clone()
        };

        let gate_command = if let Some(command_override) = request.command_override {
            let argv = command_override
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>();
            if argv.is_empty() {
                return Err(invalid_request(
                    "quality gate command override must not be empty".to_string(),
                ));
            }
            GateCommand {
                command: command_override,
                argv,
            }
        } else {
            quality_gate_command(Path::new(&execution_cwd), &task, request.kind)
                .map_err(invalid_request)?
        };

        let now = Utc::now().timestamp();
        let gate = DevflowQualityGate {
            id: Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            run_id: run_record.run.id.clone(),
            kind: request.kind,
            status: DevflowQualityGateStatus::Running,
            command: gate_command.command.clone(),
            cwd: execution_cwd.clone(),
            exit_code: None,
            duration_ms: None,
            summary: None,
            artifact_id: None,
            waived_reason: None,
            created_at: now,
            updated_at: now,
        };

        {
            let mut store = self.store.lock().await;
            if let Some(task_entry) = store.tasks.get_mut(&task.id) {
                task_entry.status = DevflowTaskStatus::Running;
                task_entry.updated_at = now;
            }
            if let Some(record) = store.runs.get_mut(&run_record.run.id) {
                record.run.status = DevflowRunStatus::Running;
                record.quality_gate_id = Some(gate.id.clone());
            }
            store.quality_gates.insert(
                gate.id.clone(),
                DevflowQualityGateRecord {
                    gate: gate.clone(),
                    command: gate_command,
                },
            );
        }
        self.persist_store_best_effort().await;

        let processor = self.clone();
        let gate_id = gate.id.clone();
        let mode = request.mode;
        tokio::spawn(async move {
            processor.run_quality_gate(gate_id, mode).await;
        });

        Ok(gate)
    }

    async fn run_quality_gate(&self, mut gate_id: String, mut mode: DevflowQualityGateStartMode) {
        loop {
            let (gate_record, task, run_record) = {
                let store = self.store.lock().await;
                let Some(gate_record) = store.quality_gates.get(&gate_id).cloned() else {
                    return;
                };
                let Some(task) = store.tasks.get(&gate_record.gate.task_id).cloned() else {
                    return;
                };
                let Some(run_record) = store.runs.get(&gate_record.gate.run_id).cloned() else {
                    return;
                };
                (gate_record, task, run_record)
            };

            let execution = match run_gate_command(
                Path::new(&gate_record.gate.cwd),
                &gate_record.command,
            )
            .await
            {
                Ok(execution) => execution,
                Err(err) => {
                    self.complete_quality_gate_failure(&gate_id, &task.id, &run_record.run.id, err)
                        .await;
                    return;
                }
            };

            let output = combine_gate_output(&execution);
            let artifact = match self
                .write_quality_gate_artifact(&gate_record.gate, &task, &output)
                .await
            {
                Ok(artifact) => artifact,
                Err(err) => {
                    self.complete_quality_gate_failure(
                        &gate_id,
                        &task.id,
                        &run_record.run.id,
                        format!("failed to persist quality gate artifact: {err}"),
                    )
                    .await;
                    return;
                }
            };

            let (gate, passed) = {
                let mut store = self.store.lock().await;
                let Some(gate_record) = store.quality_gates.get_mut(&gate_id) else {
                    return;
                };
                gate_record.gate.updated_at = Utc::now().timestamp();
                gate_record.gate.exit_code = execution.exit_code;
                gate_record.gate.duration_ms = Some(execution.duration_ms);
                gate_record.gate.summary = Some(truncate(&output, COMMAND_OUTPUT_SUMMARY_LIMIT));
                gate_record.gate.artifact_id = Some(artifact.id.clone());
                let passed = execution.exit_code == Some(0);
                gate_record.gate.status = if passed {
                    DevflowQualityGateStatus::Passed
                } else {
                    DevflowQualityGateStatus::Failed
                };
                let gate = gate_record.gate.clone();
                if let Some(task) = store.tasks.get_mut(&task.id)
                    && !task.artifact_ids.contains(&artifact.id)
                {
                    task.artifact_ids.push(artifact.id.clone());
                }
                if let Some(run_record) = store.runs.get_mut(&run_record.run.id)
                    && !run_record.run.artifact_ids.contains(&artifact.id)
                {
                    run_record.run.artifact_ids.push(artifact.id.clone());
                }
                store
                    .artifacts
                    .insert(artifact.id.clone(), artifact.clone());
                (gate, passed)
            };

            self.send_artifact_created(artifact.clone()).await;
            self.send_quality_gate_completed(gate.clone()).await;

            if passed {
                match self
                    .start_next_quality_gate_or_review(&task, &run_record)
                    .await
                {
                    Ok(Some(next_gate)) => {
                        gate_id = next_gate.id;
                        mode = DevflowQualityGateStartMode::Automatic;
                        continue;
                    }
                    Ok(None) => {}
                    Err(err) => {
                        self.complete_quality_gate_failure(
                            &gate_id,
                            &task.id,
                            &run_record.run.id,
                            format!(
                                "failed to continue validation after quality gate: {}",
                                err.message
                            ),
                        )
                        .await;
                    }
                }
            } else if mode == DevflowQualityGateStartMode::Automatic
                && let Err(err) = self
                    .queue_auto_repair_after_gate_failure(&task, &run_record, &gate, &artifact)
                    .await
            {
                self.mark_run_failed(
                    &task.id,
                    &run_record.run.id,
                    format!("quality gate failed and auto repair could not start: {err}"),
                    Utc::now().timestamp(),
                )
                .await;
            }
            return;
        }
    }

    async fn start_next_quality_gate_or_review(
        &self,
        task: &DevflowTask,
        run_record: &DevflowRunRecord,
    ) -> Result<Option<DevflowQualityGate>, JSONRPCErrorError> {
        let next_required_gate = {
            let required_gates = required_quality_gates(task);
            let store = self.store.lock().await;
            required_gates
                .into_iter()
                .find(|required_gate| {
                    let has_completed_gate = store.quality_gates.values().any(|record| {
                        record.gate.task_id == task.id
                            && record.gate.run_id == run_record.run.id
                            && record.gate.kind == required_gate.kind
                            && matches!(
                                record.gate.status,
                                DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Waived
                            )
                            && record.gate.artifact_id.as_ref().is_some_and(|artifact_id| {
                                store.artifacts.contains_key(artifact_id)
                            })
                    });
                    let has_in_flight_gate = store.quality_gates.values().any(|record| {
                        record.gate.task_id == task.id
                            && record.gate.run_id == run_record.run.id
                            && record.gate.kind == required_gate.kind
                            && matches!(
                                record.gate.status,
                                DevflowQualityGateStatus::Queued
                                    | DevflowQualityGateStatus::Running
                            )
                    });
                    !has_completed_gate && !has_in_flight_gate
                })
                .map(|required_gate| required_gate.kind)
        };

        if let Some(kind) = next_required_gate {
            let gate = self
                .create_required_quality_gate_for_run(task, run_record, kind)
                .await?;
            return Ok(Some(gate));
        }

        self.start_review_for_run(&task.id, &run_record.run.id)
            .await?;
        Ok(None)
    }

    async fn create_required_quality_gate_for_run(
        &self,
        task: &DevflowTask,
        run_record: &DevflowRunRecord,
        kind: DevflowQualityGateKind,
    ) -> Result<DevflowQualityGate, JSONRPCErrorError> {
        let execution_cwd = if let Some(worktree_id) = task.worktree_id.as_deref() {
            read_managed_worktree(self.config.codex_home.as_path(), worktree_id)
                .await
                .map_err(invalid_request)?
                .cwd_path
        } else {
            task.project_id.clone()
        };
        let gate_command =
            quality_gate_command(Path::new(&execution_cwd), task, kind).map_err(invalid_request)?;
        let now = Utc::now().timestamp();
        let gate = DevflowQualityGate {
            id: Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            run_id: run_record.run.id.clone(),
            kind,
            status: DevflowQualityGateStatus::Running,
            command: gate_command.command.clone(),
            cwd: execution_cwd,
            exit_code: None,
            duration_ms: None,
            summary: None,
            artifact_id: None,
            waived_reason: None,
            created_at: now,
            updated_at: now,
        };

        {
            let mut store = self.store.lock().await;
            if let Some(task_entry) = store.tasks.get_mut(&task.id) {
                task_entry.status = DevflowTaskStatus::Running;
                task_entry.updated_at = now;
            }
            if let Some(record) = store.runs.get_mut(&run_record.run.id) {
                record.run.status = DevflowRunStatus::Running;
                record.quality_gate_id = Some(gate.id.clone());
            }
            store.quality_gates.insert(
                gate.id.clone(),
                DevflowQualityGateRecord {
                    gate: gate.clone(),
                    command: gate_command,
                },
            );
        }
        self.persist_store_best_effort().await;

        Ok(gate)
    }

    async fn queue_auto_repair_after_gate_failure(
        &self,
        task: &DevflowTask,
        run_record: &DevflowRunRecord,
        gate: &DevflowQualityGate,
        artifact: &DevflowArtifact,
    ) -> Result<(), String> {
        if task.kind != DevflowTaskKind::Implementation {
            self.mark_run_failed(
                &task.id,
                &run_record.run.id,
                "quality gate failed".to_string(),
                Utc::now().timestamp(),
            )
            .await;
            return Ok(());
        }
        if run_record.auto_repair_attempt >= 1 {
            self.mark_run_failed(
                &task.id,
                &run_record.run.id,
                "quality gate failed after one automatic repair attempt".to_string(),
                Utc::now().timestamp(),
            )
            .await;
            return Ok(());
        }

        let gate_output = fs::read_to_string(&artifact.path)
            .await
            .unwrap_or_else(|_| artifact.summary.clone());
        let repair_prompt = format!(
            "You are executing a Devflow automatic repair pass.\n\nProject root: {}\nTask title: {}\nObjective: {}\n\nThe previous run failed the automatic quality gate.\n\nFailed gate command: {}\nGate exit code: {}\nGate summary: {}\n\nGate output:\n```text\n{}\n```\n\nFix the issue inside the current worktree, run the most relevant focused verification you can, and leave the result ready for review.",
            task.project_id,
            task.title,
            task.objective,
            gate.command,
            gate.exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            gate.summary
                .clone()
                .unwrap_or_else(|| "quality gate failed".to_string()),
            truncate(&gate_output, 12_000)
        );
        let now = Utc::now().timestamp();
        let failed_run = {
            let mut store = self.store.lock().await;
            if let Some(task_entry) = store.tasks.get_mut(&task.id) {
                task_entry.status = DevflowTaskStatus::Planned;
                task_entry.updated_at = now;
            }
            let Some(record) = store.runs.get_mut(&run_record.run.id) else {
                return Err("run disappeared before auto repair".to_string());
            };
            record.run.status = DevflowRunStatus::Failed;
            record.run.completed_at = Some(now);
            record.run.exit_reason = Some("quality gate failed; auto repair queued".to_string());
            record.run.clone()
        };
        self.send_run_status_changed(failed_run).await;
        self.start_task_run(
            &task.id,
            Some(repair_prompt),
            run_record.auto_repair_attempt + 1,
            if run_record.auto_integrator_merge {
                DevflowIntegratorMergePolicy::AutoWhenReady
            } else {
                DevflowIntegratorMergePolicy::Manual
            },
        )
        .await
        .map(|_| ())
        .map_err(|err| err.message)
    }

    async fn complete_quality_gate_failure(
        &self,
        gate_id: &str,
        task_id: &str,
        run_id: &str,
        message: String,
    ) {
        let gate = {
            let mut store = self.store.lock().await;
            let Some(gate_record) = store.quality_gates.get_mut(gate_id) else {
                return;
            };
            gate_record.gate.status = DevflowQualityGateStatus::Failed;
            gate_record.gate.updated_at = Utc::now().timestamp();
            gate_record.gate.summary = Some(message.clone());
            gate_record.gate.clone()
        };
        self.send_quality_gate_completed(gate).await;
        self.mark_run_failed(task_id, run_id, message, Utc::now().timestamp())
            .await;
    }

    async fn start_review_for_run(
        &self,
        task_id: &str,
        run_id: &str,
    ) -> Result<(), JSONRPCErrorError> {
        let thread_id = {
            let mut store = self.store.lock().await;
            let record = store
                .runs
                .get_mut(run_id)
                .ok_or_else(|| invalid_request(format!("unknown devflow run id: {run_id}")))?;
            if record.review_requested {
                return Ok(());
            }
            record.review_requested = true;
            record
                .run
                .thread_id
                .clone()
                .ok_or_else(|| invalid_request(format!("run has no thread id: {run_id}")))?
        };

        let thread_uuid = ThreadId::from_string(&thread_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;
        let thread = self
            .thread_manager
            .get_thread(thread_uuid)
            .await
            .map_err(|_| invalid_request(format!("thread not found: {thread_id}")))?;
        let review_request = ReviewRequest {
            target: ReviewTarget::UncommittedChanges,
            user_facing_hint: Some("current changes".to_string()),
        };
        thread
            .submit(Op::Review { review_request })
            .await
            .map_err(|err| {
                internal_error(format!(
                    "failed to start devflow review for task {task_id}: {err}"
                ))
            })?;
        Ok(())
    }

    pub(crate) async fn task_start(
        &self,
        params: DevflowTaskStartParams,
    ) -> Result<DevflowTaskStartResponse, JSONRPCErrorError> {
        self.start_task_run(&params.id, None, 0, DevflowIntegratorMergePolicy::Manual)
            .await
    }

    async fn start_task_run(
        &self,
        task_id: &str,
        prompt_override: Option<String>,
        auto_repair_attempt: u32,
        integrator_merge_policy: DevflowIntegratorMergePolicy,
    ) -> Result<DevflowTaskStartResponse, JSONRPCErrorError> {
        let mut task = {
            let store = self.store.lock().await;
            store
                .tasks
                .get(task_id)
                .cloned()
                .ok_or_else(|| invalid_request(format!("unknown devflow task id: {task_id}")))?
        };

        if task.status == DevflowTaskStatus::Running {
            return Err(invalid_request(format!(
                "devflow task is already running: {}",
                task.id
            )));
        }

        let blocking_dependencies = {
            let store = self.store.lock().await;
            unresolved_dependencies(&store, &task)
        };
        if !blocking_dependencies.is_empty() {
            task.status = DevflowTaskStatus::Blocked;
            task.updated_at = Utc::now().timestamp();
            {
                let mut store = self.store.lock().await;
                store.tasks.insert(task.id.clone(), task.clone());
            }
            self.send_task_status_changed(task.clone()).await;
            return Err(invalid_request(format!(
                "task is blocked by unresolved dependencies: {}",
                blocking_dependencies.join(", ")
            )));
        }

        let missing_plan_artifact = {
            let store = self.store.lock().await;
            task_requires_plan_artifact(task.risk_level) && !task_has_plan_artifact(&store, &task)
        };
        if missing_plan_artifact {
            task.status = DevflowTaskStatus::Blocked;
            task.updated_at = Utc::now().timestamp();
            {
                let mut store = self.store.lock().await;
                store.tasks.insert(task.id.clone(), task.clone());
            }
            self.send_task_status_changed(task.clone()).await;
            return Err(invalid_request(format!(
                "task requires a plan artifact before start because risk level is {}: {}",
                task_risk_level_label(task.risk_level),
                task.id
            )));
        }

        let now = Utc::now().timestamp();
        let managed_worktree = if task.kind == DevflowTaskKind::Implementation
            && get_git_repo_root(Path::new(&task.project_id)).is_some()
        {
            Some(self.ensure_managed_worktree(&task.id).await?)
        } else {
            None
        };
        if let Some(worktree) = managed_worktree.as_ref() {
            task.worktree_id = Some(worktree.id.clone());
        }
        let agent_id = task
            .assigned_agent_id
            .clone()
            .unwrap_or_else(|| default_agent_id(task.kind).to_string());
        let uses_legacy_automation_agent = matches!(
            (task.kind, agent_id.as_str()),
            (DevflowTaskKind::Automation, "hermes-automation")
        );
        let uses_legacy_text_agent = matches!(
            (task.kind, agent_id.as_str()),
            (DevflowTaskKind::Report, "claude-writer")
                | (DevflowTaskKind::Review, "claude-reviewer")
        );
        let input = prompt_override.unwrap_or_else(|| devflow_turn_prompt(&task));
        let run_id = Uuid::new_v4().to_string();
        let internal_connection_id = ConnectionId(
            self.next_internal_connection_id
                .fetch_add(1, Ordering::Relaxed),
        );

        let mut run = DevflowRun {
            id: run_id.clone(),
            task_id: task.id.clone(),
            agent_id: agent_id.clone(),
            thread_id: None,
            turn_id: None,
            status: DevflowRunStatus::Queued,
            started_at: now,
            completed_at: None,
            input,
            stream_summary: None,
            command_ids: Vec::new(),
            artifact_ids: Vec::new(),
            exit_reason: None,
        };

        let context_artifact = self.write_context_pack_artifact(&task, &run).await?;
        run.artifact_ids.push(context_artifact.id.clone());
        task.artifact_ids.push(context_artifact.id.clone());
        task.run_ids.push(run.id.clone());
        task.context_pack_id = Some(context_artifact.id.clone());
        task.assigned_agent_id = Some(agent_id);
        task.status = DevflowTaskStatus::Running;
        task.updated_at = now;

        {
            let mut store = self.store.lock().await;
            store
                .artifacts
                .insert(context_artifact.id.clone(), context_artifact.clone());
            store.tasks.insert(task.id.clone(), task.clone());
            store.runs.insert(
                run.id.clone(),
                DevflowRunRecord {
                    run: run.clone(),
                    project_root: task.project_id.clone(),
                    internal_connection_id,
                    diff_artifact_id: None,
                    summary_artifact_id: None,
                    output_archive_artifact_id: None,
                    review_artifact_id: None,
                    quality_gate_id: None,
                    review_requested: false,
                    review_completed: false,
                    auto_repair_attempt,
                    auto_integrator_merge: matches!(
                        integrator_merge_policy,
                        DevflowIntegratorMergePolicy::AutoWhenReady
                    ),
                    requested_stop: None,
                },
            );
        }

        if uses_legacy_automation_agent {
            run.status = DevflowRunStatus::Running;
            {
                let mut store = self.store.lock().await;
                if let Some(record) = store.runs.get_mut(&run.id) {
                    record.run = run.clone();
                }
            }

            self.send_artifact_created(context_artifact).await;
            self.send_task_status_changed(task.clone()).await;
            self.send_run_status_changed(run.clone()).await;

            let processor = self.clone();
            let task_id = task.id.clone();
            let run_id = run.id.clone();
            tokio::spawn(async move {
                processor
                    .run_external_automation_task(task_id, run_id)
                    .await;
            });

            return Ok(DevflowTaskStartResponse { task, run });
        }

        if uses_legacy_text_agent {
            run.status = DevflowRunStatus::Running;
            {
                let mut store = self.store.lock().await;
                if let Some(record) = store.runs.get_mut(&run.id) {
                    record.run = run.clone();
                }
            }

            self.send_artifact_created(context_artifact).await;
            self.send_task_status_changed(task.clone()).await;
            self.send_run_status_changed(run.clone()).await;

            let processor = self.clone();
            let task_id = task.id.clone();
            let run_id = run.id.clone();
            tokio::spawn(async move {
                processor.run_external_text_task(task_id, run_id).await;
            });

            return Ok(DevflowTaskStartResponse { task, run });
        }

        let execution_cwd = managed_worktree
            .as_ref()
            .map(|worktree| worktree.cwd_path.clone())
            .unwrap_or_else(|| task.project_id.clone());
        let config = match self.load_devflow_config(&task, &execution_cwd).await {
            Ok(config) => config,
            Err(err) => {
                self.mark_run_failed(
                    &task.id,
                    &run.id,
                    format!("failed to load config: {err}"),
                    now,
                )
                .await;
                return Err(internal_error(format!("failed to load config: {err}")));
            }
        };

        self.thread_state_manager
            .connection_initialized(internal_connection_id)
            .await;

        let new_thread = match self.thread_manager.start_thread(config).await {
            Ok(new_thread) => new_thread,
            Err(err) => {
                self.thread_state_manager
                    .remove_connection(internal_connection_id)
                    .await;
                self.mark_run_failed(
                    &task.id,
                    &run.id,
                    format!("failed to start devflow thread: {err}"),
                    now,
                )
                .await;
                return Err(thread_start_error(err));
            }
        };

        let thread_id = new_thread.thread_id;
        let thread = new_thread.thread;
        if let Err(err) = ensure_conversation_listener(
            self.listener_task_context(),
            thread_id,
            internal_connection_id,
            /*raw_events_enabled*/ false,
        )
        .await
        {
            self.thread_state_manager
                .remove_connection(internal_connection_id)
                .await;
            let _ = thread.shutdown_and_wait().await;
            self.mark_run_failed(
                &task.id,
                &run.id,
                format!("failed to attach devflow listener: {}", err.message),
                now,
            )
            .await;
            return Err(err);
        }

        run.thread_id = Some(thread_id.to_string());
        let turn_id = match thread
            .submit(Op::UserInput {
                items: vec![CoreUserInput::Text {
                    text: run.input.clone(),
                    text_elements: Vec::new(),
                }],
                environments: None,
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
            })
            .await
        {
            Ok(turn_id) => turn_id,
            Err(err) => {
                self.thread_state_manager
                    .remove_connection(internal_connection_id)
                    .await;
                let _ = thread.shutdown_and_wait().await;
                self.mark_run_failed(
                    &task.id,
                    &run.id,
                    format!("failed to start devflow turn: {err}"),
                    now,
                )
                .await;
                return Err(internal_error(format!(
                    "failed to start devflow turn: {err}"
                )));
            }
        };

        run.turn_id = Some(turn_id);
        run.status = DevflowRunStatus::Running;

        {
            let mut store = self.store.lock().await;
            if let Some(record) = store.runs.get_mut(&run.id) {
                record.run = run.clone();
                store
                    .thread_to_run
                    .insert(thread_id.to_string(), run.id.clone());
            }
            store.tasks.insert(task.id.clone(), task.clone());
        }

        self.send_artifact_created(context_artifact).await;
        if let Some(worktree) = managed_worktree {
            self.outgoing
                .send_server_notification(ServerNotification::DevflowWorktreeStatusChanged(
                    DevflowWorktreeStatusChangedNotification { worktree },
                ))
                .await;
        }
        self.send_task_status_changed(task.clone()).await;
        self.send_run_status_changed(run.clone()).await;

        Ok(DevflowTaskStartResponse { task, run })
    }

    pub(crate) async fn task_pause(
        &self,
        params: DevflowTaskPauseParams,
    ) -> Result<DevflowTaskPauseResponse, JSONRPCErrorError> {
        let (task, run) = self
            .request_task_stop(&params.id, DevflowRequestedStop::Pause)
            .await?;
        Ok(DevflowTaskPauseResponse { task, run })
    }

    pub(crate) async fn task_resume(
        &self,
        params: DevflowTaskResumeParams,
    ) -> Result<DevflowTaskResumeResponse, JSONRPCErrorError> {
        {
            let store = self.store.lock().await;
            let task = store.tasks.get(&params.id).ok_or_else(|| {
                invalid_request(format!("unknown devflow task id: {}", params.id))
            })?;
            if task.status != DevflowTaskStatus::Paused {
                return Err(invalid_request(format!(
                    "devflow task is not paused: {}",
                    params.id
                )));
            }
        }
        let DevflowTaskStartResponse { task, run } = self
            .task_start(DevflowTaskStartParams { id: params.id })
            .await?;
        Ok(DevflowTaskResumeResponse { task, run })
    }

    pub(crate) async fn task_cancel(
        &self,
        params: DevflowTaskCancelParams,
    ) -> Result<DevflowTaskCancelResponse, JSONRPCErrorError> {
        let status = {
            let store = self.store.lock().await;
            store
                .tasks
                .get(&params.id)
                .map(|task| task.status)
                .ok_or_else(|| invalid_request(format!("unknown devflow task id: {}", params.id)))?
        };
        if matches!(
            status,
            DevflowTaskStatus::ReadyForReview
                | DevflowTaskStatus::ReadyToMerge
                | DevflowTaskStatus::Failed
        ) {
            return Err(invalid_request(format!(
                "devflow task is already terminal: {}",
                params.id
            )));
        }

        let (task, run, already_notified) = if matches!(
            status,
            DevflowTaskStatus::Planned | DevflowTaskStatus::Blocked | DevflowTaskStatus::Paused
        ) {
            let (task, run) = {
                let mut store = self.store.lock().await;
                let run = store
                    .tasks
                    .get(&params.id)
                    .and_then(|task| task.run_ids.last())
                    .and_then(|run_id| store.runs.get(run_id))
                    .map(|record| record.run.clone());
                let task = store.tasks.get_mut(&params.id).ok_or_else(|| {
                    invalid_request(format!("unknown devflow task id: {}", params.id))
                })?;
                task.status = DevflowTaskStatus::Cancelled;
                task.updated_at = Utc::now().timestamp();
                (task.clone(), run)
            };
            (task, run, false)
        } else {
            let (task, run) = self
                .request_task_stop(&params.id, DevflowRequestedStop::Cancel)
                .await?;
            (task, Some(run), true)
        };
        if !already_notified {
            self.send_task_status_changed(task.clone()).await;
        }
        if !already_notified && let Some(run) = run.as_ref() {
            self.send_run_status_changed(run.clone()).await;
        }
        Ok(DevflowTaskCancelResponse { task, run })
    }

    pub(crate) async fn task_read(
        &self,
        params: DevflowTaskReadParams,
    ) -> Result<DevflowTaskReadResponse, JSONRPCErrorError> {
        let store = self.store.lock().await;
        let task =
            store.tasks.get(&params.id).cloned().ok_or_else(|| {
                invalid_request(format!("unknown devflow task id: {}", params.id))
            })?;
        Ok(DevflowTaskReadResponse { task })
    }

    pub(crate) async fn task_list(
        &self,
        params: DevflowTaskListParams,
    ) -> Result<DevflowTaskListResponse, JSONRPCErrorError> {
        let store = self.store.lock().await;
        let mut data = store
            .tasks
            .values()
            .filter(|task| {
                params
                    .project_id
                    .as_ref()
                    .is_none_or(|project_id| &task.project_id == project_id)
                    && params
                        .status
                        .as_ref()
                        .is_none_or(|status| &task.status == status)
                    && params
                        .assigned_agent_id
                        .as_ref()
                        .is_none_or(|assigned_agent_id| {
                            task.assigned_agent_id.as_ref() == Some(assigned_agent_id)
                        })
            })
            .cloned()
            .collect::<Vec<_>>();
        data.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });

        let start = params
            .cursor
            .as_deref()
            .and_then(|cursor| cursor.parse::<usize>().ok())
            .unwrap_or(0);
        let limit = params.limit.unwrap_or(100).min(500) as usize;
        let page = data
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor =
            (start + page.len() < data.len()).then(|| (start + page.len()).to_string());
        Ok(DevflowTaskListResponse {
            data: page,
            next_cursor,
        })
    }

    pub(crate) async fn task_assign(
        &self,
        params: DevflowTaskAssignParams,
    ) -> Result<DevflowTaskAssignResponse, JSONRPCErrorError> {
        let task = {
            let mut store = self.store.lock().await;
            let task = store.tasks.get_mut(&params.id).ok_or_else(|| {
                invalid_request(format!("unknown devflow task id: {}", params.id))
            })?;
            task.assigned_agent_id = params.assigned_agent_id;
            if task.trigger_source.is_none() {
                task.trigger_source = default_trigger_source(task.assigned_agent_id.as_deref());
            }
            task.updated_at = Utc::now().timestamp();
            task.clone()
        };

        self.send_task_status_changed(task.clone()).await;

        Ok(DevflowTaskAssignResponse { task })
    }

    pub(crate) async fn task_dependencies_update(
        &self,
        params: DevflowTaskDependenciesUpdateParams,
    ) -> Result<DevflowTaskDependenciesUpdateResponse, JSONRPCErrorError> {
        {
            let store = self.store.lock().await;
            validate_task_dependencies(&store, &params.id, &params.dependencies)?;
        }

        let task = {
            let mut store = self.store.lock().await;
            let task = store.tasks.get_mut(&params.id).ok_or_else(|| {
                invalid_request(format!("unknown devflow task id: {}", params.id))
            })?;
            task.dependencies = params.dependencies;
            if task.status == DevflowTaskStatus::Blocked {
                task.status = DevflowTaskStatus::Planned;
            }
            task.updated_at = Utc::now().timestamp();
            task.clone()
        };

        self.send_task_status_changed(task.clone()).await;

        Ok(DevflowTaskDependenciesUpdateResponse { task })
    }

    async fn request_task_stop(
        &self,
        task_id: &str,
        requested_stop: DevflowRequestedStop,
    ) -> Result<(DevflowTask, DevflowRun), JSONRPCErrorError> {
        let (task, run, thread_id) =
            {
                let mut store = self.store.lock().await;
                let run_id = {
                    let task = store.tasks.get(task_id).ok_or_else(|| {
                        invalid_request(format!("unknown devflow task id: {task_id}"))
                    })?;
                    if task.status != DevflowTaskStatus::Running {
                        return Err(invalid_request(format!(
                            "devflow task is not running: {task_id}"
                        )));
                    }
                    task.run_ids.last().cloned().ok_or_else(|| {
                        invalid_request(format!("devflow task has no run to stop: {task_id}"))
                    })?
                };

                let now = Utc::now().timestamp();
                let target_task_status = match requested_stop {
                    DevflowRequestedStop::Pause => DevflowTaskStatus::Paused,
                    DevflowRequestedStop::Cancel => DevflowTaskStatus::Cancelled,
                };
                let stop_reason = match requested_stop {
                    DevflowRequestedStop::Pause => "paused by devflowTask/pause",
                    DevflowRequestedStop::Cancel => "cancelled by devflowTask/cancel",
                }
                .to_string();

                {
                    let task = store.tasks.get_mut(task_id).ok_or_else(|| {
                        invalid_request(format!("unknown devflow task id: {task_id}"))
                    })?;
                    task.status = target_task_status;
                    task.updated_at = now;
                }

                {
                    let record = store.runs.get_mut(&run_id).ok_or_else(|| {
                        invalid_request(format!("unknown devflow run id: {run_id}"))
                    })?;
                    if record.requested_stop.is_some() {
                        return Err(invalid_request(format!(
                            "devflow task stop already requested: {task_id}"
                        )));
                    }
                    record.requested_stop = Some(requested_stop);
                    record.run.status = DevflowRunStatus::Cancelled;
                    record.run.completed_at = Some(now);
                    record.run.exit_reason = Some(stop_reason);
                }

                let mut cancelled_approvals = Vec::new();
                for record in store.approvals.values_mut() {
                    if record.approval.run_id == run_id
                        && record.approval.status == DevflowApprovalStatus::Pending
                    {
                        record.approval.status = DevflowApprovalStatus::Responded;
                        record.approval.responded_at = Some(now);
                        record.approval.decision = Some(DevflowApprovalDecision::Cancel);
                        cancelled_approvals.push(record.approval.clone());
                    }
                }
                for approval in cancelled_approvals {
                    store.approval_history.insert(approval.id.clone(), approval);
                }

                let task = store.tasks.get(task_id).cloned().ok_or_else(|| {
                    invalid_request(format!("unknown devflow task id: {task_id}"))
                })?;
                let run = store
                    .runs
                    .get(&run_id)
                    .map(|record| record.run.clone())
                    .ok_or_else(|| invalid_request(format!("unknown devflow run id: {run_id}")))?;
                let thread_id = store
                    .runs
                    .get(&run_id)
                    .and_then(|record| record.run.thread_id.clone());
                (task, run, thread_id)
            };

        self.send_task_status_changed(task.clone()).await;
        self.send_run_status_changed(run.clone()).await;

        if let Some(thread_id) = thread_id
            && let Ok(thread_uuid) = ThreadId::from_string(&thread_id)
            && let Ok(thread) = self.thread_manager.get_thread(thread_uuid).await
            && let Err(err) = thread.submit(Op::Interrupt).await
        {
            tracing::warn!(
                task_id,
                run_id = run.id,
                error = %err,
                "failed to interrupt devflow run after stop request"
            );
        }

        Ok((task, run))
    }

    async fn requested_stop_for_run(&self, run_id: &str) -> Option<DevflowRequestedStop> {
        let store = self.store.lock().await;
        store
            .runs
            .get(run_id)
            .and_then(|record| record.requested_stop)
    }

    async fn run_external_text_task(&self, task_id: String, run_id: String) {
        let (task, run) = {
            let store = self.store.lock().await;
            let Some(task) = store.tasks.get(&task_id).cloned() else {
                return;
            };
            let Some(run) = store.runs.get(&run_id).map(|record| record.run.clone()) else {
                return;
            };
            (task, run)
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        let prompt = match self.build_claude_task_prompt(&task, &run).await {
            Ok(prompt) => prompt,
            Err(err) => {
                self.mark_run_failed(
                    &task_id,
                    &run_id,
                    format!("failed to build Claude task prompt: {err}"),
                    Utc::now().timestamp(),
                )
                .await;
                return;
            }
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        let execution = match run_claude_report(Path::new(&task.project_id), &prompt).await {
            Ok(execution) => execution,
            Err(err) => {
                self.mark_run_failed(
                    &task_id,
                    &run_id,
                    format!("failed to run Claude adapter: {err}"),
                    Utc::now().timestamp(),
                )
                .await;
                return;
            }
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        let output = combine_external_agent_output(&execution);
        if !output.is_empty() {
            self.record_run_output_delta(
                task_id.clone(),
                run_id.clone(),
                DevflowRunOutputSource::Assistant,
                output.clone(),
            )
            .await;
        }

        if execution.exit_code.unwrap_or(1) != 0 {
            self.mark_run_failed(
                &task_id,
                &run_id,
                external_agent_failure_reason(&execution),
                Utc::now().timestamp(),
            )
            .await;
            return;
        }

        let artifact = match self
            .write_external_report_artifact(&task_id, &run_id, &output)
            .await
        {
            Ok(artifact) => artifact,
            Err(err) => {
                self.mark_run_failed(
                    &task_id,
                    &run_id,
                    format!("failed to persist Claude report artifact: {err}"),
                    Utc::now().timestamp(),
                )
                .await;
                return;
            }
        };

        self.send_artifact_created(artifact).await;
        self.finalize_ready_for_review(&run_id).await;
    }

    async fn run_external_automation_task(&self, task_id: String, run_id: String) {
        let (task, run) = {
            let store = self.store.lock().await;
            let Some(task) = store.tasks.get(&task_id).cloned() else {
                return;
            };
            let Some(run) = store.runs.get(&run_id).map(|record| record.run.clone()) else {
                return;
            };
            (task, run)
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        let (command, command_args) = match hermes_task_command(&task) {
            Ok(command) => command,
            Err(err) => {
                self.mark_run_failed(&task_id, &run_id, err, Utc::now().timestamp())
                    .await;
                return;
            }
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        let command_id = Uuid::new_v4().to_string();
        {
            let mut store = self.store.lock().await;
            if let Some(record) = store.runs.get_mut(&run_id)
                && !record.run.command_ids.contains(&command_id)
            {
                record.run.command_ids.push(command_id.clone());
            }
        }
        self.send_run_command_started(
            task_id.clone(),
            run_id.clone(),
            command_id.clone(),
            command.clone(),
            task.project_id.clone(),
        )
        .await;

        let command_arg_refs = command_args.iter().map(String::as_str).collect::<Vec<_>>();
        let execution =
            match run_hermes_command(Path::new(&task.project_id), &command_arg_refs).await {
                Ok(execution) => execution,
                Err(err) => {
                    self.mark_run_failed(
                        &task_id,
                        &run_id,
                        format!("failed to run Hermes automation task: {err}"),
                        Utc::now().timestamp(),
                    )
                    .await;
                    return;
                }
            };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }
        let output = combine_external_agent_output(&execution);

        if !output.is_empty() {
            self.record_run_output_delta(
                task_id.clone(),
                run_id.clone(),
                DevflowRunOutputSource::CommandExecution,
                output.clone(),
            )
            .await;
        }

        self.send_run_command_completed(DevflowRunCommandCompletedEvent {
            task_id: task_id.clone(),
            run_id: run_id.clone(),
            command_id,
            exit_code: execution.exit_code,
            status: if execution.exit_code == Some(0) {
                "completed".to_string()
            } else {
                "failed".to_string()
            },
            duration_ms: None,
            output_summary: (!output.is_empty())
                .then(|| truncate(&output, COMMAND_OUTPUT_SUMMARY_LIMIT)),
        })
        .await;

        if execution.exit_code.unwrap_or(1) != 0 {
            self.mark_run_failed(
                &task_id,
                &run_id,
                external_agent_failure_reason(&execution),
                Utc::now().timestamp(),
            )
            .await;
            return;
        }

        let artifact = match self
            .write_external_report_artifact(&task_id, &run_id, &output)
            .await
        {
            Ok(artifact) => artifact,
            Err(err) => {
                self.mark_run_failed(
                    &task_id,
                    &run_id,
                    format!("failed to persist Hermes automation artifact: {err}"),
                    Utc::now().timestamp(),
                )
                .await;
                return;
            }
        };
        self.send_artifact_created(artifact).await;
        self.finalize_ready_for_review(&run.id).await;
    }

    async fn build_claude_task_prompt(
        &self,
        task: &DevflowTask,
        run: &DevflowRun,
    ) -> Result<String, String> {
        let (context_pack_artifact, dependency_records) = {
            let store = self.store.lock().await;
            let context_pack_artifact = task
                .context_pack_id
                .as_ref()
                .and_then(|artifact_id| store.artifacts.get(artifact_id))
                .cloned();
            let dependency_records = task
                .dependencies
                .iter()
                .filter_map(|dependency_id| {
                    let dependency_task = store.tasks.get(dependency_id)?.clone();
                    let dependency_artifacts = dependency_task
                        .artifact_ids
                        .iter()
                        .filter_map(|artifact_id| store.artifacts.get(artifact_id).cloned())
                        .collect::<Vec<_>>();
                    Some((dependency_task, dependency_artifacts))
                })
                .collect::<Vec<_>>();
            (context_pack_artifact, dependency_records)
        };

        let context_pack = if let Some(artifact) = context_pack_artifact {
            fs::read_to_string(&artifact.path)
                .await
                .unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        };

        let mut dependency_sections = Vec::new();
        for (dependency_task, dependency_artifacts) in dependency_records {
            let mut section = format!(
                "## Dependency Task\n- Task ID: {}\n- Title: {}\n- Kind: {:?}\n- Status: {:?}\n",
                dependency_task.id,
                dependency_task.title,
                dependency_task.kind,
                dependency_task.status
            );
            for artifact in dependency_artifacts {
                if !should_include_dependency_artifact(task.kind, artifact.kind) {
                    continue;
                }
                let body = fs::read_to_string(&artifact.path)
                    .await
                    .unwrap_or_else(|_| artifact.summary.clone());
                section.push_str(&format!(
                    "\n### {} ({:?})\n```text\n{}\n```\n",
                    artifact.title,
                    artifact.kind,
                    truncate(&body, 12_000)
                ));
            }
            dependency_sections.push(section);
        }

        let task_instructions = match task.kind {
            DevflowTaskKind::Review => {
                "Write a concise Markdown review report. Focus on correctness risks, regressions, missing verification, and concrete follow-up checks. If the diff looks safe, say that clearly. If you find issues, write one bullet per finding in this machine-readable shape: `- [P1] Short title | file=path/to/file.rs | line=123 | status=open | resolution=required fix or reason | followUp=optional follow-up`. Use status `resolved`, `waived`, or `follow-up` only when the evidence clearly supports it."
            }
            DevflowTaskKind::Report => {
                "Write a concise Markdown report that explains the requested outcome, current evidence, and any recommended next steps."
            }
            DevflowTaskKind::Diagnostic => {
                "Write a concise Markdown diagnostic report with evidence and a final `Root cause:` line. If the cause is unknown, write `Root cause: unknown` so the release gate remains blocked."
            }
            DevflowTaskKind::Implementation | DevflowTaskKind::Automation => {
                "Write a concise Markdown report that summarizes the current task context."
            }
        };

        let dependency_block = if dependency_sections.is_empty() {
            "## Dependency Context\nNo dependency task artifacts were provided.\n".to_string()
        } else {
            dependency_sections.join("\n")
        };

        Ok(format!(
            "You are Claude Code acting as the Yuqei Devflow {} agent.\n\nRead-only mode: do not edit files, run destructive actions, or attempt complex implementation work. Use the provided context and produce only the final Markdown report.\n\nProject root: {}\nTask ID: {}\nRun ID: {}\nTask title: {}\nObjective: {}\n\n{}\n\n## Context Pack\n```json\n{}\n```\n\n{}",
            match task.kind {
                DevflowTaskKind::Review => "review",
                DevflowTaskKind::Report => "report",
                DevflowTaskKind::Implementation => "implementation",
                DevflowTaskKind::Diagnostic => "diagnostic",
                DevflowTaskKind::Automation => "automation",
            },
            task.project_id,
            task.id,
            run.id,
            task.title,
            task.objective,
            task_instructions,
            context_pack,
            dependency_block
        ))
    }

    async fn observe_outgoing_messages(
        processor: DevflowRequestProcessor,
        mut receiver: broadcast::Receiver<ObservedOutgoingMessage>,
    ) {
        loop {
            match receiver.recv().await {
                Ok(observed) => processor.handle_observed_message(observed).await,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(
                        skipped,
                        "devflow observer lagged behind outgoing notification stream"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }

    async fn handle_observed_message(&self, observed: ObservedOutgoingMessage) {
        match observed.message {
            OutgoingMessage::AppServerNotification(notification) => match notification {
                ServerNotification::AgentMessageDelta(payload) => {
                    self.handle_output_delta(
                        &payload.thread_id,
                        DevflowRunOutputSource::Assistant,
                        payload.delta,
                    )
                    .await;
                }
                ServerNotification::CommandExecutionOutputDelta(payload) => {
                    self.handle_output_delta(
                        &payload.thread_id,
                        DevflowRunOutputSource::CommandExecution,
                        payload.delta,
                    )
                    .await;
                }
                ServerNotification::ItemStarted(payload) => {
                    self.handle_item_started(payload).await;
                }
                ServerNotification::ItemCompleted(payload) => {
                    self.handle_item_completed(payload).await;
                }
                ServerNotification::TurnDiffUpdated(payload) => {
                    self.handle_turn_diff_updated(payload).await;
                }
                ServerNotification::TurnCompleted(payload) => {
                    self.handle_turn_completed(payload).await;
                }
                _ => {}
            },
            OutgoingMessage::Request(request) => {
                self.handle_server_request(request).await;
            }
            OutgoingMessage::Response(_) | OutgoingMessage::Error(_) => {}
        }
    }

    async fn handle_server_request(&self, request: ServerRequest) {
        let maybe_record = match request.clone() {
            ServerRequest::CommandExecutionRequestApproval { request_id, params } => {
                let thread_id = params.thread_id.clone();
                self.build_approval_record(DevflowApprovalProjection {
                    request_id: request_id.clone(),
                    thread_id,
                    kind: DevflowApprovalKind::CommandExecution,
                    reason: params.reason.clone(),
                    command: params.command.clone(),
                    cwd: params.cwd.as_ref().map(|cwd| cwd.display().to_string()),
                    file_paths: Vec::new(),
                    requested_permissions: None,
                    request: PendingDevflowApprovalRequest::CommandExecution { request_id, params },
                })
                .await
            }
            ServerRequest::FileChangeRequestApproval { request_id, params } => {
                let thread_id = params.thread_id.clone();
                self.build_approval_record(DevflowApprovalProjection {
                    request_id: request_id.clone(),
                    thread_id,
                    kind: DevflowApprovalKind::FileChange,
                    reason: params.reason.clone(),
                    command: None,
                    cwd: params
                        .grant_root
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    file_paths: Vec::new(),
                    requested_permissions: None,
                    request: PendingDevflowApprovalRequest::FileChange { request_id, params },
                })
                .await
            }
            ServerRequest::PermissionsRequestApproval { request_id, params } => {
                let thread_id = params.thread_id.clone();
                self.build_approval_record(DevflowApprovalProjection {
                    request_id: request_id.clone(),
                    thread_id,
                    kind: DevflowApprovalKind::Permissions,
                    reason: params.reason.clone(),
                    command: None,
                    cwd: Some(params.cwd.display().to_string()),
                    file_paths: Vec::new(),
                    requested_permissions: Some(params.permissions.clone()),
                    request: PendingDevflowApprovalRequest::Permissions { request_id, params },
                })
                .await
            }
            _ => None,
        };

        if let Some(record) = maybe_record {
            if let Some(decision) = self
                .matching_approval_grant_decision(&record.approval)
                .await
            {
                match approval_response_value(&record.request, decision, None) {
                    Ok(result) => {
                        if let Some(request_id) = pending_approval_request_id(&record.request) {
                            self.outgoing
                                .notify_client_response(request_id.clone(), result)
                                .await;
                        }
                        let mut record = record;
                        mark_approval_responded(&mut record.approval, decision);
                        let approval = record.approval.clone();
                        let mut store = self.store.lock().await;
                        store
                            .approval_history
                            .insert(approval.id.clone(), approval.clone());
                        store.approvals.insert(approval.id, record);
                    }
                    Err(err) => {
                        tracing::warn!(
                            error = %err.message,
                            "failed to auto-respond to cached devflow approval"
                        );
                    }
                }
                self.persist_store_best_effort().await;
                return;
            }

            let approval = record.approval.clone();
            {
                let mut store = self.store.lock().await;
                store
                    .approval_history
                    .insert(approval.id.clone(), approval.clone());
                store.approvals.insert(approval.id.clone(), record);
            }
            self.send_approval_requested(approval).await;
        }
    }

    async fn build_approval_record(
        &self,
        projection: DevflowApprovalProjection,
    ) -> Option<DevflowApprovalRecord> {
        let DevflowApprovalProjection {
            request_id,
            thread_id,
            kind,
            reason,
            command,
            cwd,
            file_paths,
            requested_permissions,
            request,
        } = projection;
        let (task_id, run_id) = self.run_keys_for_thread(&thread_id).await?;
        let project_id = {
            let store = self.store.lock().await;
            let record = store.runs.get(&run_id)?;
            if record.requested_stop.is_some() {
                return None;
            }
            store
                .tasks
                .get(&task_id)
                .map(|task| task.project_id.clone())
                .unwrap_or_else(|| record.project_root.clone())
        };
        let (turn_id, item_id) = match &request {
            PendingDevflowApprovalRequest::CommandExecution { params, .. } => {
                (params.turn_id.clone(), params.item_id.clone())
            }
            PendingDevflowApprovalRequest::FileChange { params, .. } => {
                (params.turn_id.clone(), params.item_id.clone())
            }
            PendingDevflowApprovalRequest::Permissions { params, .. } => {
                (params.turn_id.clone(), params.item_id.clone())
            }
            PendingDevflowApprovalRequest::QualityGateWaive { .. }
            | PendingDevflowApprovalRequest::ArtifactDelivery { .. } => {
                unreachable!("synthetic devflow approvals are not built through projection")
            }
        };
        let approval = DevflowApproval {
            id: Uuid::new_v4().to_string(),
            project_id,
            task_id,
            run_id,
            quality_gate_id: None,
            request_id: request_id_to_string(&request_id),
            thread_id,
            turn_id,
            item_id,
            kind,
            status: DevflowApprovalStatus::Pending,
            reason,
            command,
            cwd,
            file_paths,
            requested_permissions,
            responded_at: None,
            decision: None,
            created_at: Utc::now().timestamp(),
        };
        Some(DevflowApprovalRecord { approval, request })
    }

    async fn matching_approval_grant_decision(
        &self,
        approval: &DevflowApproval,
    ) -> Option<DevflowApprovalDecision> {
        let store = self.store.lock().await;
        store
            .approval_grants
            .iter()
            .find(|grant| devflow_approval_grant_matches(grant, approval))
            .map(|grant| grant.decision)
    }

    async fn handle_output_delta(
        &self,
        thread_id: &str,
        source: DevflowRunOutputSource,
        delta: String,
    ) {
        let Some((task_id, run_id)) = self.run_keys_for_thread(thread_id).await else {
            return;
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        self.record_run_output_delta(task_id, run_id, source, delta)
            .await;
    }

    async fn record_run_output_delta(
        &self,
        task_id: String,
        run_id: String,
        source: DevflowRunOutputSource,
        delta: String,
    ) {
        if let Some(artifact) = self
            .archive_output_delta_if_needed(&task_id, &run_id, &delta)
            .await
        {
            self.send_artifact_created(artifact).await;
        }

        self.send_run_output_delta(task_id, run_id, source, delta)
            .await;
    }

    async fn archive_output_delta_if_needed(
        &self,
        task_id: &str,
        run_id: &str,
        delta: &str,
    ) -> Option<DevflowArtifact> {
        let (artifact, content, is_new) = {
            let mut store = self.store.lock().await;
            let task = store.tasks.get(task_id).cloned()?;
            let (new_artifact, existing_artifact_id, content) = {
                let record = store.runs.get_mut(run_id)?;
                let previous_summary = record.run.stream_summary.clone().unwrap_or_default();
                let previous_summary_chars = previous_summary.chars().count();
                append_stream_summary(&mut record.run.stream_summary, delta);

                if let Some(artifact_id) = record.output_archive_artifact_id.clone() {
                    (None, Some(artifact_id), delta.to_string())
                } else if previous_summary_chars + delta.chars().count() > OUTPUT_ARCHIVE_THRESHOLD
                {
                    let artifact_id = Uuid::new_v4().to_string();
                    let artifact = DevflowArtifact {
                        id: artifact_id.clone(),
                        task_id: task.id.clone(),
                        run_id: run_id.to_string(),
                        kind: DevflowArtifactKind::OutputArchive,
                        title: format!("Output archive for {}", task.title),
                        path: artifact_file_path(
                            &record.project_root,
                            run_id,
                            &format!("output-archive-{artifact_id}"),
                            "log",
                        )
                        .display()
                        .to_string(),
                        mime_type: "text/plain".to_string(),
                        summary: format!(
                            "Full output archive created after the run exceeded {OUTPUT_ARCHIVE_THRESHOLD} characters"
                        ),
                        created_at: Utc::now().timestamp(),
                    };
                    record.output_archive_artifact_id = Some(artifact.id.clone());
                    if !record.run.artifact_ids.contains(&artifact.id) {
                        record.run.artifact_ids.push(artifact.id.clone());
                    }
                    (Some(artifact), None, format!("{previous_summary}{delta}"))
                } else {
                    return None;
                }
            };

            if let Some(artifact) = new_artifact {
                if let Some(task) = store.tasks.get_mut(task_id)
                    && !task.artifact_ids.contains(&artifact.id)
                {
                    task.artifact_ids.push(artifact.id.clone());
                    task.updated_at = artifact.created_at;
                }
                store
                    .artifacts
                    .insert(artifact.id.clone(), artifact.clone());
                (artifact, content, true)
            } else {
                let artifact_id = existing_artifact_id?;
                let artifact = store.artifacts.get(&artifact_id).cloned()?;
                (artifact, content, false)
            }
        };

        if let Err(err) = append_artifact_file(Path::new(&artifact.path), &content).await {
            tracing::warn!(
                error = %err,
                task_id,
                run_id,
                "failed to append devflow output archive"
            );
            return None;
        }

        is_new.then_some(artifact)
    }

    async fn handle_item_started(&self, payload: ItemStartedNotification) {
        let ThreadItem::CommandExecution {
            id, command, cwd, ..
        } = payload.item
        else {
            return;
        };
        let Some((task_id, run_id)) = self.run_keys_for_thread(&payload.thread_id).await else {
            return;
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        {
            let mut store = self.store.lock().await;
            if let Some(record) = store.runs.get_mut(&run_id)
                && !record.run.command_ids.contains(&id)
            {
                record.run.command_ids.push(id.clone());
            }
        }

        self.send_run_command_started(task_id, run_id, id, command, cwd.display().to_string())
            .await;
    }

    async fn handle_item_completed(&self, payload: ItemCompletedNotification) {
        match payload.item {
            ThreadItem::ExitedReviewMode {
                review,
                review_output,
                ..
            } => {
                self.handle_review_item_completed(&payload.thread_id, review, review_output)
                    .await;
            }
            ThreadItem::CommandExecution {
                id,
                status,
                exit_code,
                duration_ms,
                aggregated_output,
                ..
            } => {
                let Some((task_id, run_id)) = self.run_keys_for_thread(&payload.thread_id).await
                else {
                    return;
                };
                if self.requested_stop_for_run(&run_id).await.is_some() {
                    return;
                }

                if let Some(aggregated_output) = aggregated_output.clone() {
                    self.send_run_output_delta(
                        task_id.clone(),
                        run_id.clone(),
                        DevflowRunOutputSource::CommandExecution,
                        aggregated_output,
                    )
                    .await;
                }

                self.send_run_command_completed(DevflowRunCommandCompletedEvent {
                    task_id,
                    run_id,
                    command_id: id,
                    exit_code,
                    status: format!("{status:?}").to_ascii_lowercase(),
                    duration_ms,
                    output_summary: aggregated_output
                        .as_deref()
                        .map(|output| truncate(output, COMMAND_OUTPUT_SUMMARY_LIMIT)),
                })
                .await;
            }
            _ => {}
        }
    }

    async fn handle_turn_diff_updated(&self, payload: TurnDiffUpdatedNotification) {
        let Some((task_id, run_id)) = self.run_keys_for_thread(&payload.thread_id).await else {
            return;
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }

        let diff = payload.diff;
        let artifact = match self
            .write_or_update_diff_artifact(&task_id, &run_id, &diff)
            .await
        {
            Ok(artifact) => artifact,
            Err(err) => {
                tracing::warn!(error = %err, "failed to persist devflow diff artifact");
                return;
            }
        };
        let worktree_context = {
            let store = self.store.lock().await;
            store
                .tasks
                .get(&task_id)
                .map(|task| (task.project_id.clone(), task.worktree_id.clone()))
        };

        if let Some((project_id, worktree_id)) = worktree_context {
            self.outgoing
                .send_server_notification(ServerNotification::DevflowRunDiffUpdated(
                    DevflowRunDiffUpdatedNotification {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        run_id: run_id.clone(),
                        artifact_id: artifact.id.clone(),
                        diff: diff.clone(),
                    },
                ))
                .await;
            self.outgoing
                .send_server_notification(ServerNotification::DevflowWorktreeDiffUpdated(
                    DevflowWorktreeDiffUpdatedNotification {
                        project_id,
                        task_id,
                        run_id,
                        worktree_id,
                        artifact_id: artifact.id,
                        diff,
                    },
                ))
                .await;
        }
    }

    async fn handle_turn_completed(&self, payload: TurnCompletedNotification) {
        let Some(run_id) = ({
            let store = self.store.lock().await;
            store.thread_to_run.get(&payload.thread_id).cloned()
        }) else {
            return;
        };

        let (task_id, original_turn_id, task_kind, review_requested, requested_stop) = {
            let store = self.store.lock().await;
            let Some(record) = store.runs.get(&run_id) else {
                return;
            };
            let Some(task) = store.tasks.get(&record.run.task_id) else {
                return;
            };
            (
                task.id.clone(),
                record.run.turn_id.clone(),
                task.kind,
                record.review_requested,
                record.requested_stop,
            )
        };

        if requested_stop.is_none()
            && payload.turn.status == codex_app_server_protocol::TurnStatus::Completed
        {
            if original_turn_id.as_deref() == Some(payload.turn.id.as_str())
                && task_kind == codex_app_server_protocol::DevflowTaskKind::Implementation
            {
                if let Err(err) = self
                    .start_quality_gate(DevflowQualityGateStartRequest {
                        task_id: task_id.clone(),
                        kind: DevflowQualityGateKind::TargetedTest,
                        command_override: None,
                        mode: DevflowQualityGateStartMode::Automatic,
                    })
                    .await
                {
                    self.mark_run_failed(
                        &task_id,
                        &run_id,
                        format!("failed to start quality gate: {}", err.message),
                        Utc::now().timestamp(),
                    )
                    .await;
                }
                return;
            }
            if review_requested && original_turn_id.as_deref() != Some(payload.turn.id.as_str()) {
                self.finalize_ready_for_review(&run_id).await;
                return;
            }
        }

        let (task, run, summary_artifact, internal_connection_id) = {
            let mut store = self.store.lock().await;
            let Some(existing_record) = store.runs.get(&run_id) else {
                return;
            };
            let task_id = existing_record.run.task_id.clone();
            let stream_summary = existing_record
                .run
                .stream_summary
                .clone()
                .unwrap_or_default();
            let task_title = store
                .tasks
                .get(&task_id)
                .map(|task| task.title.clone())
                .unwrap_or_default();
            let summary_artifact_id = existing_record
                .summary_artifact_id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            let project_root = existing_record.project_root.clone();
            let internal_connection_id = existing_record.internal_connection_id;

            let Some(task) = store.tasks.get_mut(&task_id) else {
                return;
            };
            let completed_at = Utc::now().timestamp();
            let turn_status = payload.turn.status.clone();
            let (task_status, run_status, exit_reason) = match (requested_stop, turn_status.clone())
            {
                (Some(DevflowRequestedStop::Pause), _) => (
                    DevflowTaskStatus::Paused,
                    DevflowRunStatus::Cancelled,
                    Some("paused by devflowTask/pause".to_string()),
                ),
                (Some(DevflowRequestedStop::Cancel), _) => (
                    DevflowTaskStatus::Cancelled,
                    DevflowRunStatus::Cancelled,
                    Some("cancelled by devflowTask/cancel".to_string()),
                ),
                (None, codex_app_server_protocol::TurnStatus::Completed) => {
                    let (task_status, run_status, _) = completed_task_status(task, false);
                    (task_status, run_status, None)
                }
                (
                    None,
                    codex_app_server_protocol::TurnStatus::Interrupted
                    | codex_app_server_protocol::TurnStatus::Failed,
                ) => (
                    DevflowTaskStatus::Failed,
                    DevflowRunStatus::Failed,
                    payload
                        .turn
                        .error
                        .as_ref()
                        .map(|error| error.message.clone())
                        .or_else(|| Some(format!("{turn_status:?}").to_ascii_lowercase())),
                ),
                (None, codex_app_server_protocol::TurnStatus::InProgress) => {
                    return;
                }
            };
            let summary_status = match requested_stop {
                Some(DevflowRequestedStop::Pause) => "paused".to_string(),
                Some(DevflowRequestedStop::Cancel) => "cancelled".to_string(),
                None => format!("{:?}", payload.turn.status).to_ascii_lowercase(),
            };
            let root_cause_state =
                task_requires_root_cause(task).then(|| build_root_cause_state(&stream_summary));
            let summary_title = if root_cause_state.is_some() {
                format!("Root cause summary for {task_title}")
            } else {
                format!("Run summary for {task_title}")
            };
            let summary = root_cause_state
                .as_ref()
                .map(root_cause_artifact_summary)
                .unwrap_or_else(|| format!("{task_title} finished with {summary_status}"));
            let summary_artifact = DevflowArtifact {
                id: summary_artifact_id,
                task_id: task_id.clone(),
                run_id: run_id.clone(),
                kind: DevflowArtifactKind::RunSummary,
                title: summary_title,
                path: artifact_file_path(&project_root, &run_id, "summary", "md")
                    .display()
                    .to_string(),
                mime_type: "text/markdown".to_string(),
                summary,
                created_at: completed_at,
            };
            task.status = task_status;
            task.updated_at = completed_at;
            if !task.artifact_ids.contains(&summary_artifact.id) {
                task.artifact_ids.push(summary_artifact.id.clone());
            }
            let task_clone = task.clone();
            let run_clone = {
                let Some(record) = store.runs.get_mut(&run_id) else {
                    return;
                };
                record.run.status = run_status;
                record.run.completed_at = Some(completed_at);
                record.run.exit_reason = exit_reason;
                record.summary_artifact_id = Some(summary_artifact.id.clone());
                if !record.run.artifact_ids.contains(&summary_artifact.id) {
                    record.run.artifact_ids.push(summary_artifact.id.clone());
                }
                record.run.clone()
            };
            store
                .artifacts
                .insert(summary_artifact.id.clone(), summary_artifact.clone());
            store.tasks.insert(task_id, task_clone.clone());
            (
                task_clone,
                run_clone,
                summary_artifact,
                internal_connection_id,
            )
        };

        if let Err(err) = self
            .write_run_summary_artifact(&task, &run, &summary_artifact)
            .await
        {
            tracing::warn!(error = %err, "failed to persist devflow run summary artifact");
        }

        self.send_artifact_created(summary_artifact).await;
        self.send_task_status_changed(task.clone()).await;
        self.send_run_status_changed(run.clone()).await;

        self.thread_state_manager
            .remove_connection(internal_connection_id)
            .await;
    }

    async fn handle_review_item_completed(
        &self,
        thread_id: &str,
        review: String,
        review_output: Option<ReviewOutput>,
    ) {
        let Some((task_id, run_id)) = self.run_keys_for_thread(thread_id).await else {
            return;
        };
        if self.requested_stop_for_run(&run_id).await.is_some() {
            return;
        }
        let artifact = match self
            .write_review_artifact(&task_id, &run_id, &review, review_output.as_ref())
            .await
        {
            Ok(artifact) => artifact,
            Err(err) => {
                tracing::warn!(error = %err, "failed to persist devflow review artifact");
                return;
            }
        };
        self.send_artifact_created(artifact).await;
    }

    async fn run_keys_for_thread(&self, thread_id: &str) -> Option<(String, String)> {
        let store = self.store.lock().await;
        let run_id = store.thread_to_run.get(thread_id)?.clone();
        let task_id = store.runs.get(&run_id)?.run.task_id.clone();
        Some((task_id, run_id))
    }

    async fn project_id_for_run_or_task(&self, task_id: &str, run_id: &str) -> Option<String> {
        let store = self.store.lock().await;
        store
            .tasks
            .get(task_id)
            .map(|task| task.project_id.clone())
            .or_else(|| {
                store
                    .runs
                    .get(run_id)
                    .map(|record| record.project_root.clone())
            })
    }

    async fn persist_store_best_effort(&self) {
        let snapshot = {
            let store = self.store.lock().await;
            devflow_store_snapshot(&store)
        };
        if let Err(err) =
            save_devflow_store_snapshot(self.config.codex_home.as_path(), &snapshot).await
        {
            *self.store_snapshot_persist_error.lock().await = Some(err.clone());
            tracing::warn!(
                error = %err,
                "failed to persist devflow store snapshot"
            );
        } else {
            *self.store_snapshot_persist_error.lock().await = None;
        }
    }

    async fn send_task_status_changed(&self, task: DevflowTask) {
        self.persist_store_best_effort().await;
        self.outgoing
            .send_server_notification(ServerNotification::DevflowTaskStatusChanged(
                DevflowTaskStatusChangedNotification { task },
            ))
            .await;
    }

    async fn send_approval_requested(&self, approval: DevflowApproval) {
        self.persist_store_best_effort().await;
        self.outgoing
            .send_server_notification(ServerNotification::DevflowApprovalRequested(
                DevflowApprovalRequestedNotification { approval },
            ))
            .await;
    }

    async fn send_artifact_created(&self, artifact: DevflowArtifact) {
        self.persist_store_best_effort().await;
        let Some(project_id) = self
            .project_id_for_run_or_task(&artifact.task_id, &artifact.run_id)
            .await
        else {
            tracing::warn!(
                task_id = artifact.task_id,
                run_id = artifact.run_id,
                "skipping devflow artifact notification because project id is unavailable"
            );
            return;
        };
        self.outgoing
            .send_server_notification(ServerNotification::DevflowArtifactCreated(
                DevflowArtifactCreatedNotification {
                    project_id,
                    artifact,
                },
            ))
            .await;
    }

    async fn send_agent_status_changed(&self, agent: DevflowAgent) {
        self.outgoing
            .send_server_notification(ServerNotification::DevflowAgentStatusChanged(
                DevflowAgentStatusChangedNotification { agent },
            ))
            .await;
    }

    async fn send_quality_gate_completed(&self, gate: DevflowQualityGate) {
        self.persist_store_best_effort().await;
        let Some(project_id) = self
            .project_id_for_run_or_task(&gate.task_id, &gate.run_id)
            .await
        else {
            tracing::warn!(
                task_id = gate.task_id,
                run_id = gate.run_id,
                "skipping devflow quality gate notification because project id is unavailable"
            );
            return;
        };
        self.outgoing
            .send_server_notification(ServerNotification::DevflowQualityGateCompleted(
                DevflowQualityGateCompletedNotification { project_id, gate },
            ))
            .await;
    }

    async fn send_watchdog_alert_created(&self, alert: DevflowWatchdogAlert) {
        self.persist_store_best_effort().await;
        let project_id = if let Some(project_id) = alert.project_id.clone() {
            Some(project_id)
        } else if let Some(task_id) = alert.task_id.as_deref() {
            self.project_id_for_run_or_task(task_id, alert.run_id.as_deref().unwrap_or_default())
                .await
        } else {
            None
        };
        let Some(project_id) = project_id else {
            tracing::warn!(
                alert_id = alert.id,
                "skipping devflow watchdog alert notification because project id is unavailable"
            );
            return;
        };
        self.outgoing
            .send_server_notification(ServerNotification::DevflowWatchdogAlertCreated(
                DevflowWatchdogAlertCreatedNotification { project_id, alert },
            ))
            .await;
    }

    async fn send_run_status_changed(&self, run: DevflowRun) {
        self.persist_store_best_effort().await;
        let task_id = run.task_id.clone();
        let Some(project_id) = self.project_id_for_run_or_task(&task_id, &run.id).await else {
            tracing::warn!(
                task_id,
                run_id = run.id,
                "skipping devflow run status notification because project id is unavailable"
            );
            return;
        };
        self.outgoing
            .send_server_notification(ServerNotification::DevflowRunStatusChanged(
                DevflowRunStatusChangedNotification {
                    project_id,
                    task_id,
                    run,
                },
            ))
            .await;
    }

    async fn send_run_output_delta(
        &self,
        task_id: String,
        run_id: String,
        source: DevflowRunOutputSource,
        delta: String,
    ) {
        self.persist_store_best_effort().await;
        let Some(project_id) = self.project_id_for_run_or_task(&task_id, &run_id).await else {
            tracing::warn!(
                task_id,
                run_id,
                "skipping devflow output notification because project id is unavailable"
            );
            return;
        };
        self.outgoing
            .send_server_notification(ServerNotification::DevflowRunOutputDelta(
                DevflowRunOutputDeltaNotification {
                    project_id,
                    task_id,
                    run_id,
                    source,
                    delta,
                },
            ))
            .await;
    }

    async fn send_run_command_started(
        &self,
        task_id: String,
        run_id: String,
        command_id: String,
        command: String,
        cwd: String,
    ) {
        self.persist_store_best_effort().await;
        let Some(project_id) = self.project_id_for_run_or_task(&task_id, &run_id).await else {
            tracing::warn!(
                task_id,
                run_id,
                command_id,
                "skipping devflow command-start notification because project id is unavailable"
            );
            return;
        };
        self.outgoing
            .send_server_notification(ServerNotification::DevflowRunCommandStarted(
                DevflowRunCommandStartedNotification {
                    project_id,
                    task_id,
                    run_id,
                    command_id,
                    command,
                    cwd,
                },
            ))
            .await;
    }

    async fn send_run_command_completed(&self, event: DevflowRunCommandCompletedEvent) {
        self.persist_store_best_effort().await;
        let Some(project_id) = self
            .project_id_for_run_or_task(&event.task_id, &event.run_id)
            .await
        else {
            tracing::warn!(
                task_id = event.task_id,
                run_id = event.run_id,
                command_id = event.command_id,
                "skipping devflow command-completed notification because project id is unavailable"
            );
            return;
        };
        self.outgoing
            .send_server_notification(ServerNotification::DevflowRunCommandCompleted(
                DevflowRunCommandCompletedNotification {
                    project_id,
                    task_id: event.task_id,
                    run_id: event.run_id,
                    command_id: event.command_id,
                    exit_code: event.exit_code,
                    status: event.status,
                    duration_ms: event.duration_ms,
                    output_summary: event.output_summary,
                },
            ))
            .await;
    }

    async fn write_or_update_diff_artifact(
        &self,
        task_id: &str,
        run_id: &str,
        diff: &str,
    ) -> std::io::Result<DevflowArtifact> {
        let (artifact, is_new) = {
            let mut store = self.store.lock().await;
            let (project_root, artifact_id, is_new) = {
                let Some(record) = store.runs.get(run_id) else {
                    return Err(std::io::Error::other("run disappeared while handling diff"));
                };
                (
                    record.project_root.clone(),
                    record
                        .diff_artifact_id
                        .clone()
                        .unwrap_or_else(|| Uuid::new_v4().to_string()),
                    record.diff_artifact_id.is_none(),
                )
            };
            let Some(task_title) = store.tasks.get(task_id).map(|task| task.title.clone()) else {
                return Err(std::io::Error::other(
                    "task disappeared while handling diff",
                ));
            };
            let artifact = DevflowArtifact {
                id: artifact_id,
                task_id: task_id.to_string(),
                run_id: run_id.to_string(),
                kind: DevflowArtifactKind::Diff,
                title: format!("Diff for {task_title}"),
                path: artifact_file_path(&project_root, run_id, "diff", "patch")
                    .display()
                    .to_string(),
                mime_type: "text/x-diff".to_string(),
                summary: truncate(diff, DIFF_SUMMARY_LIMIT),
                created_at: Utc::now().timestamp(),
            };
            let Some(record) = store.runs.get_mut(run_id) else {
                return Err(std::io::Error::other(
                    "run disappeared before diff artifact update",
                ));
            };
            record.diff_artifact_id = Some(artifact.id.clone());
            if !record.run.artifact_ids.contains(&artifact.id) {
                record.run.artifact_ids.push(artifact.id.clone());
            }
            let Some(task) = store.tasks.get_mut(task_id) else {
                return Err(std::io::Error::other(
                    "task disappeared before diff artifact update",
                ));
            };
            if !task.artifact_ids.contains(&artifact.id) {
                task.artifact_ids.push(artifact.id.clone());
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
            (artifact, is_new)
        };

        write_artifact_file(Path::new(&artifact.path), diff).await?;
        self.persist_store_best_effort().await;

        if is_new {
            self.send_artifact_created(artifact.clone()).await;
        }

        Ok(artifact)
    }

    async fn write_context_pack_artifact(
        &self,
        task: &DevflowTask,
        run: &DevflowRun,
    ) -> Result<DevflowArtifact, JSONRPCErrorError> {
        let path = artifact_file_path(&task.project_id, &run.id, "context-pack", "json");
        let memory_path = project_memory_path(&task.project_id);
        let project_memory_summary = read_project_memory_summary(&memory_path).await?;
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "taskId": task.id,
            "runId": run.id,
            "projectRoot": task.project_id,
            "title": task.title,
            "objective": task.objective,
            "triggerSource": task.trigger_source,
            "kind": task.kind,
            "riskLevel": task.risk_level,
            "assignedAgentId": run.agent_id,
            "input": run.input,
            "projectMemory": {
                "path": memory_path.display().to_string(),
                "summary": project_memory_summary,
            },
        }))
        .map_err(|err| {
            internal_error(format!("failed to serialize devflow context pack: {err}"))
        })?;

        write_artifact_file(&path, &content).await.map_err(|err| {
            internal_error(format!("failed to write devflow context pack: {err}"))
        })?;

        Ok(DevflowArtifact {
            id: Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            run_id: run.id.clone(),
            kind: DevflowArtifactKind::ContextPack,
            title: format!("Context pack for {}", task.title),
            path: path.display().to_string(),
            mime_type: "application/json".to_string(),
            summary: "Task objective, project root, trigger source, project memory, and initial run prompt"
                .to_string(),
            created_at: Utc::now().timestamp(),
        })
    }

    async fn write_quality_gate_artifact(
        &self,
        gate: &DevflowQualityGate,
        task: &DevflowTask,
        output: &str,
    ) -> std::io::Result<DevflowArtifact> {
        let artifact = DevflowArtifact {
            id: Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            run_id: gate.run_id.clone(),
            kind: DevflowArtifactKind::QualityGateOutput,
            title: format!("Quality gate for {}", task.title),
            path: artifact_file_path(&task.project_id, &gate.id, "quality-gate", "txt")
                .display()
                .to_string(),
            mime_type: "text/plain".to_string(),
            summary: truncate(output, COMMAND_OUTPUT_SUMMARY_LIMIT),
            created_at: Utc::now().timestamp(),
        };
        write_artifact_file(Path::new(&artifact.path), output).await?;
        Ok(artifact)
    }

    async fn record_worktree_merge_outcome(
        &self,
        outcome: &WorktreeMergeOutcome,
    ) -> Result<(DevflowTask, DevflowArtifact), JSONRPCErrorError> {
        let now = Utc::now().timestamp();
        let status = if outcome.merged {
            DevflowTaskStatus::ReadyToMerge
        } else {
            DevflowTaskStatus::Blocked
        };
        let merge_status = if outcome.merged { "merged" } else { "blocked" };
        let next_action = if outcome.merged {
            "ready_for_release_prep"
        } else {
            "resolve_conflicts_before_retrying_integrator_merge"
        };

        let (task, artifact, content) = {
            let mut store = self.store.lock().await;
            let task_view = store
                .tasks
                .get(&outcome.worktree.task_id)
                .cloned()
                .ok_or_else(|| {
                    invalid_request(format!(
                        "unknown devflow task id for worktree: {}",
                        outcome.worktree.task_id
                    ))
                })?;
            let run_id = task_view
                .run_ids
                .last()
                .cloned()
                .unwrap_or_else(|| format!("worktree-merge-{}", Uuid::new_v4()));
            let artifact_id = Uuid::new_v4().to_string();
            let summary = if outcome.merged {
                format!("Integrator merged {} without conflicts", task_view.title)
            } else {
                format!(
                    "Integrator blocked {} with {} merge conflicts",
                    task_view.title,
                    outcome.conflicts.len()
                )
            };
            let artifact = DevflowArtifact {
                id: artifact_id.clone(),
                task_id: task_view.id.clone(),
                run_id: run_id.clone(),
                kind: DevflowArtifactKind::Report,
                title: format!("Integrator merge report for {}", task_view.title),
                path: artifact_file_path(
                    &task_view.project_id,
                    &run_id,
                    &format!("integrator-merge-{artifact_id}"),
                    "json",
                )
                .display()
                .to_string(),
                mime_type: "application/json".to_string(),
                summary,
                created_at: now,
            };
            let content = serde_json::to_string_pretty(&serde_json::json!({
                "schemaVersion": 1,
                "runner": "codex-devflow-integrator",
                "status": merge_status,
                "task": {
                    "id": task_view.id,
                    "title": task_view.title,
                    "status": status,
                },
                "worktree": &outcome.worktree,
                "merged": outcome.merged,
                "conflicts": &outcome.conflicts,
                "diff": &outcome.diff,
                "diffSummary": truncate(&outcome.diff, DIFF_SUMMARY_LIMIT),
                "nextAction": next_action,
                "policy": {
                    "mergeStrategy": "git apply --check --3way --index before applying the worktree diff to the primary worktree.",
                    "conflictHandling": "If git reports conflicts, the task is marked blocked and the primary worktree is restored before returning.",
                    "artifactFormat": "application/json; schemaVersion=1; includes worktree metadata, conflicts, full diff, and nextAction.",
                },
                "createdAt": now,
            }))
            .map_err(|err| {
                internal_error(format!("failed to serialize devflow integrator report: {err}"))
            })?;

            let task = {
                let task = store
                    .tasks
                    .get_mut(&outcome.worktree.task_id)
                    .ok_or_else(|| {
                        invalid_request(format!(
                            "unknown devflow task id for worktree: {}",
                            outcome.worktree.task_id
                        ))
                    })?;
                task.status = status;
                task.updated_at = now;
                if !task.artifact_ids.contains(&artifact.id) {
                    task.artifact_ids.push(artifact.id.clone());
                }
                task.clone()
            };
            if let Some(record) = store.runs.get_mut(&run_id)
                && !record.run.artifact_ids.contains(&artifact.id)
            {
                record.run.artifact_ids.push(artifact.id.clone());
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
            (task, artifact, content)
        };

        write_artifact_file(Path::new(&artifact.path), &content)
            .await
            .map_err(|err| internal_error(format!("failed to write integrator report: {err}")))?;
        Ok((task, artifact))
    }

    async fn write_integrator_dispatch_artifact(
        &self,
        project_id: Option<&str>,
        limit: usize,
        started: &[DevflowTaskDispatchStarted],
        skipped: &[DevflowTaskDispatchSkipped],
        blocked: &[DevflowTaskDispatchBlocked],
    ) -> Result<Option<DevflowArtifact>, JSONRPCErrorError> {
        let Some(anchor_task_id) = started
            .first()
            .map(|response| response.task.id.clone())
            .or_else(|| blocked.first().map(|item| item.task_id.clone()))
            .or_else(|| skipped.first().map(|item| item.task_id.clone()))
        else {
            return Ok(None);
        };

        let queue = self.watchdog_queue_snapshot(project_id).await;
        let now = Utc::now().timestamp();
        let (task, run_id) = {
            let store = self.store.lock().await;
            let task = store.tasks.get(&anchor_task_id).cloned().ok_or_else(|| {
                invalid_request(format!("unknown devflow task id: {anchor_task_id}"))
            })?;
            let run_id = started
                .first()
                .filter(|response| response.task.id == anchor_task_id)
                .map(|response| response.run.id.clone())
                .or_else(|| task.run_ids.last().cloned())
                .unwrap_or_else(|| format!("dispatch-{}", Uuid::new_v4()));
            (task, run_id)
        };

        let status = if !blocked.is_empty() {
            "blocked"
        } else if !started.is_empty() {
            "dispatched"
        } else {
            "idle"
        };
        let next_action = if !blocked.is_empty() {
            "resolve_blocked_or_conflicting_tasks_before_integrator_merge"
        } else if !started.is_empty() {
            "watch_started_runs_and_dispatch_followup_ready_tasks"
        } else {
            "no_ready_implementation_tasks_to_dispatch"
        };
        let artifact_id = Uuid::new_v4().to_string();
        let artifact = DevflowArtifact {
            id: artifact_id.clone(),
            task_id: task.id.clone(),
            run_id: run_id.clone(),
            kind: DevflowArtifactKind::Report,
            title: format!("Integrator dispatch report for {}", task.title),
            path: artifact_file_path(
                &task.project_id,
                &run_id,
                &format!("integrator-dispatch-{artifact_id}"),
                "json",
            )
            .display()
            .to_string(),
            mime_type: "application/json".to_string(),
            summary: format!(
                "Integrator dispatch started {}, blocked {}, skipped {}",
                started.len(),
                blocked.len(),
                skipped.len()
            ),
            created_at: now,
        };
        let started_tasks = started
            .iter()
            .map(|response| {
                serde_json::json!({
                    "taskId": &response.task.id,
                    "runId": &response.run.id,
                    "title": &response.task.title,
                    "status": response.task.status,
                    "agentId": &response.run.agent_id,
                })
            })
            .collect::<Vec<_>>();
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "schemaVersion": 1,
            "runner": "codex-devflow-integrator",
            "status": status,
            "projectId": &task.project_id,
            "scopeProjectId": project_id,
            "counts": {
                "started": started.len(),
                "blocked": blocked.len(),
                "skipped": skipped.len(),
                "dispatchLimit": limit,
            },
            "started": started_tasks,
            "blocked": blocked,
            "skipped": skipped,
            "integratorQueue": queue,
            "nextAction": next_action,
            "policy": {
                "readyRule": "Only planned implementation tasks with all dependencies resolved are started automatically.",
                "blockedRule": "Planned implementation tasks with unresolved dependencies are marked blocked; already-blocked tasks stay blocked for manual recovery or conflict resolution.",
                "dispatchScope": "Dispatch is fail-closed and requires projectId or explicit taskIds.",
                "autoMergeRule": "Runs started by dispatch auto-merge their managed worktree after the implementation task is ready_to_merge with diff, required quality-gate artifacts, and review artifacts all present.",
                "maxDispatchLimit": DEVFLOW_DISPATCH_MAX_LIMIT,
            },
            "createdAt": now,
        }))
        .map_err(|err| {
            internal_error(format!("failed to serialize devflow dispatch report: {err}"))
        })?;

        {
            let mut store = self.store.lock().await;
            if let Some(task) = store.tasks.get_mut(&artifact.task_id)
                && !task.artifact_ids.contains(&artifact.id)
            {
                task.artifact_ids.push(artifact.id.clone());
            }
            if let Some(record) = store.runs.get_mut(&run_id)
                && !record.run.artifact_ids.contains(&artifact.id)
            {
                record.run.artifact_ids.push(artifact.id.clone());
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
        }

        write_artifact_file(Path::new(&artifact.path), &content)
            .await
            .map_err(|err| {
                internal_error(format!("failed to write devflow dispatch report: {err}"))
            })?;
        Ok(Some(artifact))
    }

    async fn write_review_artifact(
        &self,
        task_id: &str,
        run_id: &str,
        review: &str,
        review_output: Option<&ReviewOutput>,
    ) -> std::io::Result<DevflowArtifact> {
        let finding_state = build_review_finding_state(review, review_output);
        let content = render_review_artifact(review, &finding_state);
        let summary = review_artifact_summary(&finding_state);
        let artifact = {
            let mut store = self.store.lock().await;
            let Some(task) = store.tasks.get(task_id).cloned() else {
                return Err(std::io::Error::other(
                    "task disappeared while writing review artifact",
                ));
            };
            let Some(record_view) = store.runs.get(run_id).cloned() else {
                return Err(std::io::Error::other(
                    "run disappeared while writing review artifact",
                ));
            };
            let artifact = DevflowArtifact {
                id: record_view
                    .review_artifact_id
                    .unwrap_or_else(|| Uuid::new_v4().to_string()),
                task_id: task_id.to_string(),
                run_id: run_id.to_string(),
                kind: DevflowArtifactKind::ReviewReport,
                title: format!("Review report for {}", task.title),
                path: artifact_file_path(&task.project_id, run_id, "review", "md")
                    .display()
                    .to_string(),
                mime_type: "text/markdown".to_string(),
                summary,
                created_at: Utc::now().timestamp(),
            };
            {
                let Some(record) = store.runs.get_mut(run_id) else {
                    return Err(std::io::Error::other(
                        "run disappeared while updating review artifact",
                    ));
                };
                record.review_completed = true;
                record.review_artifact_id = Some(artifact.id.clone());
                if !record.run.artifact_ids.contains(&artifact.id) {
                    record.run.artifact_ids.push(artifact.id.clone());
                }
            }
            {
                let Some(task) = store.tasks.get_mut(task_id) else {
                    return Err(std::io::Error::other(
                        "task disappeared while updating review artifact",
                    ));
                };
                if !task.artifact_ids.contains(&artifact.id) {
                    task.artifact_ids.push(artifact.id.clone());
                }
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
            artifact
        };
        write_artifact_file(Path::new(&artifact.path), &content).await?;
        Ok(artifact)
    }

    async fn write_external_report_artifact(
        &self,
        task_id: &str,
        run_id: &str,
        output: &str,
    ) -> std::io::Result<DevflowArtifact> {
        let (artifact, content) = {
            let mut store = self.store.lock().await;
            let Some(task) = store.tasks.get(task_id).cloned() else {
                return Err(std::io::Error::other(
                    "task disappeared while writing Claude report artifact",
                ));
            };
            let (kind, suffix, title) = match task.kind {
                DevflowTaskKind::Review => (
                    DevflowArtifactKind::ReviewReport,
                    "claude-review",
                    format!("Claude review report for {}", task.title),
                ),
                DevflowTaskKind::Diagnostic => (
                    DevflowArtifactKind::Report,
                    "root-cause",
                    format!("Root cause report for {}", task.title),
                ),
                DevflowTaskKind::Implementation
                | DevflowTaskKind::Report
                | DevflowTaskKind::Automation => (
                    DevflowArtifactKind::Report,
                    "claude-report",
                    format!("Claude report for {}", task.title),
                ),
            };
            let (content, summary) = if kind == DevflowArtifactKind::ReviewReport {
                let finding_state = build_review_finding_state(output, None);
                (
                    render_review_artifact(output, &finding_state),
                    review_artifact_summary(&finding_state),
                )
            } else if task_requires_root_cause(&task) {
                let root_cause_state = build_root_cause_state(output);
                (
                    render_root_cause_artifact(output, &root_cause_state),
                    root_cause_artifact_summary(&root_cause_state),
                )
            } else {
                (output.to_string(), truncate(output, DIFF_SUMMARY_LIMIT))
            };
            let artifact = DevflowArtifact {
                id: Uuid::new_v4().to_string(),
                task_id: task_id.to_string(),
                run_id: run_id.to_string(),
                kind,
                title,
                path: artifact_file_path(&task.project_id, run_id, suffix, "md")
                    .display()
                    .to_string(),
                mime_type: "text/markdown".to_string(),
                summary,
                created_at: Utc::now().timestamp(),
            };
            {
                let Some(record) = store.runs.get_mut(run_id) else {
                    return Err(std::io::Error::other(
                        "run disappeared while writing Claude report artifact",
                    ));
                };
                if kind == DevflowArtifactKind::ReviewReport {
                    record.review_artifact_id = Some(artifact.id.clone());
                    record.review_completed = true;
                }
                if !record.run.artifact_ids.contains(&artifact.id) {
                    record.run.artifact_ids.push(artifact.id.clone());
                }
            }
            {
                let Some(task) = store.tasks.get_mut(task_id) else {
                    return Err(std::io::Error::other(
                        "task disappeared while updating Claude report artifact",
                    ));
                };
                if !task.artifact_ids.contains(&artifact.id) {
                    task.artifact_ids.push(artifact.id.clone());
                }
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
            (artifact, content)
        };
        write_artifact_file(Path::new(&artifact.path), &content).await?;
        Ok(artifact)
    }

    async fn finalize_ready_for_review(&self, run_id: &str) {
        let (task, run, summary_artifact, internal_connection_id, auto_integrator_merge) = {
            let mut store = self.store.lock().await;
            let Some(record_view) = store.runs.get(run_id).cloned() else {
                return;
            };
            if record_view.requested_stop.is_some() {
                return;
            }
            let task_id = record_view.run.task_id.clone();
            let Some(task_view) = store.tasks.get(&task_id).cloned() else {
                return;
            };
            let completed_at = Utc::now().timestamp();
            let stream_summary = record_view.run.stream_summary.clone().unwrap_or_default();
            let review_ready = record_view.review_completed
                && record_view
                    .review_artifact_id
                    .as_deref()
                    .and_then(|artifact_id| store.artifacts.get(artifact_id))
                    .is_some_and(review_artifact_all_findings_addressed);
            let (task_status, run_status, status_label) =
                completed_task_status(&task_view, review_ready);
            let root_cause_state = task_requires_root_cause(&task_view)
                .then(|| build_root_cause_state(&stream_summary));
            let summary_title = if root_cause_state.is_some() {
                format!("Root cause summary for {}", task_view.title)
            } else {
                format!("Run summary for {}", task_view.title)
            };
            let summary = root_cause_state
                .as_ref()
                .map(root_cause_artifact_summary)
                .unwrap_or_else(|| format!("{} finished with {status_label}", task_view.title));
            let summary_artifact = DevflowArtifact {
                id: record_view
                    .summary_artifact_id
                    .clone()
                    .unwrap_or_else(|| Uuid::new_v4().to_string()),
                task_id: task_id.clone(),
                run_id: run_id.to_string(),
                kind: DevflowArtifactKind::RunSummary,
                title: summary_title,
                path: artifact_file_path(&task_view.project_id, run_id, "summary", "md")
                    .display()
                    .to_string(),
                mime_type: "text/markdown".to_string(),
                summary,
                created_at: completed_at,
            };
            {
                let Some(task) = store.tasks.get_mut(&task_id) else {
                    return;
                };
                task.status = task_status;
                task.updated_at = completed_at;
                if !task.artifact_ids.contains(&summary_artifact.id) {
                    task.artifact_ids.push(summary_artifact.id.clone());
                }
            }
            {
                let Some(record) = store.runs.get_mut(run_id) else {
                    return;
                };
                record.run.status = run_status;
                record.run.completed_at = Some(completed_at);
                record.summary_artifact_id = Some(summary_artifact.id.clone());
                if !record.run.artifact_ids.contains(&summary_artifact.id) {
                    record.run.artifact_ids.push(summary_artifact.id.clone());
                }
            }
            let Some(task) = store.tasks.get(&task_id).cloned() else {
                return;
            };
            let Some(run) = store.runs.get(run_id).map(|record| record.run.clone()) else {
                return;
            };
            let internal_connection_id = record_view.internal_connection_id;
            store
                .artifacts
                .insert(summary_artifact.id.clone(), summary_artifact.clone());
            (
                task,
                run,
                summary_artifact,
                internal_connection_id,
                record_view.auto_integrator_merge,
            )
        };

        if let Err(err) = self
            .write_run_summary_artifact(&task, &run, &summary_artifact)
            .await
        {
            tracing::warn!(error = %err, "failed to persist devflow run summary artifact");
        }

        self.send_artifact_created(summary_artifact).await;
        self.send_task_status_changed(task.clone()).await;
        self.send_run_status_changed(run.clone()).await;
        self.thread_state_manager
            .remove_connection(internal_connection_id)
            .await;
        if auto_integrator_merge {
            self.auto_merge_ready_worktree(&task, &run).await;
        }
    }

    async fn auto_merge_ready_worktree(&self, task: &DevflowTask, run: &DevflowRun) {
        if task.kind != DevflowTaskKind::Implementation {
            return;
        }
        let Some(worktree_id) = task.worktree_id.clone() else {
            return;
        };
        let has_merge_evidence = {
            let store = self.store.lock().await;
            let Some(current_task) = store.tasks.get(&task.id) else {
                return;
            };
            let Some(record) = store.runs.get(&run.id) else {
                return;
            };
            let gate_ready = record
                .quality_gate_id
                .as_deref()
                .and_then(|gate_id| store.quality_gates.get(gate_id))
                .is_some_and(|record| {
                    matches!(
                        record.gate.status,
                        DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Waived
                    ) && record.gate.artifact_id.is_some()
                });
            let review_ready = record
                .review_artifact_id
                .as_deref()
                .and_then(|artifact_id| store.artifacts.get(artifact_id))
                .is_some_and(review_artifact_all_findings_addressed);
            current_task.status == DevflowTaskStatus::ReadyToMerge
                && record.run.status == DevflowRunStatus::ReadyToMerge
                && current_task.worktree_id.as_deref() == Some(worktree_id.as_str())
                && record.diff_artifact_id.is_some()
                && record.review_completed
                && review_ready
                && gate_ready
        };
        if !has_merge_evidence {
            return;
        }

        let outcome =
            match merge_managed_worktree(self.config.codex_home.as_path(), &worktree_id).await {
                Ok(outcome) => outcome,
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        task_id = task.id,
                        run_id = run.id,
                        worktree_id,
                        "failed to auto-merge ready Devflow worktree"
                    );
                    self.record_watchdog_alert(
                        DevflowWatchdogStatus::NoProgress,
                        DevflowWatchdogAlertSeverity::Warning,
                        Some(task.project_id.clone()),
                        Some(task.id.clone()),
                        Some(run.id.clone()),
                        format!("Integrator auto-merge could not run: {err}"),
                    )
                    .await;
                    return;
                }
            };
        match self.record_worktree_merge_outcome(&outcome).await {
            Ok((merged_task, artifact)) => {
                self.send_task_status_changed(merged_task).await;
                self.send_artifact_created(artifact).await;
            }
            Err(err) => {
                let message = err.message;
                tracing::warn!(
                    error = %message,
                    task_id = task.id,
                    run_id = run.id,
                    "failed to record auto-merge outcome"
                );
                self.record_watchdog_alert(
                    DevflowWatchdogStatus::NoProgress,
                    DevflowWatchdogAlertSeverity::Warning,
                    Some(task.project_id.clone()),
                    Some(task.id.clone()),
                    Some(run.id.clone()),
                    format!("Integrator auto-merge outcome could not be recorded: {message}"),
                )
                .await;
            }
        }
    }

    async fn ensure_managed_worktree(
        &self,
        task_id: &str,
    ) -> Result<DevflowWorktree, JSONRPCErrorError> {
        let mut task = {
            let store = self.store.lock().await;
            store
                .tasks
                .get(task_id)
                .cloned()
                .ok_or_else(|| invalid_request(format!("unknown devflow task id: {task_id}")))?
        };

        if task.kind != codex_app_server_protocol::DevflowTaskKind::Implementation {
            return Err(invalid_request(format!(
                "devflow worktrees are only available for implementation tasks: {task_id}"
            )));
        }

        let worktree = if let Some(worktree_id) = task.worktree_id.as_deref() {
            read_managed_worktree(self.config.codex_home.as_path(), worktree_id)
                .await
                .map_err(invalid_request)?
        } else {
            let worktree = create_managed_worktree(self.config.codex_home.as_path(), &task)
                .await
                .map_err(invalid_request)?;
            task.worktree_id = Some(worktree.id.clone());
            task.updated_at = Utc::now().timestamp();
            {
                let mut store = self.store.lock().await;
                store.tasks.insert(task.id.clone(), task.clone());
            }
            self.send_task_status_changed(task.clone()).await;
            worktree
        };

        Ok(worktree)
    }

    pub(super) async fn resolve_capability_pack_target(
        &self,
        task_id: Option<&str>,
        project_root: Option<&str>,
    ) -> Result<DevflowCapabilityPackTarget, JSONRPCErrorError> {
        if let Some(task_id) = task_id {
            let task =
                {
                    let store = self.store.lock().await;
                    store.tasks.get(task_id).cloned().ok_or_else(|| {
                        invalid_request(format!("unknown devflow task id: {task_id}"))
                    })?
                };
            let run_id = task
                .run_ids
                .last()
                .cloned()
                .unwrap_or_else(|| format!("capability-{}", Uuid::new_v4()));
            let mut worktree_id = task.worktree_id.clone();
            let cwd_path = if task.kind
                == codex_app_server_protocol::DevflowTaskKind::Implementation
            {
                let worktree = self.ensure_managed_worktree(&task.id).await?;
                worktree_id = Some(worktree.id.clone());
                PathBuf::from(worktree.cwd_path)
            } else if let Some(worktree_id) = task.worktree_id.as_deref() {
                let worktree = read_managed_worktree(self.config.codex_home.as_path(), worktree_id)
                    .await
                    .map_err(invalid_request)?;
                PathBuf::from(worktree.cwd_path)
            } else {
                PathBuf::from(&task.project_id)
            };
            return Ok(DevflowCapabilityPackTarget {
                task_id: task.id,
                run_id,
                project_root: task.project_id,
                cwd_path,
                worktree_id,
            });
        }

        let Some(project_root) = project_root else {
            return Err(invalid_request(
                "task_id or project_root is required for capability execution".to_string(),
            ));
        };
        let project_root_path = PathBuf::from(project_root);
        if !project_root_path.is_absolute() {
            return Err(invalid_request(format!(
                "project_root must be absolute for capability execution: {project_root}"
            )));
        }
        if !project_root_path.exists() {
            return Err(invalid_request(format!(
                "project_root does not exist for capability execution: {project_root}"
            )));
        }
        Ok(DevflowCapabilityPackTarget {
            task_id: format!("standalone-capability-{}", Uuid::new_v4()),
            run_id: format!("capability-{}", Uuid::new_v4()),
            project_root: project_root.to_string(),
            cwd_path: project_root_path,
            worktree_id: None,
        })
    }

    pub(super) async fn record_policy_pack_application(
        &self,
        pack: &DevflowPolicyPack,
        task_id: Option<&str>,
        requested_risk_level: Option<DevflowTaskRiskLevel>,
        mut diagnostics: Vec<String>,
    ) -> Result<DevflowPolicyPackApplication, JSONRPCErrorError> {
        let task = if let Some(task_id) = task_id {
            let store = self.store.lock().await;
            store.tasks.get(task_id).cloned()
        } else {
            None
        };
        if let Some(task_id) = task_id
            && task.is_none()
        {
            diagnostics.push(format!(
                "policy pack artifact was not persisted because task {task_id} is not known"
            ));
        }

        let effective_risk_level =
            requested_risk_level.or(task.as_ref().map(|task| task.risk_level));
        let required_artifacts = policy_pack_required_artifacts(
            task.as_ref(),
            effective_risk_level,
            task_requires_root_cause,
        );
        let Some(task) = task else {
            return Ok(DevflowPolicyPackApplication {
                required_artifacts,
                diagnostics,
                artifact: None,
            });
        };

        let artifact_id = Uuid::new_v4().to_string();
        let run_id = task
            .run_ids
            .last()
            .cloned()
            .unwrap_or_else(|| format!("policy-pack-{artifact_id}"));
        let now = Utc::now().timestamp();
        let summary = format!(
            "Applied {} policy pack; required artifacts: {}",
            pack.id,
            required_artifacts.join(", ")
        );
        let artifact = DevflowArtifact {
            id: artifact_id.clone(),
            task_id: task.id.clone(),
            run_id: run_id.clone(),
            kind: DevflowArtifactKind::Report,
            title: format!("Policy pack application for {}", task.title),
            path: artifact_file_path(&task.project_id, &run_id, "policy-pack", "json")
                .display()
                .to_string(),
            mime_type: "application/json".to_string(),
            summary,
            created_at: now,
        };
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "schemaVersion": 1,
            "runner": "codex-devflow-policy-pack",
            "status": "applied",
            "pack": pack,
            "task": {
                "id": task.id,
                "title": task.title,
                "kind": task.kind,
                "riskLevel": task.risk_level,
                "status": task.status,
            },
            "requestedRiskLevel": requested_risk_level,
            "effectiveRiskLevel": effective_risk_level,
            "requiredArtifacts": &required_artifacts,
            "diagnostics": &diagnostics,
            "policy": {
                "writingPlans": "medium/high-risk implementation tasks require a planner report before start/dispatch",
                "worktreeIsolation": "implementation tasks execute in managed worktrees by default",
                "systematicDebugging": "diagnostic and bug-like tasks require identified root-cause evidence",
                "verificationBeforeCompletion": "implementation tasks require passed or waived quality-gate evidence with a persisted artifact; high-risk tasks require integration-test evidence and snapshot-sensitive tasks require snapshot evidence",
                "requestingCodeReview": "review artifacts must include finding-state metadata and no open findings",
                "finishBranch": "release prep fail-closes on missing policy evidence, store recovery errors, and unresolved merge/review/verification blockers"
            },
            "createdAt": now,
        }))
        .map_err(|err| internal_error(format!("failed to serialize policy pack artifact: {err}")))?;

        write_artifact_file(Path::new(&artifact.path), &content)
            .await
            .map_err(|err| {
                internal_error(format!("failed to write policy pack artifact: {err}"))
            })?;

        {
            let mut store = self.store.lock().await;
            if let Some(task) = store.tasks.get_mut(&artifact.task_id) {
                if !task.artifact_ids.contains(&artifact.id) {
                    task.artifact_ids.push(artifact.id.clone());
                }
                task.updated_at = artifact.created_at;
            }
            if let Some(record) = store.runs.get_mut(&artifact.run_id)
                && !record.run.artifact_ids.contains(&artifact.id)
            {
                record.run.artifact_ids.push(artifact.id.clone());
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
        }

        self.send_artifact_created(artifact.clone()).await;
        Ok(DevflowPolicyPackApplication {
            required_artifacts,
            diagnostics,
            artifact: Some(artifact),
        })
    }

    pub(super) async fn write_capability_pack_artifact(
        &self,
        target: &DevflowCapabilityPackTarget,
        capability: &str,
        content: &str,
        summary: String,
    ) -> Result<DevflowArtifact, JSONRPCErrorError> {
        let artifact = DevflowArtifact {
            id: Uuid::new_v4().to_string(),
            task_id: target.task_id.clone(),
            run_id: target.run_id.clone(),
            kind: DevflowArtifactKind::Report,
            title: format!("gstack {capability} capability report"),
            path: artifact_file_path(&target.project_root, &target.run_id, capability, "json")
                .display()
                .to_string(),
            mime_type: "application/json".to_string(),
            summary,
            created_at: Utc::now().timestamp(),
        };
        write_artifact_file(Path::new(&artifact.path), content)
            .await
            .map_err(|err| internal_error(format!("failed to write capability artifact: {err}")))?;

        {
            let mut store = self.store.lock().await;
            if let Some(task) = store.tasks.get_mut(&artifact.task_id) {
                if !task.artifact_ids.contains(&artifact.id) {
                    task.artifact_ids.push(artifact.id.clone());
                }
                task.updated_at = artifact.created_at;
            }
            if let Some(record) = store.runs.get_mut(&artifact.run_id)
                && !record.run.artifact_ids.contains(&artifact.id)
            {
                record.run.artifact_ids.push(artifact.id.clone());
            }
            store
                .artifacts
                .insert(artifact.id.clone(), artifact.clone());
        }

        self.send_artifact_created(artifact.clone()).await;
        Ok(artifact)
    }

    pub(super) async fn record_watchdog_alert(
        &self,
        status: DevflowWatchdogStatus,
        severity: DevflowWatchdogAlertSeverity,
        project_id: Option<String>,
        task_id: Option<String>,
        run_id: Option<String>,
        message: String,
    ) -> DevflowWatchdogAlert {
        let alert = DevflowWatchdogAlert {
            id: Uuid::new_v4().to_string(),
            status,
            severity,
            project_id,
            task_id,
            run_id,
            message,
            created_at: Utc::now().timestamp(),
        };
        {
            let mut store = self.store.lock().await;
            store.watchdog_alerts.push(alert.clone());
        }
        self.send_watchdog_alert_created(alert.clone()).await;
        alert
    }

    pub(super) async fn watchdog_alert_snapshot(&self) -> Vec<DevflowWatchdogAlert> {
        let mut alerts = {
            let store = self.store.lock().await;
            store.watchdog_alerts.clone()
        };
        if let Some(error) = self.store_snapshot_persist_error.lock().await.as_deref() {
            alerts.push(store_snapshot_persist_watchdog_alert(
                error,
                Utc::now().timestamp(),
            ));
        }
        alerts
    }

    pub(super) async fn watchdog_queue_snapshot(
        &self,
        project_id: Option<&str>,
    ) -> DevflowWatchdogQueueSnapshot {
        let store_snapshot_persist_error = self.store_snapshot_persist_error.lock().await.clone();
        let store = self.store.lock().await;
        let mut alerts = store
            .watchdog_alerts
            .iter()
            .filter(|alert| {
                project_id.is_none_or(|project_id| {
                    alert.project_id.is_none() || alert.project_id.as_deref() == Some(project_id)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if let Some(error) = store_snapshot_persist_error.as_deref() {
            alerts.push(store_snapshot_persist_watchdog_alert(
                error,
                Utc::now().timestamp(),
            ));
        }
        alerts.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut running = Vec::new();
        let mut blocked = Vec::new();
        let mut tasks = store
            .tasks
            .values()
            .filter(|task| project_id.is_none_or(|project_id| task.project_id == project_id))
            .cloned()
            .collect::<Vec<_>>();
        tasks.sort_by(|a, b| {
            a.updated_at
                .cmp(&b.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        for task in &tasks {
            let latest_run = task
                .run_ids
                .last()
                .and_then(|run_id| store.runs.get(run_id))
                .map(|record| record.run.clone());
            if task.status == DevflowTaskStatus::Running
                || latest_run.as_ref().is_some_and(|run| {
                    matches!(
                        run.status,
                        DevflowRunStatus::Queued | DevflowRunStatus::Running
                    )
                })
            {
                running.push(watchdog_queue_item_for_task(
                    task,
                    latest_run.as_ref(),
                    "task or latest run is currently active".to_string(),
                ));
            }
            if task.status == DevflowTaskStatus::Blocked {
                blocked.push(watchdog_queue_item_for_task(
                    task,
                    latest_run.as_ref(),
                    "task is blocked on dependencies, approval, or recovery".to_string(),
                ));
            }
        }

        let no_progress = alerts
            .iter()
            .filter(|alert| alert.status == DevflowWatchdogStatus::NoProgress)
            .map(|alert| watchdog_queue_item_for_alert(&store, alert))
            .collect::<Vec<_>>();
        let timed_out = alerts
            .iter()
            .filter(|alert| alert.status == DevflowWatchdogStatus::TimedOut)
            .map(|alert| watchdog_queue_item_for_alert(&store, alert))
            .collect::<Vec<_>>();
        let recovering = alerts
            .iter()
            .filter(|alert| alert.status == DevflowWatchdogStatus::Recovering)
            .map(|alert| watchdog_queue_item_for_alert(&store, alert))
            .collect::<Vec<_>>();
        let counts = DevflowWatchdogQueueCounts {
            running: running.len(),
            no_progress: no_progress.len(),
            timed_out: timed_out.len(),
            recovering: recovering.len(),
            blocked: blocked.len(),
            alerts: alerts.len(),
        };
        let status = watchdog_status_for_queue_snapshot(&alerts, !running.is_empty());
        DevflowWatchdogQueueSnapshot {
            status,
            counts,
            running,
            no_progress,
            timed_out,
            recovering,
            blocked,
            alerts,
            checked_at: Utc::now().timestamp(),
        }
    }

    pub(super) async fn record_capability_pack_quality_gate(
        &self,
        target: &DevflowCapabilityPackTarget,
        outcome: DevflowCapabilityPackGateOutcome,
        artifact: &DevflowArtifact,
    ) -> Option<DevflowQualityGate> {
        let DevflowCapabilityPackGateOutcome {
            kind,
            capability,
            status,
            command,
            exit_code,
            duration_ms,
            summary,
        } = outcome;
        match status {
            DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Failed => {}
            DevflowQualityGateStatus::Queued
            | DevflowQualityGateStatus::Running
            | DevflowQualityGateStatus::Waived => return None,
        }
        let argv = command
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if argv.is_empty() {
            return None;
        }

        let now = Utc::now().timestamp();
        let gate = DevflowQualityGate {
            id: Uuid::new_v4().to_string(),
            task_id: target.task_id.clone(),
            run_id: target.run_id.clone(),
            kind,
            status,
            command: command.clone(),
            cwd: target.cwd_path.display().to_string(),
            exit_code,
            duration_ms,
            summary: Some(summary.clone()),
            artifact_id: Some(artifact.id.clone()),
            waived_reason: None,
            created_at: now,
            updated_at: now,
        };

        let (task_to_notify, run_to_notify) = {
            let mut store = self.store.lock().await;
            let task = store.tasks.get(&target.task_id).cloned()?;
            let run_exists = store.runs.contains_key(&target.run_id);
            let run_to_notify = if run_exists {
                None
            } else {
                let run_status = match status {
                    DevflowQualityGateStatus::Passed => DevflowRunStatus::ReadyForReview,
                    DevflowQualityGateStatus::Failed => DevflowRunStatus::Failed,
                    DevflowQualityGateStatus::Queued
                    | DevflowQualityGateStatus::Running
                    | DevflowQualityGateStatus::Waived => return None,
                };
                let run = DevflowRun {
                    id: target.run_id.clone(),
                    task_id: task.id.clone(),
                    agent_id: default_agent_id(task.kind).to_string(),
                    thread_id: None,
                    turn_id: None,
                    status: run_status,
                    started_at: now,
                    completed_at: Some(now),
                    input: format!("devflowCapabilityPack/run gstack-engineering {capability}"),
                    stream_summary: Some(summary.clone()),
                    command_ids: Vec::new(),
                    artifact_ids: vec![artifact.id.clone()],
                    exit_reason: Some(summary.clone()),
                };
                store.runs.insert(
                    run.id.clone(),
                    DevflowRunRecord {
                        run: run.clone(),
                        project_root: task.project_id,
                        internal_connection_id: ConnectionId(
                            self.next_internal_connection_id
                                .fetch_add(1, Ordering::Relaxed),
                        ),
                        diff_artifact_id: None,
                        summary_artifact_id: None,
                        output_archive_artifact_id: None,
                        review_artifact_id: None,
                        quality_gate_id: Some(gate.id.clone()),
                        review_requested: false,
                        review_completed: false,
                        auto_repair_attempt: 0,
                        auto_integrator_merge: false,
                        requested_stop: None,
                    },
                );
                Some(run)
            };

            let mut task_to_notify = None;
            if let Some(task_entry) = store.tasks.get_mut(&target.task_id) {
                if !task_entry.run_ids.contains(&target.run_id) {
                    task_entry.run_ids.push(target.run_id.clone());
                }
                if !task_entry.artifact_ids.contains(&artifact.id) {
                    task_entry.artifact_ids.push(artifact.id.clone());
                }
                if !run_exists && status == DevflowQualityGateStatus::Failed {
                    task_entry.status = DevflowTaskStatus::Failed;
                    task_entry.updated_at = now;
                    task_to_notify = Some(task_entry.clone());
                }
            }
            if let Some(record) = store.runs.get_mut(&target.run_id) {
                record.quality_gate_id = Some(gate.id.clone());
                if !record.run.artifact_ids.contains(&artifact.id) {
                    record.run.artifact_ids.push(artifact.id.clone());
                }
            }
            store.quality_gates.insert(
                gate.id.clone(),
                DevflowQualityGateRecord {
                    gate: gate.clone(),
                    command: GateCommand { command, argv },
                },
            );
            (task_to_notify, run_to_notify)
        };

        if let Some(task) = task_to_notify {
            self.send_task_status_changed(task).await;
        }
        if let Some(run) = run_to_notify {
            self.send_run_status_changed(run).await;
        }
        self.send_quality_gate_completed(gate.clone()).await;
        self.persist_store_best_effort().await;
        Some(gate)
    }

    async fn write_run_summary_artifact(
        &self,
        task: &DevflowTask,
        run: &DevflowRun,
        artifact: &DevflowArtifact,
    ) -> std::io::Result<()> {
        let stream_summary = run.stream_summary.clone().unwrap_or_default();
        let body = if task_requires_root_cause(task) {
            let root_cause_state = build_root_cause_state(&stream_summary);
            render_root_cause_artifact(&stream_summary, &root_cause_state)
        } else {
            format!("## Stream summary\n\n{stream_summary}\n")
        };
        let content = format!(
            "# {}\n\n- Task ID: {}\n- Run ID: {}\n- Status: {:?}\n- Thread ID: {}\n- Turn ID: {}\n- Exit reason: {}\n\n{}",
            artifact.title,
            task.id,
            run.id,
            run.status,
            run.thread_id.clone().unwrap_or_default(),
            run.turn_id.clone().unwrap_or_default(),
            run.exit_reason
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            body
        );
        write_artifact_file(Path::new(&artifact.path), &content).await
    }

    async fn mark_run_failed(&self, task_id: &str, run_id: &str, reason: String, now: i64) {
        let maybe_payload = {
            let mut store = self.store.lock().await;
            if store
                .runs
                .get(run_id)
                .and_then(|record| record.requested_stop)
                .is_some()
            {
                return;
            }
            let Some(task) = store.tasks.get_mut(task_id) else {
                return;
            };
            task.status = DevflowTaskStatus::Failed;
            task.updated_at = now;
            let task_clone = task.clone();
            let Some(record) = store.runs.get_mut(run_id) else {
                return;
            };
            record.run.status = DevflowRunStatus::Failed;
            record.run.completed_at = Some(now);
            record.run.exit_reason = Some(reason);
            Some((task_clone, record.run.clone()))
        };

        if let Some((task, run)) = maybe_payload {
            self.send_task_status_changed(task).await;
            self.send_run_status_changed(run).await;
        }
    }

    async fn load_devflow_config(
        &self,
        task: &DevflowTask,
        execution_cwd: &str,
    ) -> std::io::Result<Config> {
        let approval_policy = load_approval_policy(self.config.codex_home.as_path())
            .await
            .map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("failed to load devflow approval policy: {err}"),
                )
            })?;
        self.config_manager
            .load_with_overrides(
                /*request_overrides*/ None,
                ConfigOverrides {
                    cwd: Some(PathBuf::from(execution_cwd)),
                    sandbox_mode: Some(SandboxMode::WorkspaceWrite),
                    approval_policy: Some(
                        approval_policy_for_risk(&approval_policy, task.risk_level).to_core(),
                    ),
                    approvals_reviewer: Some(approval_policy.approvals_reviewer.to_core()),
                    codex_linux_sandbox_exe: self.arg0_paths.codex_linux_sandbox_exe.clone(),
                    main_execve_wrapper_exe: self.arg0_paths.main_execve_wrapper_exe.clone(),
                    ..Default::default()
                },
            )
            .await
    }

    fn listener_task_context(&self) -> ListenerTaskContext {
        ListenerTaskContext {
            thread_manager: Arc::clone(&self.thread_manager),
            thread_state_manager: self.thread_state_manager.clone(),
            outgoing: Arc::clone(&self.outgoing),
            pending_thread_unloads: Arc::clone(&self.pending_thread_unloads),
            analytics_events_client: self.analytics_events_client.clone(),
            thread_watch_manager: self.thread_watch_manager.clone(),
            thread_list_state_permit: Arc::clone(&self.thread_list_state_permit),
            fallback_model_provider: self.config.model_provider_id.clone(),
            codex_home: self.config.codex_home.to_path_buf(),
        }
    }

    fn agent_by_id(&self, agent_id: &str) -> Result<DevflowAgent, JSONRPCErrorError> {
        match agent_id {
            "codex-main" => Ok(self.codex_agent(None)),
            "claude-writer" => Ok(self.local_agent(StaticAgentDescriptor {
                id: "claude-writer",
                name: "Claude Code",
                runtime: DevflowAgentRuntime::Claude,
                lane: DevflowAgentLane::Legacy,
                root: DEFAULT_CLAUDE_ROOT,
                launch_command: "cd /Users/yuqei/claude-code && claude",
                roles: &["report", "design"],
                capabilities: &["writing", "review", "artifact-generation"],
            })),
            "claude-reviewer" => Ok(self.local_agent(StaticAgentDescriptor {
                id: "claude-reviewer",
                name: "Claude Code Reviewer",
                runtime: DevflowAgentRuntime::Claude,
                lane: DevflowAgentLane::Legacy,
                root: DEFAULT_CLAUDE_ROOT,
                launch_command: "cd /Users/yuqei/claude-code && claude",
                roles: &["review", "second-opinion"],
                capabilities: &["writing", "review", "artifact-generation"],
            })),
            "hermes-automation" => Ok(self.local_agent(StaticAgentDescriptor {
                id: "hermes-automation",
                name: "Hermes Agent",
                runtime: DevflowAgentRuntime::Hermes,
                lane: DevflowAgentLane::Legacy,
                root: DEFAULT_HERMES_ROOT,
                launch_command: "cd /Users/yuqei/hermes-agent && hermes",
                roles: &["automation", "memory", "delivery"],
                capabilities: &["triggering", "memory", "messaging"],
            })),
            _ => Err(invalid_request(format!(
                "unknown devflow agent id: {agent_id}"
            ))),
        }
    }

    fn detect_agents(&self, params: DevflowAgentDetectParams) -> Vec<DevflowAgent> {
        vec![
            self.codex_agent(params.codex_root.as_deref()),
            self.local_agent(StaticAgentDescriptor {
                id: "claude-writer",
                name: "Claude Code",
                runtime: DevflowAgentRuntime::Claude,
                lane: DevflowAgentLane::Legacy,
                root: params.claude_root.as_deref().unwrap_or(DEFAULT_CLAUDE_ROOT),
                launch_command: "cd /Users/yuqei/claude-code && claude",
                roles: &["report", "design"],
                capabilities: &["writing", "review", "artifact-generation"],
            }),
            self.local_agent(StaticAgentDescriptor {
                id: "claude-reviewer",
                name: "Claude Code Reviewer",
                runtime: DevflowAgentRuntime::Claude,
                lane: DevflowAgentLane::Legacy,
                root: params.claude_root.as_deref().unwrap_or(DEFAULT_CLAUDE_ROOT),
                launch_command: "cd /Users/yuqei/claude-code && claude",
                roles: &["review", "second-opinion"],
                capabilities: &["writing", "review", "artifact-generation"],
            }),
            self.local_agent(StaticAgentDescriptor {
                id: "hermes-automation",
                name: "Hermes Agent",
                runtime: DevflowAgentRuntime::Hermes,
                lane: DevflowAgentLane::Legacy,
                root: params.hermes_root.as_deref().unwrap_or(DEFAULT_HERMES_ROOT),
                launch_command: "cd /Users/yuqei/hermes-agent && hermes",
                roles: &["automation", "memory", "delivery"],
                capabilities: &["triggering", "memory", "messaging"],
            }),
        ]
    }

    fn codex_agent(&self, override_root: Option<&str>) -> DevflowAgent {
        let root_path = override_root.map(str::to_owned).or_else(|| {
            self.arg0_paths
                .codex_self_exe
                .as_ref()
                .and_then(|path| exe_root_path(path.as_path()))
        });

        DevflowAgent {
            id: "codex-main".to_string(),
            name: "Codex".to_string(),
            runtime: DevflowAgentRuntime::Codex,
            lane: DevflowAgentLane::Main,
            roles: vec![
                "implementation".to_string(),
                "review".to_string(),
                "integration".to_string(),
            ],
            root_path: root_path.clone(),
            launch_command: Some("codex".to_string()),
            status: DevflowAgentStatus::Available,
            capabilities: vec![
                "coding".to_string(),
                "testing".to_string(),
                "review".to_string(),
                "turn-runtime".to_string(),
            ],
            diagnostics: vec![format!(
                "codex runtime ready{}",
                root_path
                    .as_ref()
                    .map(|value| format!(" at {value}"))
                    .unwrap_or_default()
            )],
            last_error: None,
        }
    }

    fn local_agent(&self, descriptor: StaticAgentDescriptor<'_>) -> DevflowAgent {
        let root_path = descriptor.root.to_string();
        let status = if Path::new(descriptor.root).exists() {
            DevflowAgentStatus::Available
        } else {
            DevflowAgentStatus::Missing
        };
        let (diagnostics, last_error) = match status {
            DevflowAgentStatus::Available => (
                vec![format!("detected local checkout at {}", descriptor.root)],
                None,
            ),
            DevflowAgentStatus::Missing => (
                vec![format!("expected local checkout at {}", descriptor.root)],
                Some(format!("missing local checkout: {}", descriptor.root)),
            ),
        };

        DevflowAgent {
            id: descriptor.id.to_string(),
            name: descriptor.name.to_string(),
            runtime: descriptor.runtime,
            lane: descriptor.lane,
            roles: descriptor.roles.iter().map(ToString::to_string).collect(),
            root_path: Some(root_path),
            launch_command: Some(descriptor.launch_command.to_string()),
            status,
            capabilities: descriptor
                .capabilities
                .iter()
                .map(ToString::to_string)
                .collect(),
            diagnostics,
            last_error,
        }
    }
}

fn watchdog_status_for_queue_snapshot(
    alerts: &[DevflowWatchdogAlert],
    has_running: bool,
) -> DevflowWatchdogStatus {
    if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::TimedOut)
    {
        DevflowWatchdogStatus::TimedOut
    } else if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::Quarantined)
    {
        DevflowWatchdogStatus::Quarantined
    } else if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::Recovering)
    {
        DevflowWatchdogStatus::Recovering
    } else if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::NoProgress)
    {
        DevflowWatchdogStatus::NoProgress
    } else if has_running {
        DevflowWatchdogStatus::Running
    } else {
        DevflowWatchdogStatus::Idle
    }
}

fn store_snapshot_persist_watchdog_alert(error: &str, created_at: i64) -> DevflowWatchdogAlert {
    DevflowWatchdogAlert {
        id: STORE_SNAPSHOT_PERSIST_ALERT_ID.to_string(),
        status: DevflowWatchdogStatus::Recovering,
        severity: DevflowWatchdogAlertSeverity::Critical,
        project_id: None,
        task_id: None,
        run_id: None,
        message: format!(
            "Devflow store snapshot could not be persisted; recent runtime indexes may not survive restart: {error}"
        ),
        created_at,
    }
}

fn watchdog_queue_item_for_task(
    task: &DevflowTask,
    run: Option<&DevflowRun>,
    reason: String,
) -> DevflowWatchdogQueueItem {
    DevflowWatchdogQueueItem {
        task_id: Some(task.id.clone()),
        run_id: run.map(|run| run.id.clone()),
        project_id: Some(task.project_id.clone()),
        title: Some(task.title.clone()),
        task_status: Some(task.status),
        run_status: run.map(|run| run.status),
        agent_id: run.map(|run| run.agent_id.clone()),
        updated_at: Some(task.updated_at),
        alert_id: None,
        alert_severity: None,
        reason,
    }
}

fn watchdog_queue_item_for_alert(
    store: &DevflowStore,
    alert: &DevflowWatchdogAlert,
) -> DevflowWatchdogQueueItem {
    let task = alert
        .task_id
        .as_deref()
        .and_then(|task_id| store.tasks.get(task_id));
    let run = alert
        .run_id
        .as_deref()
        .and_then(|run_id| store.runs.get(run_id))
        .map(|record| &record.run);
    let project_id = alert
        .project_id
        .clone()
        .or_else(|| task.map(|task| task.project_id.clone()));
    let updated_at = task
        .map(|task| task.updated_at)
        .or_else(|| run.and_then(|run| run.completed_at))
        .or_else(|| run.map(|run| run.started_at))
        .or(Some(alert.created_at));

    DevflowWatchdogQueueItem {
        task_id: alert.task_id.clone(),
        run_id: alert.run_id.clone(),
        project_id,
        title: task.map(|task| task.title.clone()),
        task_status: task.map(|task| task.status),
        run_status: run.map(|run| run.status),
        agent_id: run.map(|run| run.agent_id.clone()),
        updated_at,
        alert_id: Some(alert.id.clone()),
        alert_severity: Some(alert.severity),
        reason: alert.message.clone(),
    }
}

fn artifact_file_path(project_root: &str, run_id: &str, suffix: &str, extension: &str) -> PathBuf {
    PathBuf::from(project_root)
        .join(".codex")
        .join("devflow")
        .join("artifacts")
        .join(format!("{run_id}-{suffix}.{extension}"))
}

fn project_memory_path(project_root: &str) -> PathBuf {
    PathBuf::from(project_root)
        .join(".codex")
        .join("devflow")
        .join("project-memory.md")
}

fn task_requires_plan_artifact(risk_level: DevflowTaskRiskLevel) -> bool {
    matches!(
        risk_level,
        DevflowTaskRiskLevel::Medium | DevflowTaskRiskLevel::High
    )
}

fn task_has_plan_artifact(store: &DevflowStore, task: &DevflowTask) -> bool {
    task.artifact_ids.iter().any(|artifact_id| {
        store.artifacts.get(artifact_id).is_some_and(|artifact| {
            artifact.task_id == task.id
                && artifact.kind == DevflowArtifactKind::Report
                && artifact.title.to_ascii_lowercase().contains("plan")
        })
    })
}

fn task_risk_level_label(risk_level: DevflowTaskRiskLevel) -> &'static str {
    match risk_level {
        DevflowTaskRiskLevel::Low => "low",
        DevflowTaskRiskLevel::Medium => "medium",
        DevflowTaskRiskLevel::High => "high",
    }
}

async fn read_project_memory_summary(path: &Path) -> Result<Option<String>, JSONRPCErrorError> {
    match fs::read_to_string(path).await {
        Ok(summary) => Ok(Some(summary)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(internal_error(format!(
            "failed to read project memory summary: {err}"
        ))),
    }
}

async fn write_artifact_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, content).await
}

async fn append_artifact_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(content.as_bytes()).await
}

fn append_stream_summary(summary: &mut Option<String>, delta: &str) {
    let mut next = summary.take().unwrap_or_default();
    next.push_str(delta);
    if next.chars().count() > STREAM_SUMMARY_LIMIT {
        let overflow = next.chars().count() - STREAM_SUMMARY_LIMIT;
        next = next.chars().skip(overflow).collect();
    }
    *summary = Some(next);
}

fn devflow_turn_prompt(task: &DevflowTask) -> String {
    match task.kind {
        DevflowTaskKind::Implementation => {
            let root_cause_instruction = if task_requires_root_cause(task) {
                "\n\nThis task appears to be a bug fix. Before finishing, identify the root cause from evidence and include a final `Root cause:` line. If the root cause is still unknown, say so explicitly so the release gate can stay blocked."
            } else {
                ""
            };
            format!(
                "You are executing a Devflow implementation task.\n\nProject root: {}\nTask title: {}\nObjective: {}\n\nImplement the requested change in this project, use tools when needed, and run the most relevant focused verification you can before finishing. Leave the final result ready for review.{}",
                task.project_id, task.title, task.objective, root_cause_instruction
            )
        }
        DevflowTaskKind::Review => format!(
            "You are executing a Devflow review task.\n\nProject root: {}\nTask title: {}\nObjective: {}\n\nProduce a concise review-ready analysis of the requested work and keep the result ready for review.",
            task.project_id, task.title, task.objective
        ),
        DevflowTaskKind::Report => format!(
            "You are executing a Devflow report task.\n\nProject root: {}\nTask title: {}\nObjective: {}\n\nProduce a concise project report with the requested evidence and next steps.",
            task.project_id, task.title, task.objective
        ),
        DevflowTaskKind::Diagnostic => format!(
            "You are executing a Devflow diagnostic task.\n\nProject root: {}\nTask title: {}\nObjective: {}\n\nInvestigate the issue, collect the most relevant evidence, and summarize the result clearly. Include a final `Root cause:` line when the cause is identified. If the cause is unknown, write `Root cause: unknown` so the release gate remains blocked.",
            task.project_id, task.title, task.objective
        ),
        DevflowTaskKind::Automation => format!(
            "You are executing a Devflow automation task.\n\nProject root: {}\nTask title: {}\nObjective: {}\n\nPerform the requested automation carefully and summarize the outcome clearly.",
            task.project_id, task.title, task.objective
        ),
    }
}

fn build_task_plan_steps(title: &str, objective: &str, max_tasks: Option<u32>) -> Vec<String> {
    let max_tasks = max_tasks.unwrap_or(3).clamp(1, 6) as usize;
    let mut steps = objective
        .lines()
        .map(str::trim)
        .map(|line| {
            line.trim_start_matches(|ch: char| {
                matches!(ch, '-' | '*' | '•' | '.' | ')') || ch.is_ascii_digit()
            })
        })
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if steps.len() <= 1 {
        steps = objective
            .split(['\n', ';'])
            .flat_map(|segment| segment.split(" and "))
            .flat_map(|segment| segment.split(" then "))
            .flat_map(|segment| segment.split('。'))
            .flat_map(|segment| segment.split("然后"))
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
    }
    if steps.is_empty() {
        return vec![format!(
            "Implement the primary requirement for {title}: {objective}"
        )];
    }
    steps.truncate(max_tasks);
    steps
}

fn unresolved_dependencies(store: &DevflowStore, task: &DevflowTask) -> Vec<String> {
    task.dependencies
        .iter()
        .filter_map(|dependency_id| match store.tasks.get(dependency_id) {
            Some(dependency_task) if task_satisfies_dependency(dependency_task) => None,
            Some(_) => Some(dependency_id.clone()),
            None => Some(dependency_id.clone()),
        })
        .collect()
}

fn completed_task_status(
    task: &DevflowTask,
    review_ready: bool,
) -> (DevflowTaskStatus, DevflowRunStatus, &'static str) {
    if task.kind == DevflowTaskKind::Implementation && review_ready {
        (
            DevflowTaskStatus::ReadyToMerge,
            DevflowRunStatus::ReadyToMerge,
            "ready_to_merge",
        )
    } else {
        (
            DevflowTaskStatus::ReadyForReview,
            DevflowRunStatus::ReadyForReview,
            "ready_for_review",
        )
    }
}

fn task_satisfies_dependency(task: &DevflowTask) -> bool {
    match task.kind {
        DevflowTaskKind::Implementation => task.status == DevflowTaskStatus::ReadyToMerge,
        DevflowTaskKind::Review
        | DevflowTaskKind::Report
        | DevflowTaskKind::Diagnostic
        | DevflowTaskKind::Automation => matches!(
            task.status,
            DevflowTaskStatus::ReadyForReview | DevflowTaskStatus::ReadyToMerge
        ),
    }
}

fn validate_task_dependencies(
    store: &DevflowStore,
    task_id: &str,
    dependencies: &[String],
) -> Result<(), JSONRPCErrorError> {
    if dependencies
        .iter()
        .any(|dependency_id| dependency_id == task_id)
    {
        return Err(invalid_request(format!(
            "task cannot depend on itself: {task_id}"
        )));
    }
    for dependency_id in dependencies {
        if !store.tasks.contains_key(dependency_id) {
            return Err(invalid_request(format!(
                "unknown dependency task id: {dependency_id}"
            )));
        }
        if dependency_reaches_task(store, dependency_id, task_id) {
            return Err(invalid_request(format!(
                "dependency cycle detected for task {task_id} via {dependency_id}"
            )));
        }
    }
    Ok(())
}

fn dependency_reaches_task(store: &DevflowStore, start_id: &str, target_id: &str) -> bool {
    let mut stack = vec![start_id.to_string()];
    let mut visited = HashSet::new();
    while let Some(task_id) = stack.pop() {
        if !visited.insert(task_id.clone()) {
            continue;
        }
        if task_id == target_id {
            return true;
        }
        if let Some(task) = store.tasks.get(&task_id) {
            stack.extend(task.dependencies.iter().cloned());
        }
    }
    false
}

fn should_include_dependency_artifact(
    task_kind: DevflowTaskKind,
    artifact_kind: DevflowArtifactKind,
) -> bool {
    match task_kind {
        DevflowTaskKind::Review => matches!(
            artifact_kind,
            DevflowArtifactKind::ContextPack
                | DevflowArtifactKind::DeliveryReceipt
                | DevflowArtifactKind::Diff
                | DevflowArtifactKind::QualityGateOutput
                | DevflowArtifactKind::RunSummary
                | DevflowArtifactKind::Report
                | DevflowArtifactKind::ReviewReport
        ),
        DevflowTaskKind::Report => matches!(
            artifact_kind,
            DevflowArtifactKind::ContextPack
                | DevflowArtifactKind::DeliveryReceipt
                | DevflowArtifactKind::RunSummary
                | DevflowArtifactKind::Report
                | DevflowArtifactKind::ReviewReport
        ),
        DevflowTaskKind::Implementation
        | DevflowTaskKind::Diagnostic
        | DevflowTaskKind::Automation => false,
    }
}

fn combine_external_agent_output(execution: &ExternalAgentExecution) -> String {
    match (execution.stdout.trim(), execution.stderr.trim()) {
        ("", "") => String::new(),
        (stdout, "") => stdout.to_string(),
        ("", stderr) => stderr.to_string(),
        (stdout, stderr) => format!("{stdout}\n\n[stderr]\n{stderr}"),
    }
}

fn external_agent_failure_reason(execution: &ExternalAgentExecution) -> String {
    let summary = combine_external_agent_output(execution);
    let exit_code = execution
        .exit_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    if summary.is_empty() {
        return format!("Claude adapter failed with exit code {exit_code}");
    }
    format!(
        "Claude adapter failed with exit code {exit_code}: {}",
        truncate(&summary, COMMAND_OUTPUT_SUMMARY_LIMIT)
    )
}

fn devflow_agent_lifecycle_noop_message(action: &str) -> String {
    format!(
        "devflowAgent/{action} is a safe no-op in the current MVP; Devflow does not own long-running Claude or Hermes services yet. Use devflowAgent/diagnose for health checks and devflowTask/pause or devflowTask/cancel for active work."
    )
}

fn default_agent_id(task_kind: DevflowTaskKind) -> &'static str {
    match task_kind {
        DevflowTaskKind::Implementation
        | DevflowTaskKind::Diagnostic
        | DevflowTaskKind::Review
        | DevflowTaskKind::Report
        | DevflowTaskKind::Automation => "codex-main",
    }
}

fn default_trigger_source(assigned_agent_id: Option<&str>) -> Option<String> {
    (assigned_agent_id == Some("hermes-automation")).then_some("hermes:manual".to_string())
}

fn approval_decision_accepts(decision: DevflowApprovalDecision) -> bool {
    matches!(
        decision,
        DevflowApprovalDecision::Accept
            | DevflowApprovalDecision::AcceptForSession
            | DevflowApprovalDecision::AcceptForTask
            | DevflowApprovalDecision::AcceptForProject
    )
}

fn approval_decision_creates_grant(decision: DevflowApprovalDecision) -> bool {
    matches!(
        decision,
        DevflowApprovalDecision::AcceptForTask | DevflowApprovalDecision::AcceptForProject
    )
}

fn mark_approval_responded(approval: &mut DevflowApproval, decision: DevflowApprovalDecision) {
    approval.status = DevflowApprovalStatus::Responded;
    approval.responded_at = Some(Utc::now().timestamp());
    approval.decision = Some(decision);
}

fn devflow_approval_grant(
    approval: &DevflowApproval,
    decision: DevflowApprovalDecision,
) -> Option<DevflowApprovalGrant> {
    if !approval_decision_creates_grant(decision)
        || approval.kind == DevflowApprovalKind::QualityGateWaive
        || approval.kind == DevflowApprovalKind::ArtifactDelivery
    {
        return None;
    }
    Some(DevflowApprovalGrant {
        project_id: approval.project_id.clone(),
        task_id: (decision == DevflowApprovalDecision::AcceptForTask)
            .then(|| approval.task_id.clone()),
        kind: approval.kind,
        command: approval.command.clone(),
        cwd: approval.cwd.clone(),
        file_paths: approval.file_paths.clone(),
        requested_permissions: approval.requested_permissions.clone(),
        decision,
    })
}

fn devflow_approval_grant_matches(
    grant: &DevflowApprovalGrant,
    approval: &DevflowApproval,
) -> bool {
    if grant.project_id != approval.project_id || grant.kind != approval.kind {
        return false;
    }
    if grant
        .task_id
        .as_ref()
        .is_some_and(|task_id| task_id != &approval.task_id)
    {
        return false;
    }
    grant.command == approval.command
        && grant.cwd == approval.cwd
        && grant.file_paths == approval.file_paths
        && grant.requested_permissions == approval.requested_permissions
}

fn pending_approval_request_id(request: &PendingDevflowApprovalRequest) -> Option<&RequestId> {
    match request {
        PendingDevflowApprovalRequest::CommandExecution { request_id, .. }
        | PendingDevflowApprovalRequest::FileChange { request_id, .. }
        | PendingDevflowApprovalRequest::Permissions { request_id, .. } => Some(request_id),
        PendingDevflowApprovalRequest::QualityGateWaive { .. }
        | PendingDevflowApprovalRequest::ArtifactDelivery { .. } => None,
    }
}

fn approval_response_value(
    request: &PendingDevflowApprovalRequest,
    decision: DevflowApprovalDecision,
    scope: Option<PermissionGrantScope>,
) -> Result<JsonRpcResultValue, JSONRPCErrorError> {
    match request {
        PendingDevflowApprovalRequest::CommandExecution { .. } => {
            serde_json::to_value(CommandExecutionRequestApprovalResponse {
                decision: match decision {
                    DevflowApprovalDecision::Accept
                    | DevflowApprovalDecision::AcceptForTask
                    | DevflowApprovalDecision::AcceptForProject => {
                        CommandExecutionApprovalDecision::Accept
                    }
                    DevflowApprovalDecision::AcceptForSession => {
                        CommandExecutionApprovalDecision::AcceptForSession
                    }
                    DevflowApprovalDecision::Decline => CommandExecutionApprovalDecision::Decline,
                    DevflowApprovalDecision::Cancel => CommandExecutionApprovalDecision::Cancel,
                },
            })
            .map_err(|err| internal_error(format!("failed to serialize approval response: {err}")))
        }
        PendingDevflowApprovalRequest::FileChange { .. } => {
            serde_json::to_value(FileChangeRequestApprovalResponse {
                decision: match decision {
                    DevflowApprovalDecision::Accept
                    | DevflowApprovalDecision::AcceptForTask
                    | DevflowApprovalDecision::AcceptForProject => {
                        FileChangeApprovalDecision::Accept
                    }
                    DevflowApprovalDecision::AcceptForSession => {
                        FileChangeApprovalDecision::AcceptForSession
                    }
                    DevflowApprovalDecision::Decline => FileChangeApprovalDecision::Decline,
                    DevflowApprovalDecision::Cancel => FileChangeApprovalDecision::Cancel,
                },
            })
            .map_err(|err| internal_error(format!("failed to serialize approval response: {err}")))
        }
        PendingDevflowApprovalRequest::Permissions { params, .. } => {
            let permissions = match decision {
                DevflowApprovalDecision::Accept
                | DevflowApprovalDecision::AcceptForSession
                | DevflowApprovalDecision::AcceptForTask
                | DevflowApprovalDecision::AcceptForProject => GrantedPermissionProfile {
                    network: params.permissions.network.clone(),
                    file_system: params.permissions.file_system.clone(),
                },
                DevflowApprovalDecision::Decline | DevflowApprovalDecision::Cancel => {
                    GrantedPermissionProfile::default()
                }
            };
            serde_json::to_value(PermissionsRequestApprovalResponse {
                permissions,
                scope: scope.unwrap_or({
                    if matches!(decision, DevflowApprovalDecision::AcceptForSession) {
                        PermissionGrantScope::Session
                    } else {
                        PermissionGrantScope::Turn
                    }
                }),
                strict_auto_review: None,
            })
            .map_err(|err| internal_error(format!("failed to serialize approval response: {err}")))
        }
        PendingDevflowApprovalRequest::QualityGateWaive { .. } => Err(invalid_request(
            "quality gate waive approvals do not use direct client approval callbacks".to_string(),
        )),
        PendingDevflowApprovalRequest::ArtifactDelivery { .. } => Err(invalid_request(
            "artifact delivery approvals do not use direct client approval callbacks".to_string(),
        )),
    }
}

fn request_id_to_string(request_id: &RequestId) -> String {
    match request_id {
        RequestId::String(value) => value.clone(),
        RequestId::Integer(value) => value.to_string(),
    }
}

fn hermes_task_command(task: &DevflowTask) -> Result<(String, Vec<String>), String> {
    let text = format!("{} {}", task.title, task.objective).to_ascii_lowercase();
    if text.contains("doctor") || text.contains("diagnos") {
        return Ok(("hermes doctor".to_string(), vec!["doctor".to_string()]));
    }
    if text.contains("cron") && text.contains("list") {
        return Ok((
            "hermes cron list".to_string(),
            vec!["cron".to_string(), "list".to_string()],
        ));
    }
    if text.contains("webhook") && text.contains("list") {
        return Ok((
            "hermes webhook list".to_string(),
            vec!["webhook".to_string(), "list".to_string()],
        ));
    }
    if text.contains("status") {
        return Ok(("hermes status".to_string(), vec!["status".to_string()]));
    }
    Err(
        "unsupported Hermes automation objective; currently supported commands are doctor, status, cron list, and webhook list"
            .to_string(),
    )
}

fn thread_start_error(err: CodexErr) -> JSONRPCErrorError {
    match err {
        CodexErr::InvalidRequest(message) => invalid_request(message),
        other => internal_error(format!("failed to start devflow thread: {other}")),
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let prefix = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

fn exe_root_path(path: &Path) -> Option<String> {
    path.parent().map(|parent| parent.display().to_string())
}

#[cfg(test)]
#[path = "devflow_processor_tests.rs"]
mod tests;
