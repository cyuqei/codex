use std::collections::BTreeMap;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::create_apply_patch_sse_response;
use app_test_support::create_exec_command_sse_response;
use app_test_support::create_final_assistant_message_sse_response;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::create_mock_responses_server_sequence;
use app_test_support::create_mock_responses_server_sequence_unchecked;
use app_test_support::create_shell_command_sse_response_with_permissions;
use app_test_support::to_response;
use app_test_support::write_mock_responses_config_toml;
use codex_app_server_protocol::ApprovalsReviewer;
use codex_app_server_protocol::AskForApproval;
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
use codex_app_server_protocol::DevflowApprovalDecision;
use codex_app_server_protocol::DevflowApprovalListParams;
use codex_app_server_protocol::DevflowApprovalListResponse;
use codex_app_server_protocol::DevflowApprovalPolicy;
use codex_app_server_protocol::DevflowApprovalPolicyReadParams;
use codex_app_server_protocol::DevflowApprovalPolicyReadResponse;
use codex_app_server_protocol::DevflowApprovalPolicyUpdateParams;
use codex_app_server_protocol::DevflowApprovalPolicyUpdateResponse;
use codex_app_server_protocol::DevflowApprovalRequestedNotification;
use codex_app_server_protocol::DevflowApprovalRespondParams;
use codex_app_server_protocol::DevflowApprovalRespondResponse;
use codex_app_server_protocol::DevflowApprovalStatus;
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
use codex_app_server_protocol::DevflowCapabilityPackListParams;
use codex_app_server_protocol::DevflowCapabilityPackListResponse;
use codex_app_server_protocol::DevflowCapabilityPackReadParams;
use codex_app_server_protocol::DevflowCapabilityPackReadResponse;
use codex_app_server_protocol::DevflowCapabilityPackRunParams;
use codex_app_server_protocol::DevflowCapabilityPackRunResponse;
use codex_app_server_protocol::DevflowCapabilityPackRunStatus;
use codex_app_server_protocol::DevflowPackStatus;
use codex_app_server_protocol::DevflowPolicyPackApplyParams;
use codex_app_server_protocol::DevflowPolicyPackApplyResponse;
use codex_app_server_protocol::DevflowPolicyPackListParams;
use codex_app_server_protocol::DevflowPolicyPackListResponse;
use codex_app_server_protocol::DevflowPolicyPackReadParams;
use codex_app_server_protocol::DevflowPolicyPackReadResponse;
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
use codex_app_server_protocol::DevflowReleasePrepStatus;
use codex_app_server_protocol::DevflowReleaseSubmitMode;
use codex_app_server_protocol::DevflowReleaseSubmitParams;
use codex_app_server_protocol::DevflowReleaseSubmitResponse;
use codex_app_server_protocol::DevflowReleaseSubmitStatus;
use codex_app_server_protocol::DevflowRunCommandCompletedNotification;
use codex_app_server_protocol::DevflowRunCommandStartedNotification;
use codex_app_server_protocol::DevflowRunDiffUpdatedNotification;
use codex_app_server_protocol::DevflowRunOutputDeltaNotification;
use codex_app_server_protocol::DevflowRunOutputSource;
use codex_app_server_protocol::DevflowRunStatus;
use codex_app_server_protocol::DevflowRunStatusChangedNotification;
use codex_app_server_protocol::DevflowSupportBundleCreateParams;
use codex_app_server_protocol::DevflowSupportBundleCreateResponse;
use codex_app_server_protocol::DevflowTaskAssignParams;
use codex_app_server_protocol::DevflowTaskAssignResponse;
use codex_app_server_protocol::DevflowTaskCancelParams;
use codex_app_server_protocol::DevflowTaskCancelResponse;
use codex_app_server_protocol::DevflowTaskCreateParams;
use codex_app_server_protocol::DevflowTaskCreateResponse;
use codex_app_server_protocol::DevflowTaskDependenciesUpdateParams;
use codex_app_server_protocol::DevflowTaskDependenciesUpdateResponse;
use codex_app_server_protocol::DevflowTaskDispatchParams;
use codex_app_server_protocol::DevflowTaskDispatchResponse;
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
use codex_app_server_protocol::DevflowWatchdogAlertCreatedNotification;
use codex_app_server_protocol::DevflowWatchdogAlertSeverity;
use codex_app_server_protocol::DevflowWatchdogAlertsParams;
use codex_app_server_protocol::DevflowWatchdogAlertsResponse;
use codex_app_server_protocol::DevflowWatchdogReadParams;
use codex_app_server_protocol::DevflowWatchdogReadResponse;
use codex_app_server_protocol::DevflowWatchdogReconcileParams;
use codex_app_server_protocol::DevflowWatchdogReconcileResponse;
use codex_app_server_protocol::DevflowWatchdogStatus;
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
use codex_app_server_protocol::DevflowWorktreeStatus;
use codex_app_server_protocol::DevflowWorktreeStatusChangedNotification;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCNotification;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::PermissionGrantScope;
use codex_app_server_protocol::RequestId;
use codex_features::Feature;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;

fn init_git_repo(root: &std::path::Path) -> Result<()> {
    run_git(root, &["init", "-b", "main"])?;
    run_git(root, &["config", "user.email", "codex@example.com"])?;
    run_git(root, &["config", "user.name", "Codex Test"])?;
    Ok(())
}

fn run_git(root: &std::path::Path, args: &[&str]) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    ))
}

#[cfg(unix)]
fn write_fake_claude_cli(
    root: &std::path::Path,
    output: &str,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let script_path = root.join("fake-claude.sh");
    let args_path = root.join("claude-args.txt");
    let cwd_path = root.join("claude-cwd.txt");
    std::fs::write(
        &script_path,
        format!(
            "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > \"{}\"\npwd > \"{}\"\ncat <<'EOF'\n{}\nEOF\n",
            args_path.display(),
            cwd_path.display(),
            output
        ),
    )?;
    let mut permissions = std::fs::metadata(&script_path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script_path, permissions)?;
    Ok((script_path, args_path, cwd_path))
}

#[cfg(unix)]
fn write_fake_hermes_cli(
    root: &std::path::Path,
    output: &str,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let script_path = root.join("fake-hermes.sh");
    let args_path = root.join("hermes-args.txt");
    let cwd_path = root.join("hermes-cwd.txt");
    std::fs::write(
        &script_path,
        format!(
            "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > \"{}\"\npwd > \"{}\"\ncat <<'EOF'\n{}\nEOF\n",
            args_path.display(),
            cwd_path.display(),
            output
        ),
    )?;
    let mut permissions = std::fs::metadata(&script_path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script_path, permissions)?;
    Ok((script_path, args_path, cwd_path))
}

#[cfg(unix)]
fn write_fake_browse_cli(project_root: &std::path::Path, script: &str) -> Result<PathBuf> {
    let script_path = project_root
        .join(".codex")
        .join("skills")
        .join("gstack")
        .join("browse")
        .join("dist")
        .join("browse");
    let parent = script_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("fake browse path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(&script_path, script)?;
    let mut permissions = std::fs::metadata(&script_path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script_path, permissions)?;
    Ok(script_path)
}

#[tokio::test]
async fn devflow_agent_detect_marks_codex_main_and_external_agents_legacy() -> Result<()> {
    let codex_home = TempDir::new()?;
    let claude_root = TempDir::new()?;
    let hermes_root = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_devflow_agent_detect_request(DevflowAgentDetectParams {
            codex_root: Some("/tmp/codex-runtime".to_string()),
            claude_root: Some(claude_root.path().display().to_string()),
            hermes_root: Some(hermes_root.path().display().to_string()),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let DevflowAgentDetectResponse { agents } = to_response(response)?;

    assert_eq!(agents.len(), 7);
    assert_eq!(agents[0].id, "codex-main");
    assert_eq!(agents[0].runtime, DevflowAgentRuntime::Codex);
    assert_eq!(agents[0].lane, DevflowAgentLane::Main);
    assert_eq!(agents[0].status, DevflowAgentStatus::Available);
    assert_eq!(agents[0].root_path.as_deref(), Some("/tmp/codex-runtime"));
    assert!(agents[0].roles.contains(&"planner".to_string()));
    assert_eq!(agents[1].id, "codex-worker");
    assert_eq!(agents[1].runtime, DevflowAgentRuntime::Codex);
    assert_eq!(agents[1].lane, DevflowAgentLane::Main);
    assert_eq!(agents[1].status, DevflowAgentStatus::Available);
    assert_eq!(agents[1].root_path.as_deref(), Some("/tmp/codex-runtime"));
    assert!(agents[1].roles.contains(&"worker".to_string()));
    assert_eq!(agents[2].id, "codex-reviewer");
    assert_eq!(agents[2].runtime, DevflowAgentRuntime::Codex);
    assert_eq!(agents[2].lane, DevflowAgentLane::Main);
    assert_eq!(agents[2].status, DevflowAgentStatus::Available);
    assert_eq!(agents[2].root_path.as_deref(), Some("/tmp/codex-runtime"));
    assert!(agents[2].roles.contains(&"reviewer".to_string()));
    assert_eq!(agents[3].id, "codex-integrator");
    assert_eq!(agents[3].runtime, DevflowAgentRuntime::Codex);
    assert_eq!(agents[3].lane, DevflowAgentLane::Main);
    assert_eq!(agents[3].status, DevflowAgentStatus::Available);
    assert_eq!(agents[3].root_path.as_deref(), Some("/tmp/codex-runtime"));
    assert!(agents[3].roles.contains(&"integrator".to_string()));
    assert_eq!(agents[4].runtime, DevflowAgentRuntime::Claude);
    assert_eq!(agents[4].lane, DevflowAgentLane::Legacy);
    assert_eq!(agents[4].status, DevflowAgentStatus::Available);
    assert_eq!(
        agents[4].root_path.as_deref(),
        Some(claude_root.path().to_str().expect("utf-8 path"))
    );
    assert_eq!(agents[5].runtime, DevflowAgentRuntime::Claude);
    assert_eq!(agents[5].lane, DevflowAgentLane::Legacy);
    assert_eq!(agents[5].status, DevflowAgentStatus::Available);
    assert_eq!(
        agents[5].root_path.as_deref(),
        Some(claude_root.path().to_str().expect("utf-8 path"))
    );
    assert_eq!(agents[6].runtime, DevflowAgentRuntime::Hermes);
    assert_eq!(agents[6].lane, DevflowAgentLane::Legacy);
    assert_eq!(agents[6].status, DevflowAgentStatus::Available);
    assert_eq!(
        agents[6].root_path.as_deref(),
        Some(hermes_root.path().to_str().expect("utf-8 path"))
    );

    let hermes_status_changed = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowAgent/statusChanged")
                .await?;
            let payload: DevflowAgentStatusChangedNotification =
                serde_json::from_value(notification.params.expect("agent status params"))?;
            if payload.agent.id == "hermes-automation" {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        hermes_status_changed.agent.status,
        DevflowAgentStatus::Available
    );
    assert_eq!(hermes_status_changed.agent.lane, DevflowAgentLane::Legacy);
    assert_eq!(
        hermes_status_changed.agent.root_path.as_deref(),
        Some(hermes_root.path().to_str().expect("utf-8 path"))
    );
    Ok(())
}

#[tokio::test]
async fn devflow_legacy_agent_list_read_and_capabilities_roundtrip() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let list_request_id = mcp
        .send_devflow_agent_list_request(DevflowAgentListParams {
            runtimes: Some(vec![DevflowAgentRuntime::Claude]),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowAgentListResponse { data } = to_response(list_response)?;
    assert_eq!(data.len(), 2);
    assert!(
        data.iter()
            .all(|agent| agent.runtime == DevflowAgentRuntime::Claude)
    );
    assert!(
        data.iter()
            .all(|agent| agent.lane == DevflowAgentLane::Legacy)
    );

    let read_request_id = mcp
        .send_devflow_agent_read_request(DevflowAgentReadParams {
            id: "hermes-automation".to_string(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowAgentReadResponse { agent } = to_response(read_response)?;
    assert_eq!(agent.id, "hermes-automation");
    assert_eq!(agent.runtime, DevflowAgentRuntime::Hermes);
    assert_eq!(agent.lane, DevflowAgentLane::Legacy);

    let capabilities_request_id = mcp
        .send_devflow_agent_capabilities_read_request(DevflowAgentCapabilitiesReadParams {
            id: "codex-main".to_string(),
        })
        .await?;
    let capabilities_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(capabilities_request_id)),
    )
    .await??;
    let DevflowAgentCapabilitiesReadResponse { id, capabilities } =
        to_response(capabilities_response)?;
    assert_eq!(id, "codex-main");
    assert!(capabilities.contains(&"coding".to_string()));
    Ok(())
}

#[tokio::test]
async fn devflow_legacy_agent_lifecycle_methods_are_diagnostic_only_noops() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let start_request_id = mcp
        .send_devflow_agent_start_request(DevflowAgentStartParams {
            id: "hermes-automation".to_string(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowAgentStartResponse {
        agent,
        started,
        message,
    } = to_response(start_response)?;
    assert_eq!(agent.id, "hermes-automation");
    assert_eq!(agent.runtime, DevflowAgentRuntime::Hermes);
    assert_eq!(agent.lane, DevflowAgentLane::Legacy);
    assert!(!started);
    assert!(message.contains("diagnostic-only no-op"));

    let stop_request_id = mcp
        .send_devflow_agent_stop_request(DevflowAgentStopParams {
            id: "claude-writer".to_string(),
        })
        .await?;
    let stop_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(stop_request_id)),
    )
    .await??;
    let DevflowAgentStopResponse {
        agent,
        stopped,
        message,
    } = to_response(stop_response)?;
    assert_eq!(agent.id, "claude-writer");
    assert_eq!(agent.runtime, DevflowAgentRuntime::Claude);
    assert_eq!(agent.lane, DevflowAgentLane::Legacy);
    assert!(!stopped);
    assert!(message.contains("diagnostic-only no-op"));
    assert!(message.contains("Devflow does not own long-running Claude or Hermes services yet"));

    let restart_request_id = mcp
        .send_devflow_agent_restart_request(DevflowAgentRestartParams {
            id: "codex-main".to_string(),
        })
        .await?;
    let restart_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(restart_request_id)),
    )
    .await??;
    let DevflowAgentRestartResponse {
        agent,
        restarted,
        message,
    } = to_response(restart_response)?;
    assert_eq!(agent.id, "codex-main");
    assert_eq!(agent.runtime, DevflowAgentRuntime::Codex);
    assert_eq!(agent.lane, DevflowAgentLane::Main);
    assert!(!restarted);
    assert!(message.contains("devflowAgent/restart"));
    assert!(message.contains("does not start or stop codex-main here"));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_codex_only_packs_and_watchdog_roundtrip() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let policy_list_request_id = mcp
        .send_devflow_policy_pack_list_request(DevflowPolicyPackListParams {
            include_disabled: false,
        })
        .await?;
    let policy_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(policy_list_request_id)),
    )
    .await??;
    let DevflowPolicyPackListResponse { data: policy_packs } = to_response(policy_list_response)?;
    assert_eq!(policy_packs.len(), 1);
    assert_eq!(policy_packs[0].id, "superpowers-discipline");
    assert!(
        policy_packs[0]
            .policies
            .contains(&"writingPlans".to_string())
    );
    assert_ne!(policy_packs[0].status, DevflowPackStatus::Disabled);

    let policy_read_request_id = mcp
        .send_devflow_policy_pack_read_request(DevflowPolicyPackReadParams {
            id: "superpowers-discipline".to_string(),
        })
        .await?;
    let policy_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(policy_read_request_id)),
    )
    .await??;
    let DevflowPolicyPackReadResponse { pack } = to_response(policy_read_response)?;
    assert_eq!(pack, policy_packs[0]);

    let policy_project_root = TempDir::new()?;
    let create_policy_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: policy_project_root.path().display().to_string(),
            title: "Fix provider bug".to_string(),
            objective: "Repair the provider regression and preserve evidence.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::High,
            trigger_source: Some("bug-report".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_policy_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_policy_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task: policy_task } = to_response(create_policy_task_response)?;

    let policy_apply_request_id = mcp
        .send_devflow_policy_pack_apply_request(DevflowPolicyPackApplyParams {
            id: "superpowers-discipline".to_string(),
            task_id: Some(policy_task.id.clone()),
            risk_level: Some(DevflowTaskRiskLevel::High),
        })
        .await?;
    let policy_apply_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(policy_apply_request_id)),
    )
    .await??;
    let DevflowPolicyPackApplyResponse {
        applied,
        required_artifacts,
        diagnostics,
        artifact,
        ..
    } = to_response(policy_apply_response)?;
    assert!(applied);
    assert!(required_artifacts.contains(&"plan".to_string()));
    assert!(required_artifacts.contains(&"worktree".to_string()));
    assert!(required_artifacts.contains(&"verification".to_string()));
    assert!(required_artifacts.contains(&"integrationTest".to_string()));
    assert!(required_artifacts.contains(&"review".to_string()));
    assert!(required_artifacts.contains(&"rootCause".to_string()));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(&policy_task.id))
    );
    let artifact = artifact.expect("policy apply artifact");
    assert_eq!(artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(artifact.task_id, policy_task.id);
    assert!(artifact.title.contains("Policy pack application"));

    let policy_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: artifact.id.clone(),
        })
        .await?;
    let policy_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(policy_artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(policy_artifact_read_response)?;
    assert!(contents.contains("superpowers-discipline"));
    assert!(contents.contains("rootCause"));

    let capability_list_request_id = mcp
        .send_devflow_capability_pack_list_request(DevflowCapabilityPackListParams {
            include_disabled: false,
        })
        .await?;
    let capability_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(capability_list_request_id)),
    )
    .await??;
    let DevflowCapabilityPackListResponse {
        data: capability_packs,
    } = to_response(capability_list_response)?;
    assert_eq!(capability_packs.len(), 1);
    assert_eq!(capability_packs[0].id, "gstack-engineering");
    assert_eq!(
        capability_packs[0].capabilities,
        vec![
            "health".to_string(),
            "browseQa".to_string(),
            "review".to_string(),
            "benchmark".to_string(),
            "canary".to_string(),
            "watchdogQueue".to_string(),
        ]
    );

    let capability_read_request_id = mcp
        .send_devflow_capability_pack_read_request(DevflowCapabilityPackReadParams {
            id: "gstack-engineering".to_string(),
        })
        .await?;
    let capability_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(capability_read_request_id)),
    )
    .await??;
    let DevflowCapabilityPackReadResponse { pack } = to_response(capability_read_response)?;
    assert_eq!(pack, capability_packs[0]);

    let unknown_capability_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("artifactDeliver".to_string()),
            task_id: None,
            project_root: None,
        })
        .await?;
    let unknown_capability_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(unknown_capability_request_id)),
    )
    .await??;
    assert_eq!(
        unknown_capability_error.error.code,
        INVALID_REQUEST_ERROR_CODE
    );
    assert!(
        unknown_capability_error
            .error
            .message
            .contains("unknown capability"),
        "unexpected capability error: {}",
        unknown_capability_error.error.message
    );

    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("README.md"), "healthy\n")?;
    std::fs::write(
        project_root.path().join("package.json"),
        r#"{"scripts":{"dev":"vite --host 127.0.0.1 --port 5173"}}"#,
    )?;
    std::fs::create_dir_all(project_root.path().join("public"))?;
    std::fs::write(
        project_root.path().join("public").join("index.html"),
        "<h1>healthy</h1>\n",
    )?;
    write_fake_browse_cli(
        project_root.path(),
        r#"#!/bin/sh
printf '%s\n' "$*" >> .codex/fake-browse.log
if [ "$1" = "screenshot" ]; then
  printf '\211PNG\r\n\032\n' > "$2"
fi
"#,
    )?;
    run_git(project_root.path(), &["add", "README.md"])?;
    run_git(project_root.path(), &["add", "package.json"])?;
    run_git(project_root.path(), &["add", "public/index.html"])?;
    run_git(project_root.path(), &["commit", "-m", "initial"])?;
    let create_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Check project health".to_string(),
            objective: "Run the first Codex-owned gstack health capability.".to_string(),
            kind: DevflowTaskKind::Diagnostic,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;

    let capability_run_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("health".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let capability_run_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(capability_run_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(capability_run_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("gstack health completed"));
    let artifact = artifact.expect("gstack health should produce an artifact");
    assert_eq!(artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(artifact.task_id, task.id);
    let health_artifact_id = artifact.id.clone();

    let artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: health_artifact_id.clone(),
        })
        .await?;
    let artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(artifact_read_response)?;
    let report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(report["runner"], "codex-owned-pack-runner");
    assert_eq!(report["capability"], "health");
    assert!(report["dimensions"].as_array().is_some_and(|dimensions| {
        dimensions
            .iter()
            .any(|dimension| dimension["name"].as_str() == Some("manifestInventory"))
    }));
    assert!(
        report["policy"]["commandAllowlist"]
            .as_array()
            .is_some_and(|allowlist| allowlist
                .iter()
                .any(|command| command.as_str() == Some("git diff --check")))
    );
    let commands = report["commands"].as_array().expect("commands array");
    assert!(commands.iter().any(|command| {
        command["command"].as_str() == Some("git status --short -- . :(exclude).codex")
            && command["status"].as_str() == Some("completed")
    }));
    assert!(commands.iter().any(|command| {
        command["command"].as_str() == Some("git diff --check")
            && command["status"].as_str() == Some("completed")
    }));

    let browse_qa_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("browseQa".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let browse_qa_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(browse_qa_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(browse_qa_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("gstack browseQa completed"));
    let browse_qa_artifact = artifact.expect("gstack browseQa should produce an artifact");
    assert_eq!(browse_qa_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(browse_qa_artifact.task_id, task.id);
    let browse_qa_artifact_id = browse_qa_artifact.id.clone();

    let browse_qa_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: browse_qa_artifact_id.clone(),
        })
        .await?;
    let browse_qa_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            browse_qa_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } =
        to_response(browse_qa_artifact_read_response)?;
    let browse_qa_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(browse_qa_report["runner"], "codex-owned-pack-runner");
    assert_eq!(browse_qa_report["capability"], "browseQa");
    assert_eq!(
        browse_qa_report["policy"]["browserDaemon"]["owner"].as_str(),
        Some("gstack-browse-cli")
    );
    assert_eq!(
        browse_qa_report["selectedTargetUrl"].as_str(),
        Some("http://127.0.0.1:5173")
    );
    assert!(
        browse_qa_report["dimensions"]
            .as_array()
            .is_some_and(|dimensions| dimensions.iter().any(|dimension| {
                dimension["name"].as_str() == Some("webSurfaceInventory")
                    && dimension["status"].as_str() == Some("completed")
            }))
    );
    let target_candidates = browse_qa_report["targetCandidates"]
        .as_array()
        .expect("target candidates array");
    assert!(target_candidates.iter().any(|candidate| {
        candidate["kind"].as_str() == Some("command")
            && candidate["value"].as_str() == Some("npm run dev")
    }));
    assert!(target_candidates.iter().any(|candidate| {
        candidate["kind"].as_str() == Some("url")
            && candidate["value"].as_str() == Some("http://127.0.0.1:5173")
    }));
    let browser_commands = browse_qa_report["browserCommands"]
        .as_array()
        .expect("browser commands array");
    assert!(browser_commands.iter().any(|command| {
        command["command"].as_str() == Some("browse goto http://127.0.0.1:5173")
            && command["status"].as_str() == Some("completed")
    }));
    assert!(browser_commands.iter().any(|command| {
        command["command"].as_str().is_some_and(|value| {
            value.starts_with("browse screenshot ")
                && command["status"].as_str() == Some("completed")
        })
    }));
    assert_eq!(
        browse_qa_report["screenshotArtifact"]["status"].as_str(),
        Some("captured")
    );
    assert_eq!(
        browse_qa_report["screenshotArtifact"]["mimeType"].as_str(),
        Some("image/png")
    );
    assert!(
        browse_qa_report["screenshotArtifact"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("-browseQa-screenshot.png"))
    );

    std::fs::write(
        project_root.path().join("README.md"),
        "healthy\nreview me\n",
    )?;
    let review_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("review".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let review_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(review_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(review_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("gstack review completed"));
    let review_artifact = artifact.expect("gstack review should produce an artifact");
    assert_eq!(review_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(review_artifact.task_id, task.id);
    let review_artifact_id = review_artifact.id.clone();

    let review_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: review_artifact_id.clone(),
        })
        .await?;
    let review_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(review_artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(review_artifact_read_response)?;
    let review_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(review_report["runner"], "codex-owned-pack-runner");
    assert_eq!(review_report["capability"], "review");
    assert_eq!(
        review_report["policy"]["reviewMode"].as_str(),
        Some("static_diff_intake")
    );
    assert!(
        review_report["changedFiles"]
            .as_array()
            .is_some_and(|files| files.iter().any(|file| file.as_str() == Some("README.md")))
    );
    assert!(
        review_report["dimensions"]
            .as_array()
            .is_some_and(|dimensions| dimensions.iter().any(|dimension| {
                dimension["name"].as_str() == Some("diffInventory")
                    && dimension["status"].as_str() == Some("completed")
            }))
    );
    assert!(
        review_report["commands"]
            .as_array()
            .is_some_and(|commands| commands.iter().any(|command| {
                command["command"].as_str() == Some("git diff --check")
                    && command["status"].as_str() == Some("completed")
            }))
    );

    let benchmark_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("benchmark".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let benchmark_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(benchmark_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(benchmark_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("gstack benchmark completed"));
    let benchmark_artifact = artifact.expect("gstack benchmark should produce an artifact");
    assert_eq!(benchmark_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(benchmark_artifact.task_id, task.id);
    let benchmark_artifact_id = benchmark_artifact.id.clone();

    let benchmark_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: benchmark_artifact_id.clone(),
        })
        .await?;
    let benchmark_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            benchmark_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } =
        to_response(benchmark_artifact_read_response)?;
    let benchmark_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(benchmark_report["runner"], "codex-owned-pack-runner");
    assert_eq!(benchmark_report["capability"], "benchmark");
    assert_eq!(
        benchmark_report["benchmarkType"].as_str(),
        Some("static_asset_budget")
    );
    assert!(
        benchmark_report["dimensions"]
            .as_array()
            .is_some_and(|dimensions| dimensions.iter().any(|dimension| {
                dimension["name"].as_str() == Some("totalAssetBudget")
                    && dimension["status"].as_str() == Some("completed")
            }))
    );
    assert!(
        benchmark_report["assets"]
            .as_array()
            .is_some_and(|assets| assets.iter().any(|asset| {
                asset["path"].as_str() == Some("public/index.html")
                    && asset["status"].as_str() == Some("completed")
            }))
    );

    let canary_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("canary".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let canary_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(canary_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(canary_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("gstack canary completed"));
    let canary_artifact = artifact.expect("gstack canary should produce an artifact");
    assert_eq!(canary_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(canary_artifact.task_id, task.id);
    let canary_artifact_id = canary_artifact.id.clone();

    let canary_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: canary_artifact_id.clone(),
        })
        .await?;
    let canary_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(canary_artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(canary_artifact_read_response)?;
    let canary_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(canary_report["runner"], "codex-owned-pack-runner");
    assert_eq!(canary_report["capability"], "canary");
    assert_eq!(
        canary_report["canaryType"].as_str(),
        Some("controlled_local_browser_probe")
    );
    assert_eq!(
        canary_report["policy"]["deploymentSource"]["kind"].as_str(),
        Some("detected_local_url")
    );
    assert_eq!(
        canary_report["selectedTargetUrl"].as_str(),
        Some("http://127.0.0.1:5173")
    );
    assert!(
        canary_report["dimensions"]
            .as_array()
            .is_some_and(|dimensions| dimensions.iter().any(|dimension| {
                dimension["name"].as_str() == Some("canaryProbe")
                    && dimension["status"].as_str() == Some("completed")
            }))
    );
    assert_eq!(
        canary_report["screenshotArtifact"]["status"].as_str(),
        Some("captured")
    );

    let passed_quality_gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
        })
        .await?;
    let passed_quality_gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            passed_quality_gate_list_request_id,
        )),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } =
        to_response(passed_quality_gate_list_response)?;
    assert_eq!(gates.len(), 5);
    assert!(gates.iter().any(|gate| {
        gate.kind == DevflowQualityGateKind::GstackHealth
            && gate.status == DevflowQualityGateStatus::Passed
            && gate.command == "devflowCapabilityPack/run gstack-engineering health"
            && gate.artifact_id.as_deref() == Some(health_artifact_id.as_str())
    }));
    assert!(gates.iter().any(|gate| {
        gate.kind == DevflowQualityGateKind::GstackBrowserQa
            && gate.status == DevflowQualityGateStatus::Passed
            && gate.command == "devflowCapabilityPack/run gstack-engineering browseQa"
            && gate.artifact_id.as_deref() == Some(browse_qa_artifact_id.as_str())
    }));
    assert!(gates.iter().any(|gate| {
        gate.kind == DevflowQualityGateKind::Review
            && gate.status == DevflowQualityGateStatus::Passed
            && gate.command == "devflowCapabilityPack/run gstack-engineering review"
            && gate.artifact_id.as_deref() == Some(review_artifact_id.as_str())
    }));
    assert!(gates.iter().any(|gate| {
        gate.kind == DevflowQualityGateKind::GstackBenchmark
            && gate.status == DevflowQualityGateStatus::Passed
            && gate.command == "devflowCapabilityPack/run gstack-engineering benchmark"
            && gate.artifact_id.as_deref() == Some(benchmark_artifact_id.as_str())
    }));
    assert!(gates.iter().any(|gate| {
        gate.kind == DevflowQualityGateKind::GstackCanary
            && gate.status == DevflowQualityGateStatus::Passed
            && gate.command == "devflowCapabilityPack/run gstack-engineering canary"
            && gate.artifact_id.as_deref() == Some(canary_artifact_id.as_str())
    }));

    let watchdog_read_request_id = mcp
        .send_devflow_watchdog_read_request(DevflowWatchdogReadParams {})
        .await?;
    let watchdog_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_read_request_id)),
    )
    .await??;
    let DevflowWatchdogReadResponse { status, alerts, .. } = to_response(watchdog_read_response)?;
    assert_eq!(status, DevflowWatchdogStatus::Idle);
    assert_eq!(alerts, Vec::new());

    std::fs::write(project_root.path().join("DIRTY.md"), "uncommitted\n")?;
    let failed_capability_run_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("health".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let failed_capability_run_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            failed_capability_run_request_id,
        )),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(failed_capability_run_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Failed);
    assert!(summary.contains("gstack health failed"));
    let failed_artifact = artifact.expect("failed gstack health should produce an artifact");

    let alert_created_notification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowWatchdog/alertCreated"),
    )
    .await??;
    let alert_created: DevflowWatchdogAlertCreatedNotification = serde_json::from_value(
        alert_created_notification
            .params
            .expect("watchdog alert params"),
    )?;
    assert_eq!(
        alert_created.project_id,
        project_root.path().display().to_string()
    );
    assert_eq!(
        alert_created.alert.task_id.as_deref(),
        Some(task.id.as_str())
    );
    assert_eq!(
        alert_created.alert.run_id.as_deref(),
        Some(failed_artifact.run_id.as_str())
    );
    assert_eq!(
        alert_created.alert.severity,
        DevflowWatchdogAlertSeverity::Warning
    );
    assert!(alert_created.alert.message.contains("gstack health failed"));

    let quality_gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
        })
        .await?;
    let quality_gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(quality_gate_list_request_id)),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } = to_response(quality_gate_list_response)?;
    assert_eq!(gates.len(), 6);
    let failed_health_gate = gates
        .iter()
        .find(|gate| {
            gate.kind == DevflowQualityGateKind::GstackHealth
                && gate.status == DevflowQualityGateStatus::Failed
        })
        .expect("failed gstack health gate");
    assert_eq!(
        failed_health_gate.command,
        "git status --short -- . :(exclude).codex"
    );
    assert_eq!(
        failed_health_gate.artifact_id.as_deref(),
        Some(failed_artifact.id.as_str())
    );
    assert_eq!(failed_health_gate.run_id, failed_artifact.run_id);

    let watchdog_read_request_id = mcp
        .send_devflow_watchdog_read_request(DevflowWatchdogReadParams {})
        .await?;
    let watchdog_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_read_request_id)),
    )
    .await??;
    let DevflowWatchdogReadResponse { status, alerts, .. } = to_response(watchdog_read_response)?;
    assert_eq!(status, DevflowWatchdogStatus::NoProgress);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].severity, DevflowWatchdogAlertSeverity::Warning);
    assert!(alerts[0].message.contains("gstack health failed"));

    let watchdog_alerts_request_id = mcp
        .send_devflow_watchdog_alerts_request(DevflowWatchdogAlertsParams {
            status: Some(DevflowWatchdogStatus::NoProgress),
            severity: Some(DevflowWatchdogAlertSeverity::Warning),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let watchdog_alerts_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_alerts_request_id)),
    )
    .await??;
    let DevflowWatchdogAlertsResponse { data, next_cursor } =
        to_response(watchdog_alerts_response)?;
    assert_eq!(data, alerts);
    assert_eq!(next_cursor, None);

    let watchdog_queue_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("watchdogQueue".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let watchdog_queue_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_queue_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(watchdog_queue_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("gstack watchdogQueue completed"));
    assert!(summary.contains("1 no-progress"));
    let watchdog_queue_artifact =
        artifact.expect("gstack watchdogQueue should produce a queue artifact");
    assert_eq!(watchdog_queue_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(watchdog_queue_artifact.task_id, task.id);

    let watchdog_queue_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: watchdog_queue_artifact.id,
        })
        .await?;
    let watchdog_queue_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            watchdog_queue_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } =
        to_response(watchdog_queue_artifact_read_response)?;
    let watchdog_queue_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(watchdog_queue_report["runner"], "codex-owned-pack-runner");
    assert_eq!(watchdog_queue_report["capability"], "watchdogQueue");
    assert_eq!(
        watchdog_queue_report["queueStatus"].as_str(),
        Some("no_progress")
    );
    assert_eq!(
        watchdog_queue_report["queue"]["status"].as_str(),
        Some("no_progress")
    );
    assert_eq!(
        watchdog_queue_report["queue"]["counts"]["noProgress"].as_u64(),
        Some(1)
    );
    assert_eq!(
        watchdog_queue_report["queue"]["counts"]["timedOut"].as_u64(),
        Some(0)
    );
    assert!(
        watchdog_queue_report["queue"]["noProgress"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| {
                item["taskId"].as_str() == Some(task.id.as_str())
                    && item["runId"].as_str() == Some(failed_artifact.run_id.as_str())
                    && item["alertSeverity"].as_str() == Some("warning")
                    && item["reason"]
                        .as_str()
                        .is_some_and(|reason| reason.contains("gstack health failed"))
            }))
    );
    assert!(
        watchdog_queue_report["dimensions"]
            .as_array()
            .is_some_and(|dimensions| dimensions.iter().any(|dimension| {
                dimension["name"].as_str() == Some("noProgressQueue")
                    && dimension["status"].as_str() == Some("failed")
            }))
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_browse_qa_failure_creates_gate_and_watchdog_alert() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let project_root = TempDir::new()?;
    std::fs::write(
        project_root.path().join("package.json"),
        r#"{"scripts":{"dev":"vite --host 127.0.0.1 --port 5173"}}"#,
    )?;
    std::fs::create_dir_all(project_root.path().join("public"))?;
    std::fs::write(
        project_root.path().join("public").join("index.html"),
        "<h1>needs browser QA</h1>\n",
    )?;
    write_fake_browse_cli(
        project_root.path(),
        r#"#!/bin/sh
printf '%s\n' "$*" >> .codex/fake-browse-failure.log
if [ "$1" = "screenshot" ]; then
  echo "screenshot failed" >&2
  exit 42
fi
"#,
    )?;

    let create_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Run browser QA".to_string(),
            objective: "Capture browser QA evidence for the local frontend.".to_string(),
            kind: DevflowTaskKind::Diagnostic,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;

    let browse_qa_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("browseQa".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let browse_qa_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(browse_qa_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(browse_qa_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Failed);
    assert!(summary.contains("gstack browseQa failed"));
    let failed_artifact = artifact.expect("failed browseQa should produce an artifact");

    let artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: failed_artifact.id.clone(),
        })
        .await?;
    let artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(artifact_read_response)?;
    let report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(
        report["screenshotArtifact"]["status"].as_str(),
        Some("missing")
    );
    assert!(
        report["browserCommands"]
            .as_array()
            .is_some_and(|commands| {
                commands.iter().any(|command| {
                    command["command"]
                        .as_str()
                        .is_some_and(|value| value.starts_with("browse screenshot "))
                        && command["status"].as_str() == Some("failed")
                        && command["exitCode"].as_i64() == Some(42)
                })
            })
    );

    let alert_created_notification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowWatchdog/alertCreated"),
    )
    .await??;
    let alert_created: DevflowWatchdogAlertCreatedNotification = serde_json::from_value(
        alert_created_notification
            .params
            .expect("watchdog alert params"),
    )?;
    assert_eq!(
        alert_created.alert.severity,
        DevflowWatchdogAlertSeverity::Warning
    );
    assert_eq!(
        alert_created.alert.run_id.as_deref(),
        Some(failed_artifact.run_id.as_str())
    );
    assert!(
        alert_created
            .alert
            .message
            .contains("gstack browseQa failed")
    );

    let quality_gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
        })
        .await?;
    let quality_gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(quality_gate_list_request_id)),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } = to_response(quality_gate_list_response)?;
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0].kind, DevflowQualityGateKind::GstackBrowserQa);
    assert_eq!(gates[0].status, DevflowQualityGateStatus::Failed);
    assert!(gates[0].command.starts_with("browse screenshot "));
    assert_eq!(gates[0].exit_code, Some(42));
    assert_eq!(
        gates[0].artifact_id.as_deref(),
        Some(failed_artifact.id.as_str())
    );
    Ok(())
}

#[tokio::test]
async fn devflow_benchmark_failure_creates_gate_and_watchdog_alert() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let project_root = TempDir::new()?;
    std::fs::create_dir_all(project_root.path().join("public"))?;
    std::fs::write(
        project_root.path().join("public").join("bundle.js"),
        vec![b'a'; 600 * 1024],
    )?;

    let create_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Run benchmark".to_string(),
            objective: "Check local static asset budgets.".to_string(),
            kind: DevflowTaskKind::Diagnostic,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;

    let benchmark_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("benchmark".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let benchmark_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(benchmark_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(benchmark_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Failed);
    assert!(summary.contains("gstack benchmark failed"));
    let failed_artifact = artifact.expect("failed benchmark should produce an artifact");

    let artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: failed_artifact.id.clone(),
        })
        .await?;
    let artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(artifact_read_response)?;
    let report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(report["capability"], "benchmark");
    assert_eq!(report["totals"]["violationCount"].as_u64(), Some(1));
    assert!(
        report["assets"]
            .as_array()
            .is_some_and(|assets| assets.iter().any(|asset| {
                asset["path"].as_str() == Some("public/bundle.js")
                    && asset["status"].as_str() == Some("failed")
            }))
    );

    let alert_created_notification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowWatchdog/alertCreated"),
    )
    .await??;
    let alert_created: DevflowWatchdogAlertCreatedNotification = serde_json::from_value(
        alert_created_notification
            .params
            .expect("watchdog alert params"),
    )?;
    assert_eq!(
        alert_created.alert.severity,
        DevflowWatchdogAlertSeverity::Warning
    );
    assert!(
        alert_created
            .alert
            .message
            .contains("gstack benchmark failed")
    );

    let quality_gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
        })
        .await?;
    let quality_gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(quality_gate_list_request_id)),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } = to_response(quality_gate_list_response)?;
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0].kind, DevflowQualityGateKind::GstackBenchmark);
    assert_eq!(gates[0].status, DevflowQualityGateStatus::Failed);
    assert_eq!(gates[0].command, "static asset budget check");
    assert_eq!(
        gates[0].artifact_id.as_deref(),
        Some(failed_artifact.id.as_str())
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_canary_failure_creates_gate_and_watchdog_alert() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let project_root = TempDir::new()?;
    std::fs::write(
        project_root.path().join("package.json"),
        r#"{"scripts":{"dev":"vite --host 127.0.0.1 --port 5173"}}"#,
    )?;
    std::fs::create_dir_all(project_root.path().join("public"))?;
    std::fs::write(
        project_root.path().join("public").join("index.html"),
        "<h1>needs canary</h1>\n",
    )?;
    write_fake_browse_cli(
        project_root.path(),
        r#"#!/bin/sh
printf '%s\n' "$*" >> .codex/fake-canary-failure.log
if [ "$1" = "screenshot" ]; then
  echo "canary screenshot failed" >&2
  exit 43
fi
"#,
    )?;

    let create_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Run canary".to_string(),
            objective: "Probe the local frontend canary target.".to_string(),
            kind: DevflowTaskKind::Diagnostic,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;

    let canary_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("canary".to_string()),
            task_id: Some(task.id.clone()),
            project_root: None,
        })
        .await?;
    let canary_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(canary_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(canary_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Failed);
    assert!(summary.contains("gstack canary failed"));
    let failed_artifact = artifact.expect("failed canary should produce an artifact");

    let artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: failed_artifact.id.clone(),
        })
        .await?;
    let artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(artifact_read_response)?;
    let report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(report["capability"], "canary");
    assert_eq!(
        report["screenshotArtifact"]["status"].as_str(),
        Some("missing")
    );
    assert!(
        report["browserCommands"]
            .as_array()
            .is_some_and(|commands| commands.iter().any(|command| {
                command["command"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("browse screenshot "))
                    && command["status"].as_str() == Some("failed")
                    && command["exitCode"].as_i64() == Some(43)
            }))
    );

    let alert_created_notification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowWatchdog/alertCreated"),
    )
    .await??;
    let alert_created: DevflowWatchdogAlertCreatedNotification = serde_json::from_value(
        alert_created_notification
            .params
            .expect("watchdog alert params"),
    )?;
    assert_eq!(
        alert_created.alert.severity,
        DevflowWatchdogAlertSeverity::Warning
    );
    assert!(alert_created.alert.message.contains("gstack canary failed"));

    let quality_gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
        })
        .await?;
    let quality_gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(quality_gate_list_request_id)),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } = to_response(quality_gate_list_response)?;
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0].kind, DevflowQualityGateKind::GstackCanary);
    assert_eq!(gates[0].status, DevflowQualityGateStatus::Failed);
    assert!(gates[0].command.starts_with("browse screenshot "));
    assert_eq!(gates[0].exit_code, Some(43));
    assert_eq!(
        gates[0].artifact_id.as_deref(),
        Some(failed_artifact.id.as_str())
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_legacy_agent_diagnose_runs_hermes_doctor() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let cli_root = TempDir::new()?;
    let (script_path, args_path, cwd_path) =
        write_fake_hermes_cli(cli_root.path(), "Hermes doctor ok\nAll systems ready.")?;
    let script_path = script_path.to_string_lossy().into_owned();

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_HERMES_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_devflow_agent_diagnose_request(DevflowAgentDiagnoseParams {
            id: "hermes-automation".to_string(),
            cwd: Some(project_root.path().display().to_string()),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let DevflowAgentDiagnoseResponse {
        agent,
        command,
        ok,
        exit_code,
        stdout,
        stderr,
    } = to_response(response)?;

    assert_eq!(agent.id, "hermes-automation");
    assert_eq!(command, "hermes doctor");
    assert!(ok);
    assert_eq!(exit_code, Some(0));
    assert!(stdout.contains("Hermes doctor ok"));
    assert!(stderr.is_empty());
    assert_eq!(std::fs::read_to_string(args_path)?, "doctor\n");
    assert!(
        std::fs::read_to_string(cwd_path)?
            .contains(project_root.path().to_str().expect("utf-8 path"))
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_legacy_claude_report_task_creates_report_artifact() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let cli_root = TempDir::new()?;
    let (script_path, args_path, cwd_path) = write_fake_claude_cli(
        cli_root.path(),
        "# Release Summary\n\n- Completed a report task.\n- No code changes were made.",
    )?;
    let script_path = script_path.to_string_lossy().into_owned();

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_CLAUDE_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Summarize rollout".to_string(),
            objective: "Write a short release summary for the current project.".to_string(),
            kind: DevflowTaskKind::Report,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("legacy:manual".to_string()),
            dependencies: None,
            assigned_agent_id: Some("claude-writer".to_string()),
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    assert_eq!(task.trigger_source.as_deref(), Some("legacy:manual"));
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { task, run } = to_response(start_response)?;
    assert_eq!(run.agent_id, "claude-writer");
    assert_eq!(run.status, DevflowRunStatus::Running);
    assert_eq!(run.thread_id, None);
    assert_eq!(run.turn_id, None);
    assert_eq!(task.status, DevflowTaskStatus::Running);

    let report_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::Report
                && payload.artifact.run_id == run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;

    let ready_run_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyForReview {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(ready_run_notification.task_id, task.id);

    let ready_task_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowTask/statusChanged")
                .await?;
            let payload: DevflowTaskStatusChangedNotification =
                serde_json::from_value(notification.params.expect("task status params"))?;
            if payload.task.id == task.id
                && payload.task.status == DevflowTaskStatus::ReadyForReview
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(ready_task_notification.task.id, task.id);

    let prompt_args = std::fs::read_to_string(args_path)?;
    let execution_cwd = std::fs::read_to_string(cwd_path)?;
    let report_body = std::fs::read_to_string(&report_artifact_created.path)?;
    assert!(prompt_args.contains("Context Pack"));
    assert!(prompt_args.contains("Summarize rollout"));
    assert!(execution_cwd.contains(project_root.path().to_str().expect("utf-8 path")));
    assert!(report_body.contains("Release Summary"));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_legacy_report_task_requires_explicit_migration_trigger_source() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Summarize rollout".to_string(),
            objective: "Write a short release summary for the current project.".to_string(),
            kind: DevflowTaskKind::Report,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: Some("claude-writer".to_string()),
        })
        .await?;
    let create_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    assert_eq!(create_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        create_error
            .error
            .message
            .contains("explicit migration triggerSource"),
        "unexpected create error: {}",
        create_error.error.message
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_legacy_agent_assignment_requires_explicit_migration_trigger_source() -> Result<()>
{
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Plain report task".to_string(),
            objective: "Create a normal report task first.".to_string(),
            kind: DevflowTaskKind::Report,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;
    assert!(task.trigger_source.is_none());

    let assign_request_id = mcp
        .send_devflow_task_assign_request(DevflowTaskAssignParams {
            id: task.id.clone(),
            assigned_agent_id: Some("claude-writer".to_string()),
        })
        .await?;
    let assign_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(assign_request_id)),
    )
    .await??;
    assert_eq!(assign_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        assign_error
            .error
            .message
            .contains("explicit migration triggerSource"),
        "unexpected assign error: {}",
        assign_error.error.message
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_legacy_hermes_triggered_report_task_preserves_trigger_source() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let cli_root = TempDir::new()?;
    let (script_path, _, _) = write_fake_claude_cli(
        cli_root.path(),
        "# Hermes Triggered Report\n\n- Triggered from Hermes cron.",
    )?;
    let script_path = script_path.to_string_lossy().into_owned();

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_CLAUDE_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let memory_request_id = mcp
        .send_devflow_project_memory_write_request(DevflowProjectMemoryWriteParams {
            project_root: project_root.path().display().to_string(),
            summary: "Known decision: Hermes cron reports should include deployment context."
                .to_string(),
        })
        .await?;
    let memory_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(memory_request_id)),
    )
    .await??;
    let DevflowProjectMemoryWriteResponse { summary, .. } = to_response(memory_response)?;
    assert!(summary.contains("Hermes cron reports"));

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Hermes cron report".to_string(),
            objective: "Summarize the latest automated check.".to_string(),
            kind: DevflowTaskKind::Report,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("hermes:cron".to_string()),
            dependencies: None,
            assigned_agent_id: Some("claude-writer".to_string()),
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    assert_eq!(task.trigger_source.as_deref(), Some("hermes:cron"));
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;
    assert_eq!(run.agent_id, "claude-writer");

    let context_pack_id = run
        .artifact_ids
        .first()
        .cloned()
        .expect("context pack artifact id");
    let context_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: context_pack_id,
        })
        .await?;
    let context_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(context_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact,
        contents: context_contents,
    } = to_response(context_response)?;
    assert_eq!(artifact.kind, DevflowArtifactKind::ContextPack);
    let context_pack: serde_json::Value = serde_json::from_str(&context_contents)?;
    assert_eq!(
        context_pack.get("triggerSource"),
        Some(&serde_json::json!("hermes:cron"))
    );
    assert!(
        context_pack
            .pointer("/projectMemory/path")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|path| path.ends_with(".codex/devflow/project-memory.md"))
    );
    assert_eq!(
        context_pack
            .pointer("/projectMemory/summary")
            .and_then(serde_json::Value::as_str),
        Some("Known decision: Hermes cron reports should include deployment context.")
    );

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyForReview {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: task.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;
    assert_eq!(read_task.trigger_source.as_deref(), Some("hermes:cron"));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_legacy_automation_task_runs_hermes_doctor_and_records_trigger_source() -> Result<()>
{
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let cli_root = TempDir::new()?;
    let (script_path, args_path, cwd_path) = write_fake_hermes_cli(
        cli_root.path(),
        "# Hermes Automation Report\n\n- Hermes doctor completed successfully.",
    )?;
    let script_path = script_path.to_string_lossy().into_owned();

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_HERMES_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Hermes doctor automation".to_string(),
            objective: "Run hermes doctor and summarize the result.".to_string(),
            kind: DevflowTaskKind::Automation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("hermes:manual".to_string()),
            dependencies: None,
            assigned_agent_id: Some("hermes-automation".to_string()),
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { task, run } = to_response(start_response)?;
    assert_eq!(task.status, DevflowTaskStatus::Running);
    assert_eq!(run.agent_id, "hermes-automation");
    assert_eq!(run.thread_id, None);
    assert_eq!(run.turn_id, None);

    let command_started = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/commandStarted")
                .await?;
            let payload: DevflowRunCommandStartedNotification =
                serde_json::from_value(notification.params.expect("command started params"))?;
            if payload.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(command_started.command, "hermes doctor");

    let output_delta = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/outputDelta")
                .await?;
            let payload: DevflowRunOutputDeltaNotification =
                serde_json::from_value(notification.params.expect("output delta params"))?;
            if payload.run_id == run.id
                && payload.source == DevflowRunOutputSource::CommandExecution
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert!(output_delta.delta.contains("Hermes Automation Report"));

    let command_completed = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/commandCompleted")
                .await?;
            let payload: DevflowRunCommandCompletedNotification =
                serde_json::from_value(notification.params.expect("command completed params"))?;
            if payload.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(command_completed.status, "completed");

    let report_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::Report
                && payload.artifact.run_id == run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyForReview {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowTask/statusChanged")
                .await?;
            let payload: DevflowTaskStatusChangedNotification =
                serde_json::from_value(notification.params.expect("task status params"))?;
            if payload.task.id == task.id
                && payload.task.status == DevflowTaskStatus::ReadyForReview
            {
                return Ok::<_, anyhow::Error>(payload.task);
            }
        }
    })
    .await??;

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: task.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;
    assert_eq!(read_task.trigger_source.as_deref(), Some("hermes:manual"));
    assert_eq!(std::fs::read_to_string(args_path)?, "doctor\n");
    assert!(
        std::fs::read_to_string(cwd_path)?
            .contains(project_root.path().to_str().expect("utf-8 path"))
    );
    assert!(
        std::fs::read_to_string(&report_artifact_created.path)?
            .contains("Hermes Automation Report")
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_store_snapshot_recovers_task_run_gate_artifact_and_watchdog_after_restart()
-> Result<()> {
    let codex_home = TempDir::new()?;
    let automation_root = TempDir::new()?;
    let health_root = TempDir::new()?;
    let cli_root = TempDir::new()?;
    let (script_path, _, _) = write_fake_hermes_cli(
        cli_root.path(),
        "# Persisted Hermes Report\n\n- Restart recovery should keep this report.",
    )?;
    let script_path = script_path.to_string_lossy().into_owned();

    init_git_repo(health_root.path())?;
    std::fs::write(health_root.path().join("README.md"), "healthy\n")?;
    run_git(health_root.path(), &["add", "README.md"])?;
    run_git(health_root.path(), &["commit", "-m", "initial"])?;
    std::fs::write(health_root.path().join("DIRTY.md"), "uncommitted\n")?;

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_HERMES_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_automation_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: automation_root.path().display().to_string(),
            title: "Persist automation task".to_string(),
            objective: "Run hermes doctor and produce a report that survives app-server restart."
                .to_string(),
            kind: DevflowTaskKind::Automation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("hermes:manual".to_string()),
            dependencies: None,
            assigned_agent_id: Some("hermes-automation".to_string()),
        })
        .await?;
    let create_automation_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_automation_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse {
        task: automation_task,
    } = to_response(create_automation_response)?;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: automation_task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse {
        run: automation_run,
        ..
    } = to_response(start_response)?;

    let automation_report_artifact = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::Report
                && payload.artifact.run_id == automation_run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == automation_run.id
                && payload.run.status == DevflowRunStatus::ReadyForReview
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let create_health_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: health_root.path().display().to_string(),
            title: "Persist health gate".to_string(),
            objective: "Create a failed capability gate that survives restart.".to_string(),
            kind: DevflowTaskKind::Diagnostic,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_health_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_health_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task: health_task } = to_response(create_health_response)?;

    let capability_run_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("health".to_string()),
            task_id: Some(health_task.id.clone()),
            project_root: None,
        })
        .await?;
    let capability_run_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(capability_run_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        artifact: health_artifact,
        ..
    } = to_response(capability_run_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Failed);
    let health_artifact = health_artifact.expect("failed gstack health should produce an artifact");

    let alert_created_notification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowWatchdog/alertCreated"),
    )
    .await??;
    let alert_created: DevflowWatchdogAlertCreatedNotification =
        serde_json::from_value(alert_created_notification.params.expect("watchdog params"))?;

    let gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(health_task.id.clone()),
            run_id: Some(health_artifact.run_id.clone()),
        })
        .await?;
    let gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(gate_list_request_id)),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } = to_response(gate_list_response)?;
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0].status, DevflowQualityGateStatus::Failed);
    let failed_gate = gates[0].clone();

    let snapshot_path = codex_home
        .path()
        .join("devflow")
        .join("store")
        .join("state.json");
    let snapshot: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&snapshot_path)?)?;
    assert_eq!(snapshot["schemaVersion"], 1);
    assert!(
        snapshot["tasks"]
            .as_array()
            .is_some_and(|tasks| tasks.len() >= 2)
    );
    assert!(
        snapshot["runs"]
            .as_array()
            .is_some_and(|runs| runs.len() >= 2)
    );
    assert!(
        snapshot["qualityGates"]
            .as_array()
            .is_some_and(|quality_gates| quality_gates.len() == 1)
    );
    assert!(
        snapshot["artifacts"]
            .as_array()
            .is_some_and(|artifacts| artifacts.len() >= 2)
    );

    drop(mcp);

    let mut restored_mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_HERMES_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, restored_mcp.initialize()).await??;

    let read_automation_request_id = restored_mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: automation_task.id.clone(),
        })
        .await?;
    let read_automation_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(read_automation_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse {
        task: restored_automation_task,
    } = to_response(read_automation_response)?;
    assert_eq!(
        restored_automation_task.status,
        DevflowTaskStatus::ReadyForReview
    );
    assert!(
        restored_automation_task
            .run_ids
            .contains(&automation_run.id)
    );
    assert!(
        restored_automation_task
            .artifact_ids
            .contains(&automation_report_artifact.id)
    );

    let read_report_request_id = restored_mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: automation_report_artifact.id.clone(),
        })
        .await?;
    let read_report_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_response_message(RequestId::Integer(read_report_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: restored_report_artifact,
        contents: report_contents,
    } = to_response(read_report_response)?;
    assert_eq!(restored_report_artifact, automation_report_artifact);
    assert!(report_contents.contains("Persisted Hermes Report"));

    let read_health_request_id = restored_mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: health_task.id.clone(),
        })
        .await?;
    let read_health_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_response_message(RequestId::Integer(read_health_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse {
        task: restored_health_task,
    } = to_response(read_health_response)?;
    assert_eq!(restored_health_task.status, DevflowTaskStatus::Failed);
    assert!(
        restored_health_task
            .run_ids
            .contains(&health_artifact.run_id)
    );
    assert!(
        restored_health_task
            .artifact_ids
            .contains(&health_artifact.id)
    );

    let read_gate_request_id = restored_mcp
        .send_devflow_quality_gate_read_request(DevflowQualityGateReadParams {
            id: failed_gate.id.clone(),
        })
        .await?;
    let read_gate_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_response_message(RequestId::Integer(read_gate_request_id)),
    )
    .await??;
    let DevflowQualityGateReadResponse {
        gate: restored_gate,
    } = to_response(read_gate_response)?;
    assert_eq!(restored_gate, failed_gate);

    let watchdog_read_request_id = restored_mcp
        .send_devflow_watchdog_read_request(DevflowWatchdogReadParams {})
        .await?;
    let watchdog_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(watchdog_read_request_id)),
    )
    .await??;
    let DevflowWatchdogReadResponse { alerts, .. } = to_response(watchdog_read_response)?;
    assert!(alerts.contains(&alert_created.alert));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_artifact_list_read_and_export_roundtrip() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let cli_root = TempDir::new()?;
    let (script_path, args_path, _) = write_fake_hermes_cli(
        cli_root.path(),
        "# Hermes Exportable Report\n\n- Ready for export.",
    )?;
    let script_path = script_path.to_string_lossy().into_owned();

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_HERMES_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Hermes export report".to_string(),
            objective: "Run hermes doctor and summarize the result.".to_string(),
            kind: DevflowTaskKind::Automation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("hermes:manual".to_string()),
            dependencies: None,
            assigned_agent_id: Some("hermes-automation".to_string()),
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;

    let report_artifact = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::Report
                && payload.artifact.run_id == run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;

    let list_request_id = mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            kind: Some(DevflowArtifactKind::Report),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowArtifactListResponse { data, next_cursor } = to_response(list_response)?;
    assert_eq!(data.len(), 1);
    assert_eq!(data[0].id, report_artifact.id);
    assert_eq!(next_cursor, None);

    let read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: report_artifact.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { artifact, contents } = to_response(read_response)?;
    assert_eq!(artifact.id, report_artifact.id);
    assert!(contents.contains("Hermes Exportable Report"));

    let open_request_id = mcp
        .send_devflow_artifact_open_request(DevflowArtifactOpenParams {
            id: report_artifact.id.clone(),
        })
        .await?;
    let open_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(open_request_id)),
    )
    .await??;
    let DevflowArtifactOpenResponse {
        artifact: opened_artifact,
    } = to_response(open_response)?;
    assert_eq!(opened_artifact, report_artifact);

    let export_path = project_root.path().join("exported-report.md");
    let export_request_id = mcp
        .send_devflow_artifact_export_request(DevflowArtifactExportParams {
            id: report_artifact.id.clone(),
            destination_path: export_path.display().to_string(),
        })
        .await?;
    let export_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(export_request_id)),
    )
    .await??;
    let DevflowArtifactExportResponse {
        artifact: exported_artifact,
        destination_path,
    } = to_response(export_response)?;
    assert_eq!(exported_artifact.id, report_artifact.id);
    assert_eq!(destination_path, export_path.display().to_string());
    assert!(std::fs::read_to_string(export_path)?.contains("Hermes Exportable Report"));

    let hermes_args_before_codex_delivery = std::fs::read_to_string(&args_path)?;
    let codex_deliver_request_id = mcp
        .send_devflow_artifact_deliver_request(DevflowArtifactDeliverParams {
            id: report_artifact.id.clone(),
            target_agent_id: "codex-main".to_string(),
            destination: "local:warp".to_string(),
            message: Some("Hand this report to the local Warp handoff path.".to_string()),
        })
        .await?;
    let codex_deliver_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(codex_deliver_request_id)),
    )
    .await??;
    let DevflowArtifactDeliverResponse {
        artifact: codex_delivered_artifact,
        receipt_artifact,
        approval,
        target_agent_id,
        destination,
        command,
        exit_code,
        status,
        output_summary,
        delivered_at,
    } = to_response(codex_deliver_response)?;
    assert_eq!(codex_delivered_artifact.id, report_artifact.id);
    assert_eq!(approval, None);
    let receipt_artifact = receipt_artifact.expect("local codex delivery should create a receipt");
    assert_eq!(receipt_artifact.kind, DevflowArtifactKind::DeliveryReceipt);
    assert_eq!(target_agent_id, "codex-main");
    assert_eq!(destination, "local:warp");
    assert_eq!(command, "codex artifact handoff <local-devflow-artifact>");
    assert_eq!(exit_code, Some(0));
    assert_eq!(status, DevflowArtifactDeliveryStatus::Delivered);
    assert!(output_summary.contains("Codex local artifact handoff recorded"));
    assert!(output_summary.contains("local:warp"));
    assert!(output_summary.contains(&report_artifact.path));
    assert!(output_summary.contains("Hand this report to the local Warp handoff path."));
    assert!(delivered_at.expect("codex delivered at") > 0);

    let codex_receipt = std::fs::read_to_string(&receipt_artifact.path)?;
    assert!(codex_receipt.contains("Codex output"));
    assert!(codex_receipt.contains("Destination: local:warp"));
    assert!(codex_receipt.contains("No external message was sent"));
    assert!(codex_receipt.contains(&report_artifact.path));
    assert_eq!(
        std::fs::read_to_string(&args_path)?,
        hermes_args_before_codex_delivery
    );

    let deliver_request_id = mcp
        .send_devflow_artifact_deliver_request(DevflowArtifactDeliverParams {
            id: report_artifact.id.clone(),
            target_agent_id: "hermes-automation".to_string(),
            destination: "local".to_string(),
            message: Some("Post this report to the local delivery log.".to_string()),
        })
        .await?;
    let deliver_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(deliver_request_id)),
    )
    .await??;
    let DevflowArtifactDeliverResponse {
        artifact: delivered_artifact,
        receipt_artifact,
        approval,
        target_agent_id,
        destination,
        command,
        exit_code,
        status,
        output_summary,
        delivered_at,
    } = to_response(deliver_response)?;
    assert_eq!(delivered_artifact.id, report_artifact.id);
    assert_eq!(approval, None);
    let receipt_artifact = receipt_artifact.expect("local delivery should create a receipt");
    assert_eq!(receipt_artifact.kind, DevflowArtifactKind::DeliveryReceipt);
    assert_eq!(target_agent_id, "hermes-automation");
    assert_eq!(destination, "local");
    assert_eq!(
        command,
        "hermes chat -Q --source devflow --max-turns 5 -q <devflow-artifact-delivery-prompt>"
    );
    assert_eq!(exit_code, Some(0));
    assert_eq!(status, DevflowArtifactDeliveryStatus::Delivered);
    assert!(output_summary.contains("Hermes Exportable Report"));
    assert!(delivered_at.expect("delivered at") > 0);

    let receipt = std::fs::read_to_string(&receipt_artifact.path)?;
    assert!(receipt.contains("Delivery receipt"));
    assert!(receipt.contains("Destination: local"));
    assert!(receipt.contains("Hermes Exportable Report"));

    let hermes_args = std::fs::read_to_string(&args_path)?;
    assert!(hermes_args.contains("chat"));
    assert!(hermes_args.contains("--source"));
    assert!(hermes_args.contains("devflow"));
    assert!(hermes_args.contains("--max-turns"));
    assert!(hermes_args.contains("Post this report to the local delivery log."));
    assert!(hermes_args.contains(&report_artifact.path));

    let external_deliver_request_id = mcp
        .send_devflow_artifact_deliver_request(DevflowArtifactDeliverParams {
            id: report_artifact.id.clone(),
            target_agent_id: "hermes-automation".to_string(),
            destination: "slack:#devflow".to_string(),
            message: Some("Send this report externally.".to_string()),
        })
        .await?;
    let external_deliver_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(external_deliver_request_id)),
    )
    .await??;
    let DevflowArtifactDeliverResponse {
        artifact: pending_artifact,
        receipt_artifact,
        approval,
        target_agent_id,
        destination,
        command,
        exit_code,
        status,
        output_summary,
        delivered_at,
    } = to_response(external_deliver_response)?;
    assert_eq!(pending_artifact.id, report_artifact.id);
    assert_eq!(receipt_artifact, None);
    assert_eq!(target_agent_id, "hermes-automation");
    assert_eq!(destination, "slack:#devflow");
    assert_eq!(
        command,
        "hermes chat -Q --source devflow --max-turns 5 -q <devflow-artifact-delivery-prompt>"
    );
    assert_eq!(exit_code, None);
    assert_eq!(status, DevflowArtifactDeliveryStatus::PendingApproval);
    assert!(output_summary.contains("waiting for devflow approval"));
    assert_eq!(delivered_at, None);
    let approval = approval.expect("external delivery should request approval");
    assert_eq!(
        approval.kind,
        codex_app_server_protocol::DevflowApprovalKind::ArtifactDelivery
    );
    assert_eq!(approval.task_id, task.id);
    assert_eq!(approval.run_id, run.id);
    assert_eq!(approval.file_paths, vec![report_artifact.path.clone()]);

    let approval_notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowApproval/requested"),
    )
    .await??;
    let approval_payload: DevflowApprovalRequestedNotification =
        serde_json::from_value(approval_notification.params.expect("approval params"))?;
    assert_eq!(approval_payload.approval.id, approval.id);
    assert_eq!(std::fs::read_to_string(&args_path)?, hermes_args);

    let approve_request_id = mcp
        .send_devflow_approval_respond_request(DevflowApprovalRespondParams {
            id: approval.id,
            decision: DevflowApprovalDecision::Accept,
            scope: None,
        })
        .await?;
    let approve_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(approve_request_id)),
    )
    .await??;
    let DevflowApprovalRespondResponse { approval } = to_response(approve_response)?;
    assert_eq!(approval.status, DevflowApprovalStatus::Responded);
    assert_eq!(approval.decision, Some(DevflowApprovalDecision::Accept));

    let external_receipt = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::DeliveryReceipt
                && payload.artifact.run_id == run.id
            {
                let contents = std::fs::read_to_string(&payload.artifact.path)?;
                if contents.contains("Destination: slack:#devflow") {
                    return Ok::<_, anyhow::Error>((payload.artifact, contents));
                }
            }
        }
    })
    .await??;
    assert!(external_receipt.1.contains("Destination: slack:#devflow"));
    let external_hermes_args = std::fs::read_to_string(args_path)?;
    assert!(external_hermes_args.contains("slack:#devflow"));
    assert!(external_hermes_args.contains("Send this report externally."));
    Ok(())
}

#[tokio::test]
async fn devflow_project_memory_read_and_write_roundtrip() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let read_request_id = mcp
        .send_devflow_project_memory_read_request(DevflowProjectMemoryReadParams {
            project_root: project_root.path().display().to_string(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowProjectMemoryReadResponse {
        project_id,
        path,
        summary,
    } = to_response(read_response)?;
    assert_eq!(project_id, project_root.path().display().to_string());
    assert!(path.ends_with(".codex/devflow/project-memory.md"));
    assert_eq!(summary, None);

    let write_request_id = mcp
        .send_devflow_project_memory_write_request(DevflowProjectMemoryWriteParams {
            project_root: project_root.path().display().to_string(),
            summary: "Known decision: keep Hermes automation read-only by default.".to_string(),
        })
        .await?;
    let write_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(write_request_id)),
    )
    .await??;
    let DevflowProjectMemoryWriteResponse {
        project_id,
        path,
        summary,
    } = to_response(write_response)?;
    assert_eq!(project_id, project_root.path().display().to_string());
    assert!(path.ends_with(".codex/devflow/project-memory.md"));
    assert!(summary.contains("Hermes automation"));

    let reread_request_id = mcp
        .send_devflow_project_memory_read_request(DevflowProjectMemoryReadParams {
            project_root: project_root.path().display().to_string(),
        })
        .await?;
    let reread_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(reread_request_id)),
    )
    .await??;
    let DevflowProjectMemoryReadResponse {
        summary: reread_summary,
        ..
    } = to_response(reread_response)?;
    assert_eq!(
        reread_summary.as_deref(),
        Some("Known decision: keep Hermes automation read-only by default.")
    );
    Ok(())
}

#[tokio::test]
async fn devflow_project_diagnose_reports_git_docs_and_test_commands() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(
        project_root.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )?;
    std::fs::write(project_root.path().join("README.md"), "hello\n")?;
    std::fs::write(project_root.path().join("AGENTS.md"), "instructions\n")?;
    std::fs::create_dir_all(project_root.path().join("docs"))?;
    run_git(
        project_root.path(),
        &[
            "remote",
            "add",
            "origin",
            "https://example.com/acme/demo.git",
        ],
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_devflow_project_diagnose_request(DevflowProjectDiagnoseParams {
            project_root: project_root.path().display().to_string(),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let DevflowProjectDiagnoseResponse { project } = to_response(response)?;

    assert_eq!(project.root_path, project_root.path().display().to_string());
    assert_eq!(
        project.git_remote.as_deref(),
        Some("https://example.com/acme/demo.git")
    );
    assert_eq!(project.current_branch.as_deref(), Some("main"));
    assert!(!project.is_trusted);
    assert!(project.test_commands.contains(&"cargo test".to_string()));
    assert!(
        project
            .detected_docs
            .iter()
            .any(|doc| doc.ends_with("README.md"))
    );
    assert!(
        project
            .detected_docs
            .iter()
            .any(|doc| doc.ends_with("AGENTS.md"))
    );
    assert!(
        project
            .diagnostics
            .iter()
            .any(|item| item.contains("git repository detected"))
    );

    let list_request_id = mcp
        .send_devflow_project_test_commands_list_request(DevflowProjectTestCommandsListParams {
            project_root: project_root.path().display().to_string(),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowProjectTestCommandsListResponse {
        project_id: listed_project_id,
        commands,
    } = to_response(list_response)?;
    assert_eq!(listed_project_id, project.id);
    assert!(commands.contains(&"cargo test".to_string()));

    let trust_request_id = mcp
        .send_devflow_project_trust_request(DevflowProjectTrustParams {
            project_root: project_root.path().display().to_string(),
            trusted: true,
        })
        .await?;
    let trust_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(trust_request_id)),
    )
    .await??;
    let DevflowProjectTrustResponse {
        project: trusted_project,
    } = to_response(trust_response)?;
    assert!(trusted_project.is_trusted);

    let list_request_id = mcp
        .send_devflow_project_list_request(DevflowProjectListParams {
            project_roots: Some(vec![project_root.path().display().to_string()]),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowProjectListResponse { data } = to_response(list_response)?;
    assert_eq!(data.len(), 1);
    assert_eq!(data[0].root_path, project_root.path().display().to_string());

    let read_request_id = mcp
        .send_devflow_project_read_request(DevflowProjectReadParams {
            project_root: project_root.path().display().to_string(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowProjectReadResponse {
        project: read_project,
    } = to_response(read_response)?;
    assert_eq!(
        read_project.root_path,
        project_root.path().display().to_string()
    );
    assert!(read_project.is_trusted);

    let open_request_id = mcp
        .send_devflow_project_open_request(DevflowProjectOpenParams {
            project_root: project_root.path().display().to_string(),
        })
        .await?;
    let open_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(open_request_id)),
    )
    .await??;
    let DevflowProjectOpenResponse {
        project: open_project,
    } = to_response(open_response)?;
    assert_eq!(
        open_project.root_path,
        project_root.path().display().to_string()
    );
    assert!(open_project.is_trusted);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devflow_support_bundle_create_writes_reproducible_diagnostics() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("README.md"), "support bundle\n")?;
    run_git(project_root.path(), &["add", "README.md"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let support_bundle_policy = DevflowApprovalPolicy {
        low_risk_approval_policy: AskForApproval::OnRequest,
        medium_risk_approval_policy: AskForApproval::OnFailure,
        high_risk_approval_policy: AskForApproval::OnRequest,
        approvals_reviewer: ApprovalsReviewer::User,
    };
    let update_policy_request_id = mcp
        .send_devflow_approval_policy_update_request(DevflowApprovalPolicyUpdateParams {
            policy: support_bundle_policy.clone(),
        })
        .await?;
    let update_policy_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(update_policy_request_id)),
    )
    .await??;
    let DevflowApprovalPolicyUpdateResponse { policy } = to_response(update_policy_response)?;
    assert_eq!(policy, support_bundle_policy);

    let task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Collect diagnostics".to_string(),
            objective: "Create a reproducible support bundle.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(task_response)?;

    let worktree_request_id = mcp
        .send_devflow_worktree_create_request(DevflowWorktreeCreateParams {
            task_id: task.id.clone(),
        })
        .await?;
    let worktree_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(worktree_request_id)),
    )
    .await??;
    let DevflowWorktreeCreateResponse { worktree } = to_response(worktree_response)?;
    assert_eq!(worktree.task_id, task.id);

    let bundle_request_id = mcp
        .send_devflow_support_bundle_create_request(DevflowSupportBundleCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(task.id.clone()),
        })
        .await?;
    let bundle_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(bundle_request_id)),
    )
    .await??;
    let DevflowSupportBundleCreateResponse { bundle, project } = to_response(bundle_response)?;

    assert_eq!(project.root_path, project_root.path().display().to_string());
    assert_eq!(bundle.project_id, project.id);
    assert_eq!(bundle.mime_type, "application/json");
    assert!(
        bundle
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("bundle scoped to task"))
    );
    assert!(
        bundle.diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("missing successful Integrator merge evidence")
        })
    );

    let artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.id == bundle.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(artifact_created.project_id, project.id);
    assert_eq!(artifact_created.artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(artifact_created.artifact.path, bundle.path);
    assert_eq!(artifact_created.artifact.mime_type, "application/json");

    let bundle_contents = std::fs::read_to_string(&bundle.path)?;
    let bundle_json: serde_json::Value = serde_json::from_str(&bundle_contents)?;
    assert_eq!(bundle_json["runner"], "codex-devflow-support-bundle");
    assert_eq!(
        bundle_json["approvalPolicy"],
        serde_json::to_value(&support_bundle_policy)?
    );
    assert_eq!(bundle_json["project"]["rootPath"], project.root_path);
    assert_eq!(bundle_json["counts"]["tasks"], 1);
    assert_eq!(bundle_json["counts"]["runs"], 0);
    assert_eq!(bundle_json["counts"]["worktrees"], 1);
    assert_eq!(bundle_json["tasks"][0]["id"], task.id);
    assert_eq!(
        bundle_json["releasePrep"]["integrator"]["counts"]["pending"],
        1
    );
    assert_eq!(
        bundle_json["releasePrep"]["integrator"]["tasks"][0]["status"],
        "pending_integrator_merge"
    );
    assert_eq!(
        bundle_json["releasePrep"]["reproduction"]["create"],
        "devflowReleasePrep/create"
    );
    assert_eq!(bundle_json["persistence"]["status"], "ok");
    assert!(
        bundle_json["persistence"]["storeSnapshotPath"]
            .as_str()
            .is_some_and(|path| path.ends_with("devflow/store/state.json"))
    );
    assert_eq!(
        bundle_json["persistence"]["snapshotFile"]["path"],
        bundle_json["persistence"]["storeSnapshotPath"]
    );
    assert_eq!(
        bundle_json["persistence"]["snapshotFile"]["metadataAvailable"],
        true
    );
    assert!(
        bundle_json["persistence"]["snapshotFile"]["sizeBytes"]
            .as_u64()
            .is_some_and(|size| size > 0)
    );
    assert!(
        bundle_json["persistence"]["snapshotFile"]["modifiedAt"]
            .as_i64()
            .is_some_and(|modified_at| modified_at > 0)
    );
    assert_eq!(
        bundle_json["persistence"]["snapshotFile"]["metadataError"],
        serde_json::Value::Null
    );
    assert_eq!(
        bundle_json["persistence"]["loadError"],
        serde_json::Value::Null
    );
    assert_eq!(
        bundle_json["persistence"]["persistError"],
        serde_json::Value::Null
    );
    assert_eq!(bundle_json["persistence"]["recoverableIndexes"]["tasks"], 1);
    assert_eq!(
        bundle_json["persistence"]["recoverableIndexes"]["watchdogAlerts"],
        0
    );
    assert!(
        bundle_json["persistence"]["volatileProcessState"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item == "approval grants"))
    );
    assert_eq!(
        bundle_json["persistence"]["recoverySemantics"]["activeRunsAndGates"],
        "restored fail-closed as failed/blocked records"
    );
    assert_eq!(
        bundle_json["reproduction"]["projectDiagnose"],
        "devflowProject/diagnose"
    );
    assert_eq!(
        bundle_json["reproduction"]["releasePrepCreate"],
        "devflowReleasePrep/create"
    );
    assert!(bundle_json["watchdog"]["queue"].is_object());
    assert!(bundle.path.ends_with(".json"));

    let artifact_list_request_id = mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
            kind: Some(DevflowArtifactKind::Report),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let artifact_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(artifact_list_request_id)),
    )
    .await??;
    let DevflowArtifactListResponse { data, .. } = to_response(artifact_list_response)?;
    assert!(data.iter().any(|artifact| artifact.id == bundle.id));

    drop(mcp);

    let mut restored_mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, restored_mcp.initialize()).await??;

    let restored_artifact_read_request_id = restored_mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: bundle.id.clone(),
        })
        .await?;
    let restored_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_response_message(RequestId::Integer(
            restored_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: restored_artifact,
        contents: restored_contents,
    } = to_response(restored_artifact_read_response)?;
    assert_eq!(restored_artifact.id, bundle.id);
    assert_eq!(restored_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(restored_artifact.path, bundle.path);
    assert_eq!(restored_artifact.mime_type, "application/json");
    assert!(restored_contents.contains("\"runner\": \"codex-devflow-support-bundle\""));

    let restored_artifact_list_request_id = restored_mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
            kind: Some(DevflowArtifactKind::Report),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let restored_artifact_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_response_message(RequestId::Integer(
            restored_artifact_list_request_id,
        )),
    )
    .await??;
    let DevflowArtifactListResponse { data, .. } = to_response(restored_artifact_list_response)?;
    assert!(data.iter().any(|artifact| artifact.id == bundle.id));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devflow_approval_policy_invalid_file_fails_closed_and_support_bundle_reports_it()
-> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let policy_dir = codex_home.path().join("devflow");
    std::fs::create_dir_all(&policy_dir)?;
    std::fs::write(policy_dir.join("approval-policy.json"), "{not-json")?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let read_policy_request_id = mcp
        .send_devflow_approval_policy_read_request(DevflowApprovalPolicyReadParams {})
        .await?;
    let read_policy_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(read_policy_request_id)),
    )
    .await??;
    assert_eq!(read_policy_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        read_policy_error
            .error
            .message
            .contains("failed to load devflow approval policy")
    );
    assert!(
        read_policy_error
            .error
            .message
            .contains("invalid devflow approval policy file")
    );

    let bundle_request_id = mcp
        .send_devflow_support_bundle_create_request(DevflowSupportBundleCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: None,
        })
        .await?;
    let bundle_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(bundle_request_id)),
    )
    .await??;
    let DevflowSupportBundleCreateResponse { bundle, .. } = to_response(bundle_response)?;

    let bundle_contents = std::fs::read_to_string(&bundle.path)?;
    let bundle_json: serde_json::Value = serde_json::from_str(&bundle_contents)?;
    assert!(
        bundle_json["approvalPolicyLoadError"]
            .as_str()
            .is_some_and(|error| error.contains("invalid devflow approval policy file"))
    );
    assert!(bundle_json["diagnostics"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.as_str().is_some_and(|diagnostic| {
                diagnostic.contains("approval policy could not be loaded")
            })
        })
    }));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devflow_store_snapshot_load_error_is_exported_in_support_bundle() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let store_dir = codex_home.path().join("devflow").join("store");
    std::fs::create_dir_all(&store_dir)?;
    std::fs::write(store_dir.join("state.json"), "{not-json")?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let watchdog_read_request_id = mcp
        .send_devflow_watchdog_read_request(DevflowWatchdogReadParams {})
        .await?;
    let watchdog_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_read_request_id)),
    )
    .await??;
    let DevflowWatchdogReadResponse { status, alerts, .. } = to_response(watchdog_read_response)?;
    assert_eq!(status, DevflowWatchdogStatus::Recovering);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].severity, DevflowWatchdogAlertSeverity::Critical);
    assert!(alerts[0].message.contains("store snapshot"));

    let watchdog_alerts_request_id = mcp
        .send_devflow_watchdog_alerts_request(DevflowWatchdogAlertsParams {
            status: Some(DevflowWatchdogStatus::Recovering),
            severity: Some(DevflowWatchdogAlertSeverity::Critical),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let watchdog_alerts_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_alerts_request_id)),
    )
    .await??;
    let DevflowWatchdogAlertsResponse { data, next_cursor } =
        to_response(watchdog_alerts_response)?;
    assert_eq!(data, alerts);
    assert_eq!(next_cursor, None);

    let watchdog_queue_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("watchdogQueue".to_string()),
            task_id: None,
            project_root: Some(project_root.path().display().to_string()),
        })
        .await?;
    let watchdog_queue_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_queue_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(watchdog_queue_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("status recovering"));
    assert!(summary.contains("1 recovering"));
    let watchdog_queue_artifact =
        artifact.expect("gstack watchdogQueue should produce recovering queue artifact");
    let watchdog_queue_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: watchdog_queue_artifact.id,
        })
        .await?;
    let watchdog_queue_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            watchdog_queue_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } =
        to_response(watchdog_queue_artifact_read_response)?;
    let watchdog_queue_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(
        watchdog_queue_report["queueStatus"].as_str(),
        Some("recovering")
    );
    assert_eq!(
        watchdog_queue_report["queue"]["counts"]["recovering"].as_u64(),
        Some(1)
    );
    assert!(
        watchdog_queue_report["queue"]["recovering"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| {
                item["alertSeverity"].as_str() == Some("critical")
                    && item["reason"]
                        .as_str()
                        .is_some_and(|reason| reason.contains("store snapshot"))
            }))
    );
    assert!(
        watchdog_queue_report["dimensions"]
            .as_array()
            .is_some_and(|dimensions| dimensions.iter().any(|dimension| {
                dimension["name"].as_str() == Some("recoveringQueue")
                    && dimension["status"].as_str() == Some("failed")
            }))
    );

    let bundle_request_id = mcp
        .send_devflow_support_bundle_create_request(DevflowSupportBundleCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: None,
        })
        .await?;
    let bundle_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(bundle_request_id)),
    )
    .await??;
    let DevflowSupportBundleCreateResponse { bundle, .. } = to_response(bundle_response)?;

    let bundle_contents = std::fs::read_to_string(&bundle.path)?;
    let bundle_json: serde_json::Value = serde_json::from_str(&bundle_contents)?;
    assert!(
        bundle_json["storeSnapshotLoadError"]
            .as_str()
            .is_some_and(|error| error.contains("failed to parse Devflow store snapshot"))
    );
    assert_eq!(bundle_json["persistence"]["status"], "degraded");
    assert_eq!(
        bundle_json["persistence"]["snapshotFile"]["metadataAvailable"],
        true
    );
    assert!(
        bundle_json["persistence"]["snapshotFile"]["sizeBytes"]
            .as_u64()
            .is_some_and(|size| size > 0)
    );
    assert!(
        bundle_json["persistence"]["loadError"]
            .as_str()
            .is_some_and(|error| error.contains("failed to parse Devflow store snapshot"))
    );
    assert!(bundle_json["diagnostics"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.as_str().is_some_and(|diagnostic| {
                diagnostic.contains("devflow store snapshot could not be restored")
            })
        })
    }));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devflow_store_snapshot_persist_error_is_exported_and_cleared_in_support_bundle()
-> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let devflow_dir = codex_home.path().join("devflow");

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    std::fs::create_dir_all(&devflow_dir)?;
    std::fs::write(devflow_dir.join("store"), "not a directory")?;

    let task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Persist failure diagnostics".to_string(),
            objective: "Create a task while state persistence is unavailable.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(task_response)?;
    assert_eq!(task.title, "Persist failure diagnostics");

    let watchdog_read_request_id = mcp
        .send_devflow_watchdog_read_request(DevflowWatchdogReadParams {})
        .await?;
    let watchdog_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_read_request_id)),
    )
    .await??;
    let DevflowWatchdogReadResponse { status, alerts, .. } = to_response(watchdog_read_response)?;
    assert_eq!(status, DevflowWatchdogStatus::Recovering);
    assert!(alerts.iter().any(|alert| {
        alert.severity == DevflowWatchdogAlertSeverity::Critical
            && alert
                .message
                .contains("Devflow store snapshot could not be persisted")
    }));

    let watchdog_alerts_request_id = mcp
        .send_devflow_watchdog_alerts_request(DevflowWatchdogAlertsParams {
            status: Some(DevflowWatchdogStatus::Recovering),
            severity: Some(DevflowWatchdogAlertSeverity::Critical),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let watchdog_alerts_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_alerts_request_id)),
    )
    .await??;
    let DevflowWatchdogAlertsResponse { data, next_cursor } =
        to_response(watchdog_alerts_response)?;
    assert!(data.iter().any(|alert| {
        alert
            .message
            .contains("Devflow store snapshot could not be persisted")
    }));
    assert_eq!(next_cursor, None);

    let watchdog_queue_request_id = mcp
        .send_devflow_capability_pack_run_request(DevflowCapabilityPackRunParams {
            id: "gstack-engineering".to_string(),
            capability: Some("watchdogQueue".to_string()),
            task_id: None,
            project_root: Some(project_root.path().display().to_string()),
        })
        .await?;
    let watchdog_queue_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(watchdog_queue_request_id)),
    )
    .await??;
    let DevflowCapabilityPackRunResponse {
        status,
        summary,
        artifact,
        ..
    } = to_response(watchdog_queue_response)?;
    assert_eq!(status, DevflowCapabilityPackRunStatus::Completed);
    assert!(summary.contains("status recovering"));
    assert!(summary.contains("1 recovering"));
    let watchdog_queue_artifact =
        artifact.expect("gstack watchdogQueue should produce persist-error queue artifact");
    let watchdog_queue_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: watchdog_queue_artifact.id,
        })
        .await?;
    let watchdog_queue_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            watchdog_queue_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } =
        to_response(watchdog_queue_artifact_read_response)?;
    let watchdog_queue_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(
        watchdog_queue_report["queueStatus"].as_str(),
        Some("recovering")
    );
    assert_eq!(
        watchdog_queue_report["queue"]["counts"]["recovering"].as_u64(),
        Some(1)
    );
    assert!(
        watchdog_queue_report["queue"]["recovering"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| {
                item["alertSeverity"].as_str() == Some("critical")
                    && item["reason"].as_str().is_some_and(|reason| {
                        reason.contains("Devflow store snapshot could not be persisted")
                    })
            }))
    );

    let bundle_request_id = mcp
        .send_devflow_support_bundle_create_request(DevflowSupportBundleCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(task.id.clone()),
        })
        .await?;
    let bundle_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(bundle_request_id)),
    )
    .await??;
    let DevflowSupportBundleCreateResponse { bundle, .. } = to_response(bundle_response)?;

    let bundle_contents = std::fs::read_to_string(&bundle.path)?;
    let bundle_json: serde_json::Value = serde_json::from_str(&bundle_contents)?;
    assert!(
        bundle_json["storeSnapshotPersistError"]
            .as_str()
            .is_some_and(|error| {
                error.contains("failed to create Devflow store directory")
                    || error.contains("failed to write Devflow store snapshot temp file")
            })
    );
    assert_eq!(bundle_json["persistence"]["status"], "degraded");
    assert_eq!(
        bundle_json["persistence"]["snapshotFile"]["metadataAvailable"],
        false
    );
    assert!(
        bundle_json["persistence"]["snapshotFile"]["metadataError"]
            .as_str()
            .is_some_and(|error| !error.is_empty())
    );
    assert!(
        bundle_json["persistence"]["persistError"]
            .as_str()
            .is_some_and(|error| {
                error.contains("failed to create Devflow store directory")
                    || error.contains("failed to write Devflow store snapshot temp file")
            })
    );
    assert!(bundle_json["diagnostics"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.as_str().is_some_and(|diagnostic| {
                diagnostic.contains("devflow store snapshot could not be persisted")
            })
        })
    }));
    assert!(
        bundle_json["watchdog"]["alerts"]
            .as_array()
            .is_some_and(|alerts| alerts.iter().any(|alert| {
                alert["message"].as_str().is_some_and(|message| {
                    message.contains("Devflow store snapshot could not be persisted")
                })
            }))
    );

    std::fs::remove_file(devflow_dir.join("store"))?;
    let recovery_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Persist recovery diagnostics".to_string(),
            objective: "Create a task after state persistence recovers.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let recovery_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(recovery_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse {
        task: recovery_task,
    } = to_response(recovery_task_response)?;
    assert_eq!(recovery_task.title, "Persist recovery diagnostics");

    let recovered_watchdog_read_request_id = mcp
        .send_devflow_watchdog_read_request(DevflowWatchdogReadParams {})
        .await?;
    let recovered_watchdog_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            recovered_watchdog_read_request_id,
        )),
    )
    .await??;
    let DevflowWatchdogReadResponse {
        status: recovered_watchdog_status,
        alerts: recovered_watchdog_alerts,
        ..
    } = to_response(recovered_watchdog_read_response)?;
    assert_eq!(recovered_watchdog_status, DevflowWatchdogStatus::Idle);
    assert!(recovered_watchdog_alerts.iter().all(|alert| {
        !alert
            .message
            .contains("Devflow store snapshot could not be persisted")
    }));

    let recovered_bundle_request_id = mcp
        .send_devflow_support_bundle_create_request(DevflowSupportBundleCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(recovery_task.id.clone()),
        })
        .await?;
    let recovered_bundle_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(recovered_bundle_request_id)),
    )
    .await??;
    let DevflowSupportBundleCreateResponse {
        bundle: recovered_bundle,
        ..
    } = to_response(recovered_bundle_response)?;

    let recovered_bundle_contents = std::fs::read_to_string(&recovered_bundle.path)?;
    let recovered_bundle_json: serde_json::Value =
        serde_json::from_str(&recovered_bundle_contents)?;
    assert!(recovered_bundle_json["storeSnapshotPersistError"].is_null());
    assert_eq!(recovered_bundle_json["persistence"]["status"], "ok");
    assert!(recovered_bundle_json["persistence"]["persistError"].is_null());
    assert_eq!(
        recovered_bundle_json["persistence"]["snapshotFile"]["metadataAvailable"],
        true
    );
    assert!(
        recovered_bundle_json["persistence"]["snapshotFile"]["sizeBytes"]
            .as_u64()
            .is_some_and(|size| size > 0)
    );
    assert!(
        recovered_bundle_json["diagnostics"]
            .as_array()
            .is_some_and(|items| {
                items.iter().all(|item| {
                    item.as_str().is_none_or(|diagnostic| {
                        !diagnostic.contains("devflow store snapshot could not be persisted")
                    })
                })
            })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devflow_release_prep_create_writes_finish_branch_artifacts() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("README.md"), "release prep\n")?;
    run_git(project_root.path(), &["add", "README.md"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;
    std::fs::write(project_root.path().join("feature.txt"), "new work\n")?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Prepare release".to_string(),
            objective: "Generate release prep artifacts.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(task_response)?;

    let prep_request_id = mcp
        .send_devflow_release_prep_create_request(DevflowReleasePrepCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(task.id.clone()),
        })
        .await?;
    let prep_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(prep_request_id)),
    )
    .await??;
    let DevflowReleasePrepCreateResponse {
        status,
        summary,
        blockers,
        commit_message_artifact,
        pr_body_artifact,
        release_note_artifact,
    } = to_response(prep_response)?;

    assert_eq!(status, DevflowReleasePrepStatus::Blocked);
    assert!(summary.contains("Release prep blocked"));
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker.contains("still planned"))
    );
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker.contains("no quality gate evidence"))
    );
    assert_eq!(commit_message_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(pr_body_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(release_note_artifact.kind, DevflowArtifactKind::Report);

    let commit_message = std::fs::read_to_string(&commit_message_artifact.path)?;
    assert!(commit_message.contains("devflow: Prepare release"));
    assert!(commit_message.contains("Finish branch blockers"));

    let pr_body = std::fs::read_to_string(&pr_body_artifact.path)?;
    assert!(pr_body.contains("## Persistence"));
    assert!(pr_body.contains("- Store snapshot: healthy"));
    assert!(pr_body.contains("## Finish Branch Gate"));
    assert!(pr_body.contains("feature.txt"));

    let release_notes = std::fs::read_to_string(&release_note_artifact.path)?;
    assert!(release_notes.contains("Prepare release"));

    let list_request_id = mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
            kind: Some(DevflowArtifactKind::Report),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowArtifactListResponse { data, .. } = to_response(list_response)?;
    assert_eq!(data.len(), 3);

    drop(mcp);

    let mut restored_mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, restored_mcp.initialize()).await??;

    let restored_commit_request_id = restored_mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: commit_message_artifact.id.clone(),
        })
        .await?;
    let restored_commit_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(restored_commit_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: restored_commit_artifact,
        contents: restored_commit_message,
    } = to_response(restored_commit_response)?;
    assert_eq!(restored_commit_artifact, commit_message_artifact);
    assert!(restored_commit_message.contains("devflow: Prepare release"));

    let restored_pr_body_request_id = restored_mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: pr_body_artifact.id.clone(),
        })
        .await?;
    let restored_pr_body_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(restored_pr_body_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: restored_pr_body_artifact,
        contents: restored_pr_body,
    } = to_response(restored_pr_body_response)?;
    assert_eq!(restored_pr_body_artifact, pr_body_artifact);
    assert!(restored_pr_body.contains("## Finish Branch Gate"));

    let restored_release_note_request_id = restored_mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: release_note_artifact.id.clone(),
        })
        .await?;
    let restored_release_note_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_response_message(RequestId::Integer(
            restored_release_note_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: restored_release_note_artifact,
        contents: restored_release_notes,
    } = to_response(restored_release_note_response)?;
    assert_eq!(restored_release_note_artifact, release_note_artifact);
    assert!(restored_release_notes.contains("Prepare release"));

    let restored_list_request_id = restored_mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
            kind: Some(DevflowArtifactKind::Report),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let restored_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(restored_list_request_id)),
    )
    .await??;
    let DevflowArtifactListResponse { data, .. } = to_response(restored_list_response)?;
    assert_eq!(data.len(), 3);
    assert!(
        data.iter()
            .any(|artifact| artifact.id == commit_message_artifact.id)
    );
    assert!(
        data.iter()
            .any(|artifact| artifact.id == pr_body_artifact.id)
    );
    assert!(
        data.iter()
            .any(|artifact| artifact.id == release_note_artifact.id)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devflow_release_prep_blocks_when_store_snapshot_persist_is_unhealthy() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("README.md"), "release prep\n")?;
    run_git(project_root.path(), &["add", "README.md"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Prepare release with persistence risk".to_string(),
            objective: "Generate release prep artifacts when persistence is unhealthy.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(task_response)?;

    let store_dir = codex_home.path().join("devflow").join("store");
    std::fs::remove_dir_all(&store_dir)?;
    std::fs::write(&store_dir, "not a directory")?;

    let trigger_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Trigger persistence failure".to_string(),
            objective: "Force a best-effort store snapshot write.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let trigger_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(trigger_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task: trigger_task } = to_response(trigger_response)?;
    assert_eq!(trigger_task.title, "Trigger persistence failure");

    let prep_request_id = mcp
        .send_devflow_release_prep_create_request(DevflowReleasePrepCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(task.id.clone()),
        })
        .await?;
    let prep_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(prep_request_id)),
    )
    .await??;
    let DevflowReleasePrepCreateResponse {
        status,
        blockers,
        pr_body_artifact,
        ..
    } = to_response(prep_response)?;

    assert_eq!(status, DevflowReleasePrepStatus::Blocked);
    assert!(blockers.iter().any(|blocker| {
        blocker.contains("devflow store snapshot could not be persisted")
            && blocker.contains("recent runtime indexes may not survive restart")
    }));
    let pr_body = std::fs::read_to_string(&pr_body_artifact.path)?;
    assert!(pr_body.contains("## Persistence"));
    assert!(pr_body.contains("Store snapshot persist error"));
    assert!(pr_body.contains("devflow store snapshot could not be persisted"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devflow_release_prep_blocks_when_store_snapshot_load_is_unhealthy() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("README.md"), "release prep\n")?;
    run_git(project_root.path(), &["add", "README.md"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let store_dir = codex_home.path().join("devflow").join("store");
    std::fs::create_dir_all(&store_dir)?;
    std::fs::write(store_dir.join("state.json"), "{not-json")?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Prepare release after load recovery".to_string(),
            objective: "Generate release prep artifacts after store recovery.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("test".to_string()),
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(task_response)?;

    let prep_request_id = mcp
        .send_devflow_release_prep_create_request(DevflowReleasePrepCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(task.id.clone()),
        })
        .await?;
    let prep_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(prep_request_id)),
    )
    .await??;
    let DevflowReleasePrepCreateResponse {
        status,
        blockers,
        pr_body_artifact,
        ..
    } = to_response(prep_response)?;

    assert_eq!(status, DevflowReleasePrepStatus::Blocked);
    assert!(blockers.iter().any(|blocker| {
        blocker.contains("devflow store snapshot could not be restored")
            && blocker.contains("runtime indexes may be incomplete")
    }));
    let pr_body = std::fs::read_to_string(&pr_body_artifact.path)?;
    assert!(pr_body.contains("## Persistence"));
    assert!(pr_body.contains("Store snapshot load error"));
    assert!(pr_body.contains("devflow store snapshot could not be restored"));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_automation_task_archives_large_output() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let cli_root = TempDir::new()?;
    let large_output = format!(
        "# Large Hermes Report\n{}\nARCHIVE-END\n",
        "x".repeat(4_100)
    );
    let (script_path, _, _) = write_fake_hermes_cli(cli_root.path(), &large_output)?;
    let script_path = script_path.to_string_lossy().into_owned();

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_HERMES_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Archive Hermes output".to_string(),
            objective: "Run hermes doctor with verbose output.".to_string(),
            kind: DevflowTaskKind::Automation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("hermes:manual".to_string()),
            dependencies: None,
            assigned_agent_id: Some("hermes-automation".to_string()),
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;

    let output_archive = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::OutputArchive
                && payload.artifact.run_id == run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;

    let archive_contents = std::fs::read_to_string(&output_archive.path)?;
    assert!(archive_contents.contains("# Large Hermes Report"));
    assert!(archive_contents.contains("ARCHIVE-END"));
    assert_eq!(output_archive.mime_type, "text/plain");

    let output_delta = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/outputDelta")
                .await?;
            let payload: DevflowRunOutputDeltaNotification =
                serde_json::from_value(notification.params.expect("output delta params"))?;
            if payload.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert!(output_delta.delta.contains("ARCHIVE-END"));

    let list_request_id = mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            kind: Some(DevflowArtifactKind::OutputArchive),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowArtifactListResponse { data, .. } = to_response(list_response)?;
    assert_eq!(data, vec![output_archive.clone()]);

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyForReview {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    drop(mcp);

    let mut restored_mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, restored_mcp.initialize()).await??;

    let restored_archive_request_id = restored_mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: output_archive.id.clone(),
        })
        .await?;
    let restored_archive_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(restored_archive_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: restored_archive,
        contents: restored_archive_contents,
    } = to_response(restored_archive_response)?;
    assert_eq!(restored_archive, output_archive);
    assert!(restored_archive_contents.contains("# Large Hermes Report"));
    assert!(restored_archive_contents.contains("ARCHIVE-END"));

    let restored_list_request_id = restored_mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            kind: Some(DevflowArtifactKind::OutputArchive),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let restored_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(restored_list_request_id)),
    )
    .await??;
    let DevflowArtifactListResponse { data, .. } = to_response(restored_list_response)?;
    assert_eq!(data, vec![output_archive]);

    Ok(())
}

#[tokio::test]
async fn devflow_approval_projection_reads_updates_and_responds() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let responses = vec![
        create_shell_command_sse_response_with_permissions(
            vec![
                "python3".to_string(),
                "-c".to_string(),
                "print(42)".to_string(),
            ],
            /*workdir*/ None,
            Some(5000),
            Some("require_escalated"),
            Some("Need explicit unsandboxed execution for approval projection"),
            "call1",
        )?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: approved.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let read_policy_request_id = mcp
        .send_devflow_approval_policy_read_request(DevflowApprovalPolicyReadParams {})
        .await?;
    let read_policy_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_policy_request_id)),
    )
    .await??;
    let DevflowApprovalPolicyReadResponse { policy } = to_response(read_policy_response)?;
    assert_eq!(policy.low_risk_approval_policy, AskForApproval::Never);
    assert_eq!(policy.approvals_reviewer, ApprovalsReviewer::User);

    let updated_policy = DevflowApprovalPolicy {
        low_risk_approval_policy: AskForApproval::OnRequest,
        medium_risk_approval_policy: AskForApproval::OnFailure,
        high_risk_approval_policy: AskForApproval::OnRequest,
        approvals_reviewer: ApprovalsReviewer::User,
    };
    let update_policy_request_id = mcp
        .send_devflow_approval_policy_update_request(DevflowApprovalPolicyUpdateParams {
            policy: updated_policy.clone(),
        })
        .await?;
    let update_policy_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(update_policy_request_id)),
    )
    .await??;
    let DevflowApprovalPolicyUpdateResponse { policy } = to_response(update_policy_response)?;
    assert_eq!(policy, updated_policy);

    drop(mcp);

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let restored_policy_request_id = mcp
        .send_devflow_approval_policy_read_request(DevflowApprovalPolicyReadParams {})
        .await?;
    let restored_policy_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(restored_policy_request_id)),
    )
    .await??;
    let DevflowApprovalPolicyReadResponse { policy } = to_response(restored_policy_response)?;
    assert_eq!(policy, updated_policy);

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Approval gated change".to_string(),
            objective: "Change note.txt from before to after.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;

    let notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowApproval/requested"),
    )
    .await??;
    let payload: DevflowApprovalRequestedNotification =
        serde_json::from_value(notification.params.expect("approval params"))?;
    assert_eq!(payload.approval.task_id, task.id);
    assert_eq!(payload.approval.run_id, run.id);
    assert_eq!(payload.approval.status, DevflowApprovalStatus::Pending);
    assert_eq!(
        payload.approval.kind,
        codex_app_server_protocol::DevflowApprovalKind::CommandExecution
    );
    assert!(
        payload
            .approval
            .command
            .as_deref()
            .is_some_and(|command| command.contains("python3 -c"))
    );

    let list_request_id = mcp
        .send_devflow_approval_list_request(DevflowApprovalListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            status: Some(DevflowApprovalStatus::Pending),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowApprovalListResponse { data } = to_response(list_response)?;
    assert!(
        data.iter()
            .any(|approval| approval.id == payload.approval.id)
    );

    let respond_request_id = mcp
        .send_devflow_approval_respond_request(DevflowApprovalRespondParams {
            id: payload.approval.id.clone(),
            decision: DevflowApprovalDecision::Accept,
            scope: Some(PermissionGrantScope::Turn),
        })
        .await?;
    let respond_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(respond_request_id)),
    )
    .await??;
    let DevflowApprovalRespondResponse { approval } = to_response(respond_response)?;
    assert_eq!(approval.status, DevflowApprovalStatus::Responded);
    assert_eq!(approval.decision, Some(DevflowApprovalDecision::Accept));

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyToMerge {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let final_list_request_id = mcp
        .send_devflow_approval_list_request(DevflowApprovalListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            status: None,
        })
        .await?;
    let final_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(final_list_request_id)),
    )
    .await??;
    let DevflowApprovalListResponse { data } = to_response(final_list_response)?;
    assert!(data.iter().any(|item| item.id == approval.id));
    assert!(
        data.iter()
            .any(|item| item.id == approval.id && item.status == DevflowApprovalStatus::Responded)
    );
    Ok(())
}

#[tokio::test]
async fn devflow_pending_approval_recovers_as_cancelled_audit_after_restart() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let responses = vec![create_shell_command_sse_response_with_permissions(
        vec![
            "python3".to_string(),
            "-c".to_string(),
            "print('approval audit')".to_string(),
        ],
        None,
        None,
        Some("require_escalated"),
        Some("Need explicit unsandboxed execution for approval audit"),
        "call1",
    )?];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let update_policy_request_id = mcp
        .send_devflow_approval_policy_update_request(DevflowApprovalPolicyUpdateParams {
            policy: DevflowApprovalPolicy {
                low_risk_approval_policy: AskForApproval::OnRequest,
                medium_risk_approval_policy: AskForApproval::OnFailure,
                high_risk_approval_policy: AskForApproval::OnRequest,
                approvals_reviewer: ApprovalsReviewer::User,
            },
        })
        .await?;
    let update_policy_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(update_policy_request_id)),
    )
    .await??;
    let _: DevflowApprovalPolicyUpdateResponse = to_response(update_policy_response)?;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Approval audit restart".to_string(),
            objective: "Start a task and restart while an approval is pending.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;

    let notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowApproval/requested"),
    )
    .await??;
    let payload: DevflowApprovalRequestedNotification =
        serde_json::from_value(notification.params.expect("approval params"))?;
    assert_eq!(payload.approval.task_id, task.id);
    assert_eq!(payload.approval.run_id, run.id);
    assert_eq!(payload.approval.status, DevflowApprovalStatus::Pending);

    let pending_list_request_id = mcp
        .send_devflow_approval_list_request(DevflowApprovalListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            status: Some(DevflowApprovalStatus::Pending),
        })
        .await?;
    let pending_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(pending_list_request_id)),
    )
    .await??;
    let DevflowApprovalListResponse { data } = to_response(pending_list_response)?;
    assert_eq!(data, vec![payload.approval.clone()]);

    drop(mcp);

    let mut restored_mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, restored_mcp.initialize()).await??;

    let recovered_list_request_id = restored_mcp
        .send_devflow_approval_list_request(DevflowApprovalListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            status: None,
        })
        .await?;
    let recovered_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp
            .read_stream_until_response_message(RequestId::Integer(recovered_list_request_id)),
    )
    .await??;
    let DevflowApprovalListResponse { data } = to_response(recovered_list_response)?;
    assert_eq!(data.len(), 1);
    let recovered = data.first().expect("recovered approval");
    assert_eq!(recovered.id, payload.approval.id);
    assert_eq!(recovered.status, DevflowApprovalStatus::Responded);
    assert_eq!(recovered.decision, Some(DevflowApprovalDecision::Cancel));
    assert!(recovered.responded_at.is_some());
    assert!(
        recovered
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("app-server restarted"))
    );

    let pending_after_restart_request_id = restored_mcp
        .send_devflow_approval_list_request(DevflowApprovalListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            status: Some(DevflowApprovalStatus::Pending),
        })
        .await?;
    let pending_after_restart_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_response_message(RequestId::Integer(
            pending_after_restart_request_id,
        )),
    )
    .await??;
    let DevflowApprovalListResponse { data } = to_response(pending_after_restart_response)?;
    assert_eq!(data, Vec::new());

    let respond_request_id = restored_mcp
        .send_devflow_approval_respond_request(DevflowApprovalRespondParams {
            id: payload.approval.id,
            decision: DevflowApprovalDecision::Accept,
            scope: Some(PermissionGrantScope::Turn),
        })
        .await?;
    let respond_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        restored_mcp.read_stream_until_error_message(RequestId::Integer(respond_request_id)),
    )
    .await??;
    assert_eq!(respond_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        respond_error
            .error
            .message
            .contains("devflow approval is not pending")
    );

    Ok(())
}

#[tokio::test]
async fn devflow_task_create_and_read_roundtrip() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Implement provider diff artifact".to_string(),
            objective: "Make Warp show a stored diff artifact for one coding task.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Medium,
            trigger_source: None,
            dependencies: Some(vec!["task-plan".to_string()]),
            assigned_agent_id: Some("codex-main".to_string()),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(response)?;

    assert_eq!(task.project_id, project_root.path().display().to_string());
    assert_eq!(task.title, "Implement provider diff artifact");
    assert_eq!(task.kind, DevflowTaskKind::Implementation);
    assert_eq!(task.risk_level, DevflowTaskRiskLevel::Medium);
    assert_eq!(task.status, DevflowTaskStatus::Planned);
    assert_eq!(task.dependencies, vec!["task-plan".to_string()]);
    assert_eq!(task.assigned_agent_id.as_deref(), Some("codex-main"));
    assert_eq!(task.run_ids, Vec::<String>::new());
    assert_eq!(task.artifact_ids, Vec::<String>::new());
    assert_eq!(task.worktree_id, None);
    assert_eq!(task.context_pack_id, None);
    assert!(task.created_at > 0);
    assert_eq!(task.created_at, task.updated_at);

    let notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;
    let notification: DevflowTaskStatusChangedNotification =
        serde_json::from_value(notification.params.expect("notification params"))?;
    assert_eq!(notification.task, task);

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: task.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;

    assert_eq!(read_task, task);
    Ok(())
}

#[tokio::test]
async fn devflow_task_start_blocks_medium_risk_without_plan_artifact() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Implement risky provider migration".to_string(),
            objective: "Change provider routing in a way that needs an explicit plan.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Medium,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    assert_eq!(start_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        start_error
            .error
            .message
            .contains("requires a plan artifact"),
        "unexpected start error: {}",
        start_error.error.message
    );

    let blocked_notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;
    let blocked_notification: DevflowTaskStatusChangedNotification =
        serde_json::from_value(blocked_notification.params.expect("task params"))?;
    assert_eq!(blocked_notification.task.id, task.id);
    assert_eq!(blocked_notification.task.status, DevflowTaskStatus::Blocked);
    assert_eq!(blocked_notification.task.run_ids, Vec::<String>::new());
    assert_eq!(blocked_notification.task.context_pack_id, None);

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: task.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;
    assert_eq!(read_task, blocked_notification.task);

    Ok(())
}

#[tokio::test]
async fn devflow_task_plan_list_assign_and_dependency_update_roundtrip() -> Result<()> {
    let codex_home = TempDir::new()?;
    let project_root = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_devflow_task_plan_request(DevflowTaskPlanParams {
            project_root: project_root.path().display().to_string(),
            title: "Ship provider improvements".to_string(),
            objective: "- Add provider form validation\n- Add focused provider regression coverage"
                .to_string(),
            risk_level: DevflowTaskRiskLevel::Medium,
            max_tasks: Some(2),
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let DevflowTaskPlanResponse { data: tasks } = to_response(response)?;

    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0].kind, DevflowTaskKind::Implementation);
    assert_eq!(tasks[1].kind, DevflowTaskKind::Implementation);
    assert_eq!(tasks[2].kind, DevflowTaskKind::Review);
    assert_eq!(tasks[0].artifact_ids.len(), 1);
    assert_eq!(tasks[1].artifact_ids.len(), 1);
    assert_eq!(tasks[2].artifact_ids.len(), 1);
    assert_eq!(
        tasks[2].dependencies,
        vec![tasks[0].id.clone(), tasks[1].id.clone()]
    );
    assert_eq!(tasks[0].assigned_agent_id.as_deref(), Some("codex-worker"));
    assert_eq!(tasks[1].assigned_agent_id.as_deref(), Some("codex-worker"));
    assert_eq!(
        tasks[2].assigned_agent_id.as_deref(),
        Some("codex-reviewer")
    );

    for expected_task in &tasks {
        let notification: JSONRPCNotification = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
        )
        .await??;
        let payload: DevflowTaskStatusChangedNotification =
            serde_json::from_value(notification.params.expect("task params"))?;
        assert_eq!(payload.task.id, expected_task.id);
    }

    let plan_artifact_id = tasks[0]
        .artifact_ids
        .first()
        .cloned()
        .expect("implementation task should include a plan artifact");
    let plan_artifact_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: plan_artifact_id,
        })
        .await?;
    let plan_artifact_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(plan_artifact_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: plan_artifact,
        contents: plan_contents,
    } = to_response(plan_artifact_response)?;
    assert_eq!(plan_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(plan_artifact.task_id, tasks[0].id);
    assert!(plan_artifact.title.contains("Plan for"));
    assert!(plan_contents.contains("## Objective"));
    assert!(plan_contents.contains("## Execution Discipline"));

    let planner_dag_artifact_id = tasks[2]
        .artifact_ids
        .first()
        .cloned()
        .expect("review task should include planner DAG artifact");
    let planner_dag_artifact_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: planner_dag_artifact_id,
        })
        .await?;
    let planner_dag_artifact_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(planner_dag_artifact_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        artifact: planner_dag_artifact,
        contents: planner_dag_contents,
    } = to_response(planner_dag_artifact_response)?;
    assert_eq!(planner_dag_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(planner_dag_artifact.task_id, tasks[2].id);
    assert!(planner_dag_artifact.title.contains("Planner DAG"));
    let planner_dag: serde_json::Value = serde_json::from_str(&planner_dag_contents)?;
    assert_eq!(planner_dag["runner"], "codex-devflow-planner");
    assert_eq!(planner_dag["plannerAgentId"], "codex-main");
    assert_eq!(planner_dag["workerAgentId"], "codex-worker");
    assert_eq!(planner_dag["reviewerAgentId"], "codex-reviewer");
    assert_eq!(planner_dag["integratorAgentId"], "codex-integrator");
    assert_eq!(planner_dag["reviewTaskId"], tasks[2].id);
    assert_eq!(
        planner_dag["implementationTaskIds"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    let list_request_id = mcp
        .send_devflow_task_list_request(DevflowTaskListParams {
            project_id: Some(project_root.path().display().to_string()),
            status: None,
            assigned_agent_id: Some("codex-worker".to_string()),
            cursor: None,
            limit: Some(1),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowTaskListResponse {
        data: page_one,
        next_cursor,
    } = to_response(list_response)?;
    assert_eq!(page_one.len(), 1);
    assert_eq!(next_cursor.as_deref(), Some("1"));

    let page_two_request_id = mcp
        .send_devflow_task_list_request(DevflowTaskListParams {
            project_id: Some(project_root.path().display().to_string()),
            status: None,
            assigned_agent_id: Some("codex-worker".to_string()),
            cursor: next_cursor,
            limit: Some(2),
        })
        .await?;
    let page_two_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(page_two_request_id)),
    )
    .await??;
    let DevflowTaskListResponse {
        data: page_two,
        next_cursor,
    } = to_response(page_two_response)?;
    assert_eq!(page_two.len(), 1);
    assert_eq!(next_cursor, None);

    let assign_request_id = mcp
        .send_devflow_task_assign_request(DevflowTaskAssignParams {
            id: tasks[0].id.clone(),
            assigned_agent_id: Some("codex-reviewer".to_string()),
        })
        .await?;
    let assign_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(assign_request_id)),
    )
    .await??;
    let DevflowTaskAssignResponse {
        task: assigned_task,
    } = to_response(assign_response)?;
    assert_eq!(
        assigned_task.assigned_agent_id.as_deref(),
        Some("codex-reviewer")
    );

    let assign_notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;
    let assign_notification: DevflowTaskStatusChangedNotification =
        serde_json::from_value(assign_notification.params.expect("task params"))?;
    assert_eq!(assign_notification.task, assigned_task);

    let dependencies_update_request_id = mcp
        .send_devflow_task_dependencies_update_request(DevflowTaskDependenciesUpdateParams {
            id: tasks[0].id.clone(),
            dependencies: vec![tasks[1].id.clone()],
        })
        .await?;
    let dependencies_update_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(dependencies_update_request_id)),
    )
    .await??;
    let DevflowTaskDependenciesUpdateResponse {
        task: dependent_task,
    } = to_response(dependencies_update_response)?;
    assert_eq!(dependent_task.dependencies, vec![tasks[1].id.clone()]);

    let dependencies_notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;
    let dependencies_notification: DevflowTaskStatusChangedNotification =
        serde_json::from_value(dependencies_notification.params.expect("task params"))?;
    assert_eq!(dependencies_notification.task, dependent_task);

    let blocked_start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: dependent_task.id.clone(),
        })
        .await?;
    let blocked_start_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(blocked_start_request_id)),
    )
    .await??;
    assert_eq!(blocked_start_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        blocked_start_error
            .error
            .message
            .contains("unresolved dependencies"),
        "unexpected blocked error: {}",
        blocked_start_error.error.message
    );

    let blocked_notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;
    let blocked_notification: DevflowTaskStatusChangedNotification =
        serde_json::from_value(blocked_notification.params.expect("task params"))?;
    assert_eq!(blocked_notification.task.id, dependent_task.id);
    assert_eq!(blocked_notification.task.status, DevflowTaskStatus::Blocked);

    Ok(())
}

#[tokio::test]
async fn devflow_task_dispatch_starts_ready_implementation_tasks_and_reports_integrator_queue()
-> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("README.md"), "dispatch test\n")?;
    run_git(project_root.path(), &["add", "README.md"])?;
    run_git(project_root.path(), &["commit", "-m", "initial"])?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let plan_request_id = mcp
        .send_devflow_task_plan_request(DevflowTaskPlanParams {
            project_root: project_root.path().display().to_string(),
            title: "Dispatch provider graph".to_string(),
            objective: "- Implement provider form\n- Add provider tests".to_string(),
            risk_level: DevflowTaskRiskLevel::Medium,
            max_tasks: Some(2),
        })
        .await?;
    let plan_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(plan_request_id)),
    )
    .await??;
    let DevflowTaskPlanResponse { data: tasks } = to_response(plan_response)?;
    assert_eq!(tasks.len(), 3);

    let create_blocked_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Dependent follow-up implementation".to_string(),
            objective: "Run after the first implementation task finishes.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Medium,
            trigger_source: None,
            dependencies: Some(vec![tasks[0].id.clone()]),
            assigned_agent_id: None,
        })
        .await?;
    let create_blocked_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_blocked_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse {
        task: dependent_task,
    } = to_response(create_blocked_response)?;

    let dispatch_request_id = mcp
        .send_devflow_task_dispatch_request(DevflowTaskDispatchParams {
            project_id: Some(project_root.path().display().to_string()),
            task_ids: None,
            limit: Some(4),
        })
        .await?;
    let dispatch_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(dispatch_request_id)),
    )
    .await??;
    let DevflowTaskDispatchResponse {
        started,
        skipped,
        blocked,
        integrator_artifact,
    } = to_response(dispatch_response)?;

    assert_eq!(started.len(), 2);
    assert!(started.iter().all(|response| {
        response.task.kind == DevflowTaskKind::Implementation
            && response.task.status == DevflowTaskStatus::Running
            && response.run.status == DevflowRunStatus::Running
            && response.task.assigned_agent_id.as_deref() == Some("codex-worker")
    }));
    assert!(blocked.iter().any(|item| {
        item.task_id == dependent_task.id && item.dependencies == vec![tasks[0].id.clone()]
    }));
    assert!(skipped.iter().any(|item| {
        item.task_id == tasks[2].id && item.reason == "waiting for dependency workstreams to merge"
    }));

    let integrator_artifact = integrator_artifact.expect("dispatch report artifact");
    assert_eq!(integrator_artifact.kind, DevflowArtifactKind::Report);
    let artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: integrator_artifact.id,
        })
        .await?;
    let artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse { contents, .. } = to_response(artifact_read_response)?;
    let dispatch_report: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(dispatch_report["runner"], "codex-devflow-integrator");
    assert_eq!(dispatch_report["status"], "blocked");
    assert_eq!(dispatch_report["counts"]["started"], 2);
    assert_eq!(dispatch_report["counts"]["blocked"], 1);
    assert_eq!(dispatch_report["counts"]["skipped"], 1);
    assert_eq!(dispatch_report["integratorQueue"]["counts"]["blocked"], 1);

    for started_response in &started {
        timeout(DEFAULT_TIMEOUT, async {
            loop {
                let notification = mcp
                    .read_stream_until_notification_message("devflowRun/statusChanged")
                    .await?;
                let payload: DevflowRunStatusChangedNotification =
                    serde_json::from_value(notification.params.expect("run status params"))?;
                if payload.run.id == started_response.run.id
                    && payload.run.status == DevflowRunStatus::ReadyToMerge
                {
                    return Ok::<_, anyhow::Error>(());
                }
            }
        })
        .await??;
    }

    let review_dispatch_request_id = mcp
        .send_devflow_task_dispatch_request(DevflowTaskDispatchParams {
            project_id: Some(project_root.path().display().to_string()),
            task_ids: None,
            limit: Some(4),
        })
        .await?;
    let review_dispatch_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(review_dispatch_request_id)),
    )
    .await??;
    let DevflowTaskDispatchResponse {
        started: review_started,
        skipped: review_skipped,
        blocked: review_blocked,
        ..
    } = to_response(review_dispatch_response)?;
    assert_eq!(review_started.len(), 1);
    assert!(review_started.iter().all(|response| {
        response.task.id == tasks[2].id
            && response.task.kind == DevflowTaskKind::Review
            && response.task.assigned_agent_id.as_deref() == Some("codex-reviewer")
    }));
    assert!(review_blocked.iter().any(|item| {
        item.task_id == dependent_task.id
            && item.reason
                == "already blocked; requires dependency resolution, approval, or a conflict-repair action"
    }));
    assert!(review_skipped.iter().any(|item| {
        item.task_id == tasks[0].id && item.status == DevflowTaskStatus::ReadyToMerge
    }));

    let review_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let review_task_request_id = mcp
                .send_devflow_task_read_request(DevflowTaskReadParams {
                    id: tasks[2].id.clone(),
                })
                .await?;
            let review_task_response: JSONRPCResponse = timeout(
                DEFAULT_TIMEOUT,
                mcp.read_stream_until_response_message(RequestId::Integer(review_task_request_id)),
            )
            .await??;
            let DevflowTaskReadResponse { task: review_task } = to_response(review_task_response)?;

            for artifact_id in review_task.artifact_ids.iter().rev() {
                let review_artifact_read_request_id = mcp
                    .send_devflow_artifact_read_request(DevflowArtifactReadParams {
                        id: artifact_id.clone(),
                    })
                    .await?;
                let review_artifact_read_response: JSONRPCResponse = timeout(
                    DEFAULT_TIMEOUT,
                    mcp.read_stream_until_response_message(RequestId::Integer(
                        review_artifact_read_request_id,
                    )),
                )
                .await??;
                let review_artifact_read: DevflowArtifactReadResponse =
                    to_response(review_artifact_read_response)?;
                if review_artifact_read.artifact.kind == DevflowArtifactKind::ReviewReport
                    && review_artifact_read
                        .artifact
                        .summary
                        .starts_with("Review finding state:")
                {
                    return Ok::<_, anyhow::Error>(review_artifact_read);
                }
            }

            sleep(Duration::from_millis(50)).await;
        }
    })
    .await??;
    let DevflowArtifactReadResponse {
        contents: review_contents,
        artifact: review_artifact,
        ..
    } = review_artifact_created;
    assert_eq!(
        review_artifact.summary,
        "Review finding state: status=clear; open=0; resolved=0; waived=0; followUp=0"
    );
    assert!(review_contents.contains("# Review Finding State"));
    assert!(review_contents.contains("\"status\": \"clear\""));

    Ok(())
}

#[tokio::test]
async fn devflow_task_dispatch_auto_merges_clean_ready_worktree() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let responses = vec![
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: ready for integrator merge.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Auto merge provider workstream".to_string(),
            objective: "Update note.txt and let the Integrator merge the clean worktree."
                .to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let dispatch_request_id = mcp
        .send_devflow_task_dispatch_request(DevflowTaskDispatchParams {
            project_id: Some(project_root.path().display().to_string()),
            task_ids: None,
            limit: Some(1),
        })
        .await?;
    let dispatch_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(dispatch_request_id)),
    )
    .await??;
    let DevflowTaskDispatchResponse {
        started,
        skipped,
        blocked,
        integrator_artifact,
    } = to_response(dispatch_response)?;

    assert_eq!(started.len(), 1);
    assert!(skipped.is_empty());
    assert!(blocked.is_empty());
    let started_run = started[0].run.clone();

    let dispatch_artifact = integrator_artifact.expect("dispatch report artifact");
    let dispatch_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: dispatch_artifact.id,
        })
        .await?;
    let dispatch_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            dispatch_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse {
        contents: dispatch_report,
        ..
    } = to_response(dispatch_artifact_read_response)?;
    let dispatch_report: serde_json::Value = serde_json::from_str(&dispatch_report)?;
    assert_eq!(
        dispatch_report["policy"]["autoMergeRule"].as_str(),
        Some(
            "Runs started by dispatch auto-merge their managed worktree after the implementation task is ready_to_merge with diff, required quality-gate artifacts, and review artifacts all present."
        )
    );

    let merge_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.run_id == started_run.id
                && payload
                    .artifact
                    .title
                    .starts_with("Integrator merge report")
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        std::fs::read_to_string(project_root.path().join("note.txt"))?,
        "after\n"
    );

    let merge_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: merge_artifact_created.artifact.id.clone(),
        })
        .await?;
    let merge_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(merge_artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        contents: merge_report,
        ..
    } = to_response(merge_artifact_read_response)?;
    let merge_report: serde_json::Value = serde_json::from_str(&merge_report)?;
    assert_eq!(merge_report["runner"], "codex-devflow-integrator");
    assert_eq!(merge_report["merged"].as_bool(), Some(true));
    assert_eq!(
        merge_report["nextAction"].as_str(),
        Some("ready_for_release_prep")
    );
    assert!(
        merge_report["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("-before") && diff.contains("+after"))
    );

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams { id: task.id })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;
    assert_eq!(read_task.status, DevflowTaskStatus::ReadyToMerge);
    assert!(
        read_task
            .artifact_ids
            .contains(&merge_artifact_created.artifact.id)
    );

    let prep_request_id = mcp
        .send_devflow_release_prep_create_request(DevflowReleasePrepCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(read_task.id.clone()),
        })
        .await?;
    let prep_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(prep_request_id)),
    )
    .await??;
    let DevflowReleasePrepCreateResponse {
        status,
        blockers,
        pr_body_artifact,
        ..
    } = to_response(prep_response)?;
    assert_eq!(status, DevflowReleasePrepStatus::Ready);
    assert!(blockers.is_empty());
    let pr_body = std::fs::read_to_string(&pr_body_artifact.path)?;
    assert!(pr_body.contains("## Integrator"));
    assert!(pr_body.contains("Auto merge provider workstream: merged via"));

    let submit_request_id = mcp
        .send_devflow_release_prep_submit_request(DevflowReleaseSubmitParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(read_task.id.clone()),
            mode: DevflowReleaseSubmitMode::CommitOnly,
        })
        .await?;
    let submit_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(submit_request_id)),
    )
    .await??;
    let DevflowReleaseSubmitResponse {
        status,
        blockers,
        approval,
        commit_message_artifact,
        pr_body_artifact: submit_pr_body_artifact,
        release_note_artifact,
        command,
        remote,
        branch,
        ..
    } = to_response(submit_response)?;
    assert_eq!(status, DevflowReleaseSubmitStatus::PendingApproval);
    assert!(blockers.is_empty());
    assert!(command.contains("git commit -F"));
    assert!(!command.contains("git push"));
    assert!(remote.is_none());
    assert_eq!(branch.as_deref(), Some("main"));
    assert!(
        commit_message_artifact
            .summary
            .contains("Release prep status: ready")
    );
    assert_eq!(submit_pr_body_artifact.kind, DevflowArtifactKind::Report);
    assert_eq!(release_note_artifact.kind, DevflowArtifactKind::Report);

    let approval = approval.expect("release publish approval");
    assert_eq!(
        approval.kind,
        codex_app_server_protocol::DevflowApprovalKind::ReleasePublish
    );
    let approval_requested = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowApproval/requested")
                .await?;
            let payload: DevflowApprovalRequestedNotification =
                serde_json::from_value(notification.params.expect("approval params"))?;
            if payload.approval.id == approval.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        approval_requested.approval.command.as_deref(),
        Some(command.as_str())
    );

    let approval_request_id = mcp
        .send_devflow_approval_respond_request(DevflowApprovalRespondParams {
            id: approval.id.clone(),
            decision: DevflowApprovalDecision::Accept,
            scope: None,
        })
        .await?;
    let approval_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(approval_request_id)),
    )
    .await??;
    let DevflowApprovalRespondResponse { approval } = to_response(approval_response)?;
    assert_eq!(approval.status, DevflowApprovalStatus::Responded);

    let publish_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload
                .artifact
                .title
                .starts_with("Release publish report for ")
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    let publish_report = std::fs::read_to_string(&publish_artifact_created.artifact.path)?;
    assert!(publish_report.contains("Release publish report"));
    assert!(publish_report.contains("Mode: commit_only"));
    assert!(publish_report.contains("Command template:"));
    assert!(publish_report.contains("Status: submitted"));

    let git_commit_message = std::process::Command::new("git")
        .args(["log", "-1", "--pretty=%B"])
        .current_dir(project_root.path())
        .output()?;
    assert!(git_commit_message.status.success());
    let git_commit_message = String::from_utf8(git_commit_message.stdout)?;
    assert!(git_commit_message.contains("devflow: Auto merge provider workstream"));

    Ok(())
}

#[tokio::test]
async fn devflow_task_dispatch_blocks_release_prep_when_review_findings_open() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let responses = vec![
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response(
            "- [P1] Resolve the review finding before merge.",
        )?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Review finding workstream".to_string(),
            objective: "Update note.txt but leave a review finding open.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let dispatch_request_id = mcp
        .send_devflow_task_dispatch_request(DevflowTaskDispatchParams {
            project_id: Some(project_root.path().display().to_string()),
            task_ids: None,
            limit: Some(1),
        })
        .await?;
    let dispatch_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(dispatch_request_id)),
    )
    .await??;
    let DevflowTaskDispatchResponse { started, .. } = to_response(dispatch_response)?;
    assert_eq!(started.len(), 1);
    let started_run = started[0].run.clone();

    let review_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::ReviewReport
                && payload.artifact.run_id == started_run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;
    assert!(
        review_artifact_created
            .summary
            .contains("Review finding state: status=open")
    );
    let review_body = std::fs::read_to_string(&review_artifact_created.path)?;
    assert!(review_body.contains("# Review Finding State"));
    assert!(review_body.contains("## Structured Findings"));
    assert!(review_body.contains("\"status\": \"open\""));
    assert!(review_body.contains("\"severity\": \"p1\""));

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == started_run.id
                && payload.run.status == DevflowRunStatus::ReadyForReview
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    assert_eq!(
        std::fs::read_to_string(project_root.path().join("note.txt"))?,
        "before\n"
    );

    let prep_request_id = mcp
        .send_devflow_release_prep_create_request(DevflowReleasePrepCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(task.id),
        })
        .await?;
    let prep_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(prep_request_id)),
    )
    .await??;
    let DevflowReleasePrepCreateResponse {
        status,
        blockers,
        pr_body_artifact,
        ..
    } = to_response(prep_response)?;
    assert_eq!(status, DevflowReleasePrepStatus::Blocked);
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker.contains("unresolved review findings"))
    );
    let pr_body = std::fs::read_to_string(&pr_body_artifact.path)?;
    assert!(pr_body.contains("unresolved review findings"));

    Ok(())
}

#[tokio::test]
async fn devflow_task_read_rejects_unknown_task_id() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: "missing-task".to_string(),
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.id, RequestId::Integer(request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(error.error.message, "unknown devflow task id: missing-task");
    Ok(())
}

#[tokio::test]
async fn devflow_worktree_create_read_and_cleanup_roundtrip() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Isolated change".to_string(),
            objective: "Prepare an isolated worktree for implementation.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let create_worktree_request_id = mcp
        .send_devflow_worktree_create_request(DevflowWorktreeCreateParams {
            task_id: task.id.clone(),
        })
        .await?;
    let create_worktree_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_worktree_request_id)),
    )
    .await??;
    let DevflowWorktreeCreateResponse { worktree } = to_response(create_worktree_response)?;
    assert_eq!(worktree.task_id, task.id);
    assert_eq!(worktree.status, DevflowWorktreeStatus::Active);
    assert!(std::path::Path::new(&worktree.root_path).exists());

    let worktree_status_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowWorktree/statusChanged")
                .await?;
            let payload: DevflowWorktreeStatusChangedNotification =
                serde_json::from_value(notification.params.expect("worktree params"))?;
            if payload.worktree.id == worktree.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(worktree_status_notification.worktree.id, worktree.id);

    let read_request_id = mcp
        .send_devflow_worktree_read_request(DevflowWorktreeReadParams {
            id: worktree.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowWorktreeReadResponse {
        worktree: read_worktree,
    } = to_response(read_response)?;
    assert_eq!(read_worktree.id, worktree.id);
    assert_eq!(read_worktree.status, DevflowWorktreeStatus::Active);

    let cleanup_request_id = mcp
        .send_devflow_worktree_cleanup_request(DevflowWorktreeCleanupParams {
            id: worktree.id.clone(),
        })
        .await?;
    let cleanup_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(cleanup_request_id)),
    )
    .await??;
    let DevflowWorktreeCleanupResponse {
        cleaned,
        worktree: cleaned_worktree,
    } = to_response(cleanup_response)?;
    assert!(cleaned);
    assert_eq!(cleaned_worktree.status, DevflowWorktreeStatus::Cleaned);
    assert!(!std::path::Path::new(&cleaned_worktree.root_path).exists());

    let read_after_cleanup_request_id = mcp
        .send_devflow_worktree_read_request(DevflowWorktreeReadParams { id: worktree.id })
        .await?;
    let read_after_cleanup_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_after_cleanup_request_id)),
    )
    .await??;
    let DevflowWorktreeReadResponse {
        worktree: read_after_cleanup,
    } = to_response(read_after_cleanup_response)?;
    assert_eq!(read_after_cleanup.status, DevflowWorktreeStatus::Cleaned);
    Ok(())
}

#[tokio::test]
async fn devflow_worktree_list_paginates_created_worktrees() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let codex_home = TempDir::new()?;
    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let mut task_ids = Vec::new();
    for index in 0..2 {
        let create_task_request_id = mcp
            .send_devflow_task_create_request(DevflowTaskCreateParams {
                project_root: project_root.path().display().to_string(),
                title: format!("Task {}", index + 1),
                objective: "Prepare an isolated worktree.".to_string(),
                kind: DevflowTaskKind::Implementation,
                risk_level: DevflowTaskRiskLevel::Low,
                trigger_source: None,
                dependencies: None,
                assigned_agent_id: None,
            })
            .await?;
        let create_task_response: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
        )
        .await??;
        let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;
        task_ids.push(task.id.clone());
        let _: JSONRPCNotification = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
        )
        .await??;

        let create_worktree_request_id = mcp
            .send_devflow_worktree_create_request(DevflowWorktreeCreateParams { task_id: task.id })
            .await?;
        let _: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_response_message(RequestId::Integer(create_worktree_request_id)),
        )
        .await??;
        let _: JSONRPCNotification = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_notification_message("devflowWorktree/statusChanged"),
        )
        .await??;
    }

    let list_request_id = mcp
        .send_devflow_worktree_list_request(DevflowWorktreeListParams {
            project_id: Some(project_root.path().display().to_string()),
            task_id: None,
            status: None,
            cursor: None,
            limit: Some(1),
        })
        .await?;
    let list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_request_id)),
    )
    .await??;
    let DevflowWorktreeListResponse {
        data: page_one,
        next_cursor,
    } = to_response(list_response)?;
    assert_eq!(page_one.len(), 1);
    assert_eq!(next_cursor.as_deref(), Some("1"));
    assert!(task_ids.contains(&page_one[0].task_id));

    let page_two_request_id = mcp
        .send_devflow_worktree_list_request(DevflowWorktreeListParams {
            project_id: Some(project_root.path().display().to_string()),
            task_id: None,
            status: None,
            cursor: next_cursor,
            limit: Some(5),
        })
        .await?;
    let page_two_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(page_two_request_id)),
    )
    .await??;
    let DevflowWorktreeListResponse {
        data: page_two,
        next_cursor,
    } = to_response(page_two_response)?;
    assert_eq!(page_two.len(), 1);
    assert_eq!(next_cursor, None);
    assert!(task_ids.contains(&page_two[0].task_id));
    assert_ne!(page_one[0].id, page_two[0].id);

    Ok(())
}

#[tokio::test]
async fn devflow_worktree_cleanup_rejects_primary_worktree() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let codex_home = TempDir::new()?;
    let worktree_id = "primary-worktree-test";
    let metadata_root = codex_home.path().join("devflow").join("worktree-metadata");
    std::fs::create_dir_all(&metadata_root)?;
    let fake_worktree = serde_json::json!({
        "worktree": {
            "id": worktree_id,
            "taskId": "task-1",
            "projectId": project_root.path().display().to_string(),
            "repoRoot": project_root.path().display().to_string(),
            "rootPath": project_root.path().display().to_string(),
            "cwdPath": project_root.path().display().to_string(),
            "branch": "main",
            "baseBranch": "main",
            "baseCommit": "deadbeef",
            "headCommit": "deadbeef",
            "managed": true,
            "status": "active",
            "createdAt": 1,
            "updatedAt": 1
        }
    });
    std::fs::write(
        metadata_root.join(format!("{worktree_id}.json")),
        serde_json::to_string_pretty(&fake_worktree)?,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let cleanup_request_id = mcp
        .send_devflow_worktree_cleanup_request(DevflowWorktreeCleanupParams {
            id: worktree_id.to_string(),
        })
        .await?;
    let cleanup_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(cleanup_request_id)),
    )
    .await??;

    assert_eq!(cleanup_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        cleanup_error.error.message.contains("primary worktree"),
        "unexpected cleanup error: {}",
        cleanup_error.error.message
    );
    Ok(())
}

#[tokio::test]
async fn devflow_automatic_repair_retries_failed_quality_gate_once() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let bad_patch = concat!(
        "*** Begin Patch\n",
        "*** Update File: note.txt\n",
        "@@\n",
        "-before\n",
        "+after",
        "   \n",
        "*** End Patch\n",
    );
    let repair_patch = concat!(
        "*** Begin Patch\n",
        "*** Update File: note.txt\n",
        "@@\n",
        "-after",
        "   \n",
        "+after\n",
        "*** End Patch\n",
    );
    let responses = vec![
        create_apply_patch_sse_response(bad_patch, "patch-1")?,
        create_final_assistant_message_sse_response("first pass finished")?,
        create_apply_patch_sse_response(repair_patch, "patch-2")?,
        create_final_assistant_message_sse_response("automatic repair finished")?,
        create_final_assistant_message_sse_response("Review: automatic repair succeeded.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Repair note formatting".to_string(),
            objective: "Update note.txt and leave the diff clean.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse {
        task: _task,
        run: first_run,
    } = to_response(start_response)?;

    let failed_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.run_id == first_run.id
                && payload.gate.status == DevflowQualityGateStatus::Failed
            {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(failed_gate.command, "git diff --check");

    let failed_run_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == first_run.id && payload.run.status == DevflowRunStatus::Failed {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert!(
        failed_run_notification
            .run
            .exit_reason
            .as_deref()
            .unwrap_or_default()
            .contains("auto repair queued")
    );

    let repair_run_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.task_id == task.id
                && payload.run.id != first_run.id
                && payload.run.status == DevflowRunStatus::Running
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    let repair_run_id = repair_run_notification.run.id.clone();

    let passed_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.run_id == repair_run_id
                && payload.gate.status == DevflowQualityGateStatus::Passed
            {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(passed_gate.command, "git diff --check");

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == repair_run_id
                && payload.run.status == DevflowRunStatus::ReadyToMerge
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(task.id.clone()),
            run_id: None,
        })
        .await?;
    let gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(gate_list_request_id)),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } = to_response(gate_list_response)?;
    assert_eq!(gates.len(), 2);
    assert!(
        gates
            .iter()
            .any(|gate| gate.status == DevflowQualityGateStatus::Failed)
    );
    assert!(
        gates
            .iter()
            .any(|gate| gate.status == DevflowQualityGateStatus::Passed)
    );

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: task.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;
    assert_eq!(read_task.status, DevflowTaskStatus::ReadyToMerge);
    assert_eq!(read_task.run_ids.len(), 2);

    let worktree_id = read_task
        .worktree_id
        .clone()
        .expect("repair task should keep managed worktree");
    let worktree_read_request_id = mcp
        .send_devflow_worktree_read_request(DevflowWorktreeReadParams { id: worktree_id })
        .await?;
    let worktree_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(worktree_read_request_id)),
    )
    .await??;
    let DevflowWorktreeReadResponse { worktree } = to_response(worktree_read_response)?;
    assert_eq!(
        std::fs::read_to_string(std::path::Path::new(&worktree.cwd_path).join("note.txt"))?,
        "after\n"
    );
    Ok(())
}

#[tokio::test]
async fn devflow_worktree_merge_applies_clean_diff_and_blocks_conflicts() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let mut responses = vec![
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: ready to merge.")?,
        create_final_assistant_message_sse_response("Conflict repair ready.")?,
        create_final_assistant_message_sse_response("Review: repair ready to merge.")?,
    ];
    responses.extend(
        (0..10)
            .map(|_| create_final_assistant_message_sse_response("Done"))
            .collect::<Result<Vec<_>>>()?,
    );
    let server = create_mock_responses_server_sequence_unchecked(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let merge_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Mergeable change".to_string(),
            objective: "Change note.txt from before to after.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let merge_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(merge_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task: merge_task } = to_response(merge_task_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let merge_start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: merge_task.id.clone(),
        })
        .await?;
    let merge_start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(merge_start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse {
        task: merge_started_task,
        run: merge_run,
    } = to_response(merge_start_response)?;
    let merge_worktree_id = merge_started_task
        .worktree_id
        .clone()
        .expect("implementation task should have managed worktree");

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == merge_run.id
                && payload.run.status == DevflowRunStatus::ReadyToMerge
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let merge_request_id = mcp
        .send_devflow_worktree_merge_request(DevflowWorktreeMergeParams {
            id: merge_worktree_id.clone(),
        })
        .await?;
    let merge_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(merge_request_id)),
    )
    .await??;
    let DevflowWorktreeMergeResponse {
        merged,
        worktree: merged_worktree,
        task: merged_task,
        conflicts,
    } = to_response(merge_response)?;
    assert!(merged);
    assert!(conflicts.is_empty());
    assert_eq!(merged_worktree.id, merge_worktree_id);
    assert_eq!(merged_task.status, DevflowTaskStatus::ReadyToMerge);
    assert_eq!(
        std::fs::read_to_string(project_root.path().join("note.txt"))?,
        "after\n"
    );

    let merge_status_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowTask/statusChanged")
                .await?;
            let payload: DevflowTaskStatusChangedNotification =
                serde_json::from_value(notification.params.expect("task params"))?;
            if payload.task.id == merged_task.id
                && payload.task.status == DevflowTaskStatus::ReadyToMerge
                && payload.task.artifact_ids == merged_task.artifact_ids
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(merge_status_notification.task, merged_task);

    let merge_artifact_list_request_id = mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(merged_task.id.clone()),
            run_id: Some(merge_run.id.clone()),
            kind: Some(DevflowArtifactKind::Report),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let merge_artifact_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(merge_artifact_list_request_id)),
    )
    .await??;
    let DevflowArtifactListResponse {
        data: merge_artifacts,
        ..
    } = to_response(merge_artifact_list_response)?;
    let merge_report_artifact = merge_artifacts
        .iter()
        .find(|artifact| artifact.title.starts_with("Integrator merge report"))
        .cloned()
        .expect("merge should produce an integrator report artifact");
    assert!(merged_task.artifact_ids.contains(&merge_report_artifact.id));
    let merge_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: merge_report_artifact.id,
        })
        .await?;
    let merge_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(merge_artifact_read_request_id)),
    )
    .await??;
    let DevflowArtifactReadResponse {
        contents: merge_report,
        ..
    } = to_response(merge_artifact_read_response)?;
    let merge_report: serde_json::Value = serde_json::from_str(&merge_report)?;
    assert_eq!(merge_report["runner"], "codex-devflow-integrator");
    assert_eq!(merge_report["merged"].as_bool(), Some(true));
    assert_eq!(
        merge_report["nextAction"].as_str(),
        Some("ready_for_release_prep")
    );
    assert!(
        merge_report["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("-before") && diff.contains("+after"))
    );

    let conflict_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Conflicting change".to_string(),
            objective: "Prepare another change for note.txt.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let conflict_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(conflict_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse {
        task: conflict_task,
    } = to_response(conflict_task_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let conflict_worktree_request_id = mcp
        .send_devflow_worktree_create_request(DevflowWorktreeCreateParams {
            task_id: conflict_task.id.clone(),
        })
        .await?;
    let conflict_worktree_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(conflict_worktree_request_id)),
    )
    .await??;
    let DevflowWorktreeCreateResponse {
        worktree: conflict_worktree,
    } = to_response(conflict_worktree_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowWorktree/statusChanged"),
    )
    .await??;

    std::fs::write(
        std::path::Path::new(&conflict_worktree.cwd_path).join("note.txt"),
        "conflict-worktree\n",
    )?;
    std::fs::write(project_root.path().join("note.txt"), "mainline\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "mainline change"])?;

    let conflict_merge_request_id = mcp
        .send_devflow_worktree_merge_request(DevflowWorktreeMergeParams {
            id: conflict_worktree.id.clone(),
        })
        .await?;
    let conflict_merge_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(conflict_merge_request_id)),
    )
    .await??;
    let DevflowWorktreeMergeResponse {
        merged,
        worktree: merged_conflict_worktree,
        task: blocked_task,
        conflicts,
    } = to_response(conflict_merge_response)?;
    assert!(!merged);
    assert_eq!(merged_conflict_worktree.id, conflict_worktree.id);
    assert_eq!(blocked_task.status, DevflowTaskStatus::Blocked);
    assert!(!conflicts.is_empty());
    assert_eq!(
        std::fs::read_to_string(project_root.path().join("note.txt"))?,
        "mainline\n"
    );

    let blocked_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowTask/statusChanged")
                .await?;
            let payload: DevflowTaskStatusChangedNotification =
                serde_json::from_value(notification.params.expect("task params"))?;
            if payload.task.id == blocked_task.id
                && payload.task.status == DevflowTaskStatus::Blocked
                && payload.task.artifact_ids == blocked_task.artifact_ids
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(blocked_notification.task, blocked_task);

    let conflict_artifact_list_request_id = mcp
        .send_devflow_artifact_list_request(DevflowArtifactListParams {
            task_id: Some(blocked_task.id.clone()),
            run_id: None,
            kind: Some(DevflowArtifactKind::Report),
            cursor: None,
            limit: Some(10),
        })
        .await?;
    let conflict_artifact_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            conflict_artifact_list_request_id,
        )),
    )
    .await??;
    let DevflowArtifactListResponse {
        data: conflict_artifacts,
        ..
    } = to_response(conflict_artifact_list_response)?;
    let conflict_report_artifact = conflict_artifacts
        .iter()
        .find(|artifact| artifact.title.starts_with("Integrator merge report"))
        .cloned()
        .expect("conflict should produce an integrator report artifact");
    assert!(
        blocked_task
            .artifact_ids
            .contains(&conflict_report_artifact.id)
    );
    let conflict_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: conflict_report_artifact.id,
        })
        .await?;
    let conflict_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            conflict_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse {
        contents: conflict_report,
        ..
    } = to_response(conflict_artifact_read_response)?;
    let conflict_report: serde_json::Value = serde_json::from_str(&conflict_report)?;
    assert_eq!(conflict_report["runner"], "codex-devflow-integrator");
    assert_eq!(conflict_report["merged"].as_bool(), Some(false));
    assert_eq!(
        conflict_report["nextAction"].as_str(),
        Some("resolve_conflicts_before_retrying_integrator_merge")
    );
    assert!(
        conflict_report["conflicts"]
            .as_array()
            .is_some_and(|conflicts| !conflicts.is_empty())
    );
    assert!(
        conflict_report["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("conflict-worktree"))
    );

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.task_id == blocked_task.id
                && payload
                    .artifact
                    .title
                    .starts_with("Integrator merge report")
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    std::fs::write(project_root.path().join("note.txt"), "after\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(
        project_root.path(),
        &["commit", "-m", "resolve mainline conflict"],
    )?;

    let repair_reconcile_request_id = mcp
        .send_devflow_watchdog_reconcile_request(DevflowWatchdogReconcileParams {
            project_id: project_root.path().display().to_string(),
            limit: Some(1),
        })
        .await?;
    let repair_reconcile_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(repair_reconcile_request_id)),
    )
    .await??;
    let DevflowWatchdogReconcileResponse {
        project_id,
        summary,
        started,
        skipped,
        blocked,
        integrator_artifact,
        ..
    } = to_response(repair_reconcile_response)?;
    assert_eq!(project_id, project_root.path().display().to_string());
    assert!(summary.contains("watchdog reconcile selected 1 repairable blocked conflict task"));
    assert!(integrator_artifact.is_some());
    assert_eq!(started.len(), 1);
    assert!(skipped.is_empty());
    assert!(blocked.is_empty());
    let repair_run = started[0].run.clone();
    assert_eq!(repair_run.task_id, blocked_task.id);
    assert!(repair_run.input.contains("Devflow conflict repair task"));
    assert!(
        repair_run
            .input
            .contains("resolve_conflicts_before_retrying_integrator_merge")
    );

    let repair_merge_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.run_id == repair_run.id
                && payload
                    .artifact
                    .title
                    .starts_with("Integrator merge report")
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;
    assert_eq!(
        std::fs::read_to_string(project_root.path().join("note.txt"))?,
        "conflict-worktree\n"
    );

    let repair_merge_artifact_read_request_id = mcp
        .send_devflow_artifact_read_request(DevflowArtifactReadParams {
            id: repair_merge_artifact_created.id,
        })
        .await?;
    let repair_merge_artifact_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            repair_merge_artifact_read_request_id,
        )),
    )
    .await??;
    let DevflowArtifactReadResponse {
        contents: repair_merge_report,
        ..
    } = to_response(repair_merge_artifact_read_response)?;
    let repair_merge_report: serde_json::Value = serde_json::from_str(&repair_merge_report)?;
    assert_eq!(repair_merge_report["merged"].as_bool(), Some(true));
    assert_eq!(
        repair_merge_report["nextAction"].as_str(),
        Some("ready_for_release_prep")
    );

    Ok(())
}

#[tokio::test]
async fn devflow_task_start_runs_required_snapshot_gate_before_review() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let responses = vec![
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: snapshot-sensitive change is safe.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Update provider settings UI".to_string(),
            objective: "Adjust the settings dialog copy and verify the snapshot path.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;

    let completed_gates = timeout(DEFAULT_TIMEOUT, async {
        let mut gates = Vec::new();
        while gates.len() < 2 {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.run_id == run.id {
                gates.push(payload.gate);
            }
        }
        Ok::<_, anyhow::Error>(gates)
    })
    .await??;
    assert_eq!(
        completed_gates
            .iter()
            .map(|gate| gate.kind)
            .collect::<Vec<_>>(),
        vec![
            DevflowQualityGateKind::TargetedTest,
            DevflowQualityGateKind::Snapshot
        ]
    );
    assert!(
        completed_gates
            .iter()
            .all(|gate| gate.status == DevflowQualityGateStatus::Passed)
    );
    assert!(
        completed_gates
            .iter()
            .all(|gate| gate.artifact_id.is_some())
    );

    let review_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::ReviewReport
                && payload.artifact.run_id == run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;
    assert!(
        review_artifact_created
            .summary
            .contains("Review finding state: status=clear")
    );

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyToMerge {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    Ok(())
}

#[tokio::test]
async fn devflow_quality_gate_run_rerun_and_waive_roundtrip() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let responses = vec![
        create_exec_command_sse_response("exec-1")?,
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: looks fine.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_task_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Implementation task".to_string(),
            objective: "Prepare a finished implementation run for manual quality gates."
                .to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_task_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_task_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_task_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { task, run } = to_response(start_response)?;

    let ready_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(ready_gate.status, DevflowQualityGateStatus::Passed);

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyToMerge {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let gate_run_request_id = mcp
        .send_devflow_quality_gate_run_request(DevflowQualityGateRunParams {
            task_id: task.id.clone(),
            kind: Some(DevflowQualityGateKind::Format),
            command_override: Some("git diff --check".to_string()),
        })
        .await?;
    let gate_run_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(gate_run_request_id)),
    )
    .await??;
    let DevflowQualityGateRunResponse { gate } = to_response(gate_run_response)?;
    assert_eq!(gate.kind, DevflowQualityGateKind::Format);
    assert_eq!(gate.status, DevflowQualityGateStatus::Running);

    let passed_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.id == gate.id {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(passed_gate.kind, DevflowQualityGateKind::Format);
    assert_eq!(passed_gate.status, DevflowQualityGateStatus::Passed);

    let rerun_request_id = mcp
        .send_devflow_quality_gate_rerun_request(DevflowQualityGateRerunParams {
            id: gate.id.clone(),
        })
        .await?;
    let rerun_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(rerun_request_id)),
    )
    .await??;
    let DevflowQualityGateRerunResponse { gate: rerun_gate } = to_response(rerun_response)?;
    assert_eq!(rerun_gate.kind, DevflowQualityGateKind::Format);
    let rerun_completed = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.id == rerun_gate.id {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(rerun_completed.kind, DevflowQualityGateKind::Format);
    assert_eq!(rerun_completed.status, DevflowQualityGateStatus::Passed);

    let failing_gate_request_id = mcp
        .send_devflow_quality_gate_run_request(DevflowQualityGateRunParams {
            task_id: task.id.clone(),
            kind: Some(DevflowQualityGateKind::Lint),
            command_override: Some("git status --definitely-invalid-arg".to_string()),
        })
        .await?;
    let failing_gate_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(failing_gate_request_id)),
    )
    .await??;
    let DevflowQualityGateRunResponse { gate: failing_gate } = to_response(failing_gate_response)?;
    assert_eq!(failing_gate.kind, DevflowQualityGateKind::Lint);
    let failed_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.id == failing_gate.id {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(failed_gate.kind, DevflowQualityGateKind::Lint);
    assert_eq!(failed_gate.status, DevflowQualityGateStatus::Failed);

    let integration_gate_request_id = mcp
        .send_devflow_quality_gate_run_request(DevflowQualityGateRunParams {
            task_id: task.id.clone(),
            kind: Some(DevflowQualityGateKind::IntegrationTest),
            command_override: Some("git status --definitely-invalid-arg".to_string()),
        })
        .await?;
    let integration_gate_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(integration_gate_request_id)),
    )
    .await??;
    let DevflowQualityGateRunResponse {
        gate: integration_gate,
    } = to_response(integration_gate_response)?;
    assert_eq!(
        integration_gate.kind,
        DevflowQualityGateKind::IntegrationTest
    );
    let integration_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.id == integration_gate.id {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(
        integration_gate.kind,
        DevflowQualityGateKind::IntegrationTest
    );
    assert_eq!(integration_gate.status, DevflowQualityGateStatus::Failed);

    let snapshot_gate_request_id = mcp
        .send_devflow_quality_gate_run_request(DevflowQualityGateRunParams {
            task_id: task.id.clone(),
            kind: Some(DevflowQualityGateKind::Snapshot),
            command_override: Some("git status --definitely-invalid-arg".to_string()),
        })
        .await?;
    let snapshot_gate_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(snapshot_gate_request_id)),
    )
    .await??;
    let DevflowQualityGateRunResponse {
        gate: snapshot_gate,
    } = to_response(snapshot_gate_response)?;
    assert_eq!(snapshot_gate.kind, DevflowQualityGateKind::Snapshot);
    let snapshot_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.id == snapshot_gate.id {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(snapshot_gate.kind, DevflowQualityGateKind::Snapshot);
    assert_eq!(snapshot_gate.status, DevflowQualityGateStatus::Failed);

    let waive_request_id = mcp
        .send_devflow_quality_gate_waive_request(DevflowQualityGateWaiveParams {
            id: failing_gate.id.clone(),
            reason: "accepted for diagnostics".to_string(),
        })
        .await?;
    let waive_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(waive_request_id)),
    )
    .await??;
    let DevflowQualityGateWaiveResponse {
        gate: waiver_requested_gate,
        approval,
    } = to_response(waive_response)?;
    assert_eq!(
        waiver_requested_gate.status,
        DevflowQualityGateStatus::Failed
    );
    assert_eq!(waiver_requested_gate.waived_reason, None);
    let approval = approval.expect("waive should queue an approval");
    assert_eq!(
        approval.kind,
        codex_app_server_protocol::DevflowApprovalKind::QualityGateWaive
    );
    assert_eq!(
        approval.quality_gate_id.as_deref(),
        Some(failing_gate.id.as_str())
    );
    assert_eq!(approval.reason.as_deref(), Some("accepted for diagnostics"));
    assert_eq!(approval.status, DevflowApprovalStatus::Pending);

    let approval_notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowApproval/requested"),
    )
    .await??;
    let approval_payload: DevflowApprovalRequestedNotification =
        serde_json::from_value(approval_notification.params.expect("approval params"))?;
    assert_eq!(approval_payload.approval.id, approval.id);

    let respond_request_id = mcp
        .send_devflow_approval_respond_request(DevflowApprovalRespondParams {
            id: approval.id.clone(),
            decision: DevflowApprovalDecision::Accept,
            scope: None,
        })
        .await?;
    let respond_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(respond_request_id)),
    )
    .await??;
    let DevflowApprovalRespondResponse {
        approval: responded_approval,
    } = to_response(respond_response)?;
    assert_eq!(responded_approval.status, DevflowApprovalStatus::Responded);
    assert_eq!(
        responded_approval.decision,
        Some(DevflowApprovalDecision::Accept)
    );

    let waived_gate = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.id == failing_gate.id
                && payload.gate.status == DevflowQualityGateStatus::Waived
            {
                return Ok::<_, anyhow::Error>(payload.gate);
            }
        }
    })
    .await??;
    assert_eq!(waived_gate.status, DevflowQualityGateStatus::Waived);
    assert_eq!(
        waived_gate.waived_reason.as_deref(),
        Some("accepted for diagnostics")
    );

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn devflow_legacy_claude_review_task_uses_dependency_diff_context() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let responses = vec![
        create_exec_command_sse_response("exec-1")?,
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: no blocking issues.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let cli_root = TempDir::new()?;
    let (script_path, args_path, _) = write_fake_claude_cli(
        cli_root.path(),
        "# Claude Review\n\n- The diff is small and easy to audit.\n- No blocking issues found.",
    )?;
    let script_path = script_path.to_string_lossy().into_owned();

    let mut mcp = McpProcess::new_with_env(
        codex_home.path(),
        &[("CODEX_DEVFLOW_CLAUDE_CLI", Some(script_path.as_str()))],
    )
    .await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let implementation_create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Implementation task".to_string(),
            objective: "Change note.txt from before to after and verify the repo state."
                .to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let implementation_create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(
            implementation_create_request_id,
        )),
    )
    .await??;
    let DevflowTaskCreateResponse {
        task: implementation_task,
    } = to_response(implementation_create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let implementation_start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: implementation_task.id.clone(),
        })
        .await?;
    let implementation_start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(implementation_start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse {
        task: _implementation_task,
        run: implementation_run,
    } = to_response(implementation_start_response)?;

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == implementation_run.id
                && payload.run.status == DevflowRunStatus::ReadyToMerge
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let review_create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Review the implementation diff".to_string(),
            objective: "Write a text review for the implementation task.".to_string(),
            kind: DevflowTaskKind::Review,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: Some("legacy:manual".to_string()),
            dependencies: Some(vec![implementation_task.id.clone()]),
            assigned_agent_id: Some("claude-reviewer".to_string()),
        })
        .await?;
    let review_create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(review_create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task: review_task } = to_response(review_create_response)?;
    assert_eq!(review_task.trigger_source.as_deref(), Some("legacy:manual"));
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let review_start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: review_task.id.clone(),
        })
        .await?;
    let review_start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(review_start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse {
        task: _review_task,
        run: review_run,
    } = to_response(review_start_response)?;
    assert_eq!(review_run.agent_id, "claude-reviewer");
    assert_eq!(review_run.thread_id, None);

    let review_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::ReviewReport
                && payload.artifact.run_id == review_run.id
            {
                return Ok::<_, anyhow::Error>(payload.artifact);
            }
        }
    })
    .await??;

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == review_run.id
                && payload.run.status == DevflowRunStatus::ReadyForReview
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    let prompt_args = std::fs::read_to_string(args_path)?;
    let review_body = std::fs::read_to_string(&review_artifact_created.path)?;
    assert!(prompt_args.contains("Dependency Task"));
    assert!(prompt_args.contains("note.txt"));
    assert!(prompt_args.contains("+after"));
    assert!(review_body.contains("Claude Review"));
    Ok(())
}

#[tokio::test]
async fn devflow_task_pause_and_resume_roundtrip() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let responses = vec![
        create_shell_command_sse_response_with_permissions(
            vec![
                "python3".to_string(),
                "-c".to_string(),
                "print('pause me')".to_string(),
            ],
            None,
            None,
            Some("require_escalated"),
            Some("Need explicit unsandboxed execution for pause"),
            "call1",
        )?,
        create_exec_command_sse_response("exec-1")?,
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: no blocking issues.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let update_policy_request_id = mcp
        .send_devflow_approval_policy_update_request(DevflowApprovalPolicyUpdateParams {
            policy: DevflowApprovalPolicy {
                low_risk_approval_policy: AskForApproval::OnRequest,
                medium_risk_approval_policy: AskForApproval::OnFailure,
                high_risk_approval_policy: AskForApproval::OnRequest,
                approvals_reviewer: ApprovalsReviewer::User,
            },
        })
        .await?;
    let update_policy_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(update_policy_request_id)),
    )
    .await??;
    let _: DevflowApprovalPolicyUpdateResponse = to_response(update_policy_response)?;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Pause and resume".to_string(),
            objective: "Pause once, then resume and change note.txt from before to after."
                .to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;

    let approval_notification: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowApproval/requested"),
    )
    .await??;
    let approval_payload: DevflowApprovalRequestedNotification =
        serde_json::from_value(approval_notification.params.expect("approval params"))?;
    assert_eq!(approval_payload.approval.run_id, run.id);

    let pause_request_id = mcp
        .send_devflow_task_pause_request(DevflowTaskPauseParams {
            id: task.id.clone(),
        })
        .await?;
    let pause_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(pause_request_id)),
    )
    .await??;
    let DevflowTaskPauseResponse {
        task: paused_task,
        run: paused_run,
    } = to_response(pause_response)?;
    assert_eq!(paused_task.status, DevflowTaskStatus::Paused);
    assert_eq!(paused_run.status, DevflowRunStatus::Cancelled);
    assert_eq!(
        paused_run.exit_reason.as_deref(),
        Some("paused by devflowTask/pause")
    );

    let approval_list_request_id = mcp
        .send_devflow_approval_list_request(DevflowApprovalListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            status: Some(DevflowApprovalStatus::Pending),
        })
        .await?;
    let approval_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(approval_list_request_id)),
    )
    .await??;
    let DevflowApprovalListResponse { data } = to_response(approval_list_response)?;
    assert_eq!(data.len(), 0);

    let resume_request_id = mcp
        .send_devflow_task_resume_request(DevflowTaskResumeParams {
            id: task.id.clone(),
        })
        .await?;
    let resume_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_request_id)),
    )
    .await??;
    let DevflowTaskResumeResponse {
        task: resumed_task,
        run: resumed_run,
    } = to_response(resume_response)?;
    assert_eq!(resumed_task.status, DevflowTaskStatus::Running);
    assert_eq!(resumed_run.status, DevflowRunStatus::Running);
    assert_ne!(resumed_run.id, run.id);

    timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == resumed_run.id
                && payload.run.status == DevflowRunStatus::ReadyToMerge
            {
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;

    Ok(())
}

#[tokio::test]
async fn devflow_task_cancel_interrupts_running_run() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let responses = vec![create_shell_command_sse_response_with_permissions(
        vec![
            "python3".to_string(),
            "-c".to_string(),
            "print('cancel me')".to_string(),
        ],
        None,
        None,
        Some("require_escalated"),
        Some("Need explicit unsandboxed execution for cancel"),
        "call1",
    )?];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let update_policy_request_id = mcp
        .send_devflow_approval_policy_update_request(DevflowApprovalPolicyUpdateParams {
            policy: DevflowApprovalPolicy {
                low_risk_approval_policy: AskForApproval::OnRequest,
                medium_risk_approval_policy: AskForApproval::OnFailure,
                high_risk_approval_policy: AskForApproval::OnRequest,
                approvals_reviewer: ApprovalsReviewer::User,
            },
        })
        .await?;
    let update_policy_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(update_policy_request_id)),
    )
    .await??;
    let _: DevflowApprovalPolicyUpdateResponse = to_response(update_policy_response)?;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Cancel a running task".to_string(),
            objective: "Start a task and cancel it while it is waiting on approval.".to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { run, .. } = to_response(start_response)?;

    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowApproval/requested"),
    )
    .await??;

    let cancel_request_id = mcp
        .send_devflow_task_cancel_request(DevflowTaskCancelParams {
            id: task.id.clone(),
        })
        .await?;
    let cancel_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(cancel_request_id)),
    )
    .await??;
    let DevflowTaskCancelResponse {
        task: cancelled_task,
        run: cancelled_run,
    } = to_response(cancel_response)?;
    assert_eq!(cancelled_task.status, DevflowTaskStatus::Cancelled);
    assert_eq!(
        cancelled_run.expect("cancelled run").status,
        DevflowRunStatus::Cancelled
    );

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: task.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;
    assert_eq!(read_task.status, DevflowTaskStatus::Cancelled);

    let approval_list_request_id = mcp
        .send_devflow_approval_list_request(DevflowApprovalListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
            status: Some(DevflowApprovalStatus::Pending),
        })
        .await?;
    let approval_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(approval_list_request_id)),
    )
    .await??;
    let DevflowApprovalListResponse { data } = to_response(approval_list_response)?;
    assert_eq!(data.len(), 0);

    Ok(())
}

#[tokio::test]
async fn devflow_task_start_streams_phase_one_loop() -> Result<()> {
    let project_root = TempDir::new()?;
    init_git_repo(project_root.path())?;
    std::fs::write(project_root.path().join("note.txt"), "before\n")?;
    run_git(project_root.path(), &["add", "note.txt"])?;
    run_git(project_root.path(), &["commit", "-m", "init"])?;

    let patch = r#"*** Begin Patch
*** Update File: note.txt
@@
-before
+after
*** End Patch
"#;
    let responses = vec![
        create_exec_command_sse_response("exec-1")?,
        create_apply_patch_sse_response(patch, "patch-1")?,
        create_final_assistant_message_sse_response("done")?,
        create_final_assistant_message_sse_response("Review: no blocking issues.")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let create_request_id = mcp
        .send_devflow_task_create_request(DevflowTaskCreateParams {
            project_root: project_root.path().display().to_string(),
            title: "Update note file".to_string(),
            objective: "Change note.txt from before to after and verify the repo state."
                .to_string(),
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::Low,
            trigger_source: None,
            dependencies: None,
            assigned_agent_id: None,
        })
        .await?;
    let create_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(create_request_id)),
    )
    .await??;
    let DevflowTaskCreateResponse { task } = to_response(create_response)?;
    let _: JSONRPCNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message("devflowTask/statusChanged"),
    )
    .await??;

    let start_request_id = mcp
        .send_devflow_task_start_request(DevflowTaskStartParams {
            id: task.id.clone(),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let DevflowTaskStartResponse { task, run } = to_response(start_response)?;

    assert_eq!(task.status, DevflowTaskStatus::Running);
    assert_eq!(run.status, DevflowRunStatus::Running);
    assert_eq!(run.agent_id, "codex-main");
    assert!(run.thread_id.is_some());
    assert!(run.turn_id.is_some());
    assert_eq!(run.artifact_ids.len(), 1);
    let worktree_id = task
        .worktree_id
        .clone()
        .expect("implementation task should default to a managed worktree");

    let artifact_created_context = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::ContextPack {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        artifact_created_context.project_id.as_str(),
        task.project_id.as_str()
    );
    assert_eq!(artifact_created_context.artifact.run_id, run.id);

    let running_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::Running {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        running_notification.project_id.as_str(),
        task.project_id.as_str()
    );
    assert_eq!(running_notification.task_id, task.id);

    let command_started = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/commandStarted")
                .await?;
            let payload: DevflowRunCommandStartedNotification =
                serde_json::from_value(notification.params.expect("command started params"))?;
            if payload.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        command_started.project_id.as_str(),
        task.project_id.as_str()
    );
    assert!(command_started.command.contains("echo hi"));

    let output_delta = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/outputDelta")
                .await?;
            let payload: DevflowRunOutputDeltaNotification =
                serde_json::from_value(notification.params.expect("output delta params"))?;
            if payload.run_id == run.id
                && payload.source == DevflowRunOutputSource::CommandExecution
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(output_delta.project_id.as_str(), task.project_id.as_str());
    assert!(!output_delta.delta.is_empty());

    let command_completed = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/commandCompleted")
                .await?;
            let payload: DevflowRunCommandCompletedNotification =
                serde_json::from_value(notification.params.expect("command completed params"))?;
            if payload.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        command_completed.project_id.as_str(),
        task.project_id.as_str()
    );
    assert_eq!(command_completed.status, "completed");

    let diff_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::Diff
                && payload.artifact.run_id == run.id
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(
        diff_artifact_created.project_id.as_str(),
        task.project_id.as_str()
    );

    let worktree_read_request_id = mcp
        .send_devflow_worktree_read_request(DevflowWorktreeReadParams {
            id: worktree_id.clone(),
        })
        .await?;
    let worktree_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(worktree_read_request_id)),
    )
    .await??;
    let DevflowWorktreeReadResponse { worktree } = to_response(worktree_read_response)?;
    assert_eq!(worktree.status, DevflowWorktreeStatus::Dirty);
    assert_eq!(
        std::fs::read_to_string(project_root.path().join("note.txt"))?,
        "before\n"
    );
    assert_eq!(
        std::fs::read_to_string(std::path::Path::new(&worktree.cwd_path).join("note.txt"))?,
        "after\n"
    );

    let diff_updated = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/diffUpdated")
                .await?;
            let payload: DevflowRunDiffUpdatedNotification =
                serde_json::from_value(notification.params.expect("diff params"))?;
            if payload.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(diff_updated.project_id.as_str(), task.project_id.as_str());
    assert_eq!(diff_updated.artifact_id, diff_artifact_created.artifact.id);
    assert!(diff_updated.diff.contains("after"));

    let worktree_diff_updated = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowWorktree/diffUpdated")
                .await?;
            let payload: DevflowWorktreeDiffUpdatedNotification =
                serde_json::from_value(notification.params.expect("worktree diff params"))?;
            if payload.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(worktree_diff_updated.project_id, task.project_id);
    assert_eq!(worktree_diff_updated.task_id, task.id);
    assert_eq!(worktree_diff_updated.worktree_id, Some(worktree_id.clone()));
    assert_eq!(
        worktree_diff_updated.artifact_id,
        diff_artifact_created.artifact.id
    );
    assert!(worktree_diff_updated.diff.contains("after"));

    let explicit_diff_request_id = mcp
        .send_devflow_worktree_diff_request(DevflowWorktreeDiffParams {
            id: worktree_id.clone(),
        })
        .await?;
    let explicit_diff_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(explicit_diff_request_id)),
    )
    .await??;
    let DevflowWorktreeDiffResponse {
        worktree: diff_worktree,
        diff: explicit_diff,
    } = to_response(explicit_diff_response)?;
    assert_eq!(diff_worktree.id, worktree_id);
    assert!(explicit_diff.contains("after"));

    let gate_completed = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowQualityGate/completed")
                .await?;
            let payload: DevflowQualityGateCompletedNotification =
                serde_json::from_value(notification.params.expect("gate params"))?;
            if payload.gate.run_id == run.id {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(gate_completed.project_id.as_str(), task.project_id.as_str());
    assert_eq!(gate_completed.gate.status, DevflowQualityGateStatus::Passed);

    let gate_list_request_id = mcp
        .send_devflow_quality_gate_list_request(DevflowQualityGateListParams {
            task_id: Some(task.id.clone()),
            run_id: Some(run.id.clone()),
        })
        .await?;
    let gate_list_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(gate_list_request_id)),
    )
    .await??;
    let DevflowQualityGateListResponse { data: gates } = to_response(gate_list_response)?;
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0].status, DevflowQualityGateStatus::Passed);

    let gate_read_request_id = mcp
        .send_devflow_quality_gate_read_request(DevflowQualityGateReadParams {
            id: gates[0].id.clone(),
        })
        .await?;
    let gate_read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(gate_read_request_id)),
    )
    .await??;
    let DevflowQualityGateReadResponse { gate } = to_response(gate_read_response)?;
    assert_eq!(gate.id, gates[0].id);
    assert_eq!(gate.status, DevflowQualityGateStatus::Passed);
    assert!(gate.artifact_id.is_some());

    let review_artifact_created = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowArtifact/created")
                .await?;
            let payload: DevflowArtifactCreatedNotification =
                serde_json::from_value(notification.params.expect("artifact params"))?;
            if payload.artifact.kind == DevflowArtifactKind::ReviewReport
                && payload.artifact.run_id == run.id
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert!(review_artifact_created.artifact.summary.contains("Review"));

    let ready_task_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowTask/statusChanged")
                .await?;
            let payload: DevflowTaskStatusChangedNotification =
                serde_json::from_value(notification.params.expect("task status params"))?;
            if payload.task.id == task.id && payload.task.status == DevflowTaskStatus::ReadyToMerge
            {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;

    let ready_run_notification = timeout(DEFAULT_TIMEOUT, async {
        loop {
            let notification = mcp
                .read_stream_until_notification_message("devflowRun/statusChanged")
                .await?;
            let payload: DevflowRunStatusChangedNotification =
                serde_json::from_value(notification.params.expect("run status params"))?;
            if payload.run.id == run.id && payload.run.status == DevflowRunStatus::ReadyToMerge {
                return Ok::<_, anyhow::Error>(payload);
            }
        }
    })
    .await??;
    assert_eq!(ready_task_notification.task.id, task.id);
    assert_eq!(ready_run_notification.task_id, task.id);

    let read_request_id = mcp
        .send_devflow_task_read_request(DevflowTaskReadParams {
            id: task.id.clone(),
        })
        .await?;
    let read_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_request_id)),
    )
    .await??;
    let DevflowTaskReadResponse { task: read_task } = to_response(read_response)?;

    assert_eq!(read_task.status, DevflowTaskStatus::ReadyToMerge);
    assert!(!read_task.run_ids.is_empty());
    assert!(read_task.artifact_ids.len() >= 2);
    assert_eq!(
        std::fs::read_to_string(project_root.path().join("note.txt"))?,
        "before\n"
    );

    let prep_request_id = mcp
        .send_devflow_release_prep_create_request(DevflowReleasePrepCreateParams {
            project_root: project_root.path().display().to_string(),
            task_id: Some(read_task.id.clone()),
        })
        .await?;
    let prep_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(prep_request_id)),
    )
    .await??;
    let DevflowReleasePrepCreateResponse {
        status,
        blockers,
        pr_body_artifact,
        ..
    } = to_response(prep_response)?;
    assert_eq!(status, DevflowReleasePrepStatus::Blocked);
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker.contains("has not been merged by Integrator"))
    );
    let pr_body = std::fs::read_to_string(&pr_body_artifact.path)?;
    assert!(pr_body.contains("## Integrator"));
    assert!(pr_body.contains("Update note file: pending Integrator merge evidence"));

    let dirty_cleanup_request_id = mcp
        .send_devflow_worktree_cleanup_request(DevflowWorktreeCleanupParams {
            id: worktree_id.clone(),
        })
        .await?;
    let dirty_cleanup_error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(dirty_cleanup_request_id)),
    )
    .await??;
    assert_eq!(dirty_cleanup_error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        dirty_cleanup_error.error.message.contains("dirty worktree"),
        "unexpected cleanup error: {}",
        dirty_cleanup_error.error.message
    );

    run_git(
        std::path::Path::new(&worktree.root_path),
        &["reset", "--hard", "HEAD"],
    )?;
    let clean_cleanup_request_id = mcp
        .send_devflow_worktree_cleanup_request(DevflowWorktreeCleanupParams {
            id: worktree_id.clone(),
        })
        .await?;
    let clean_cleanup_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(clean_cleanup_request_id)),
    )
    .await??;
    let DevflowWorktreeCleanupResponse {
        cleaned,
        worktree: cleaned_worktree,
    } = to_response(clean_cleanup_response)?;
    assert!(cleaned);
    assert_eq!(cleaned_worktree.status, DevflowWorktreeStatus::Cleaned);
    assert!(!std::path::Path::new(&cleaned_worktree.root_path).exists());

    Ok(())
}
