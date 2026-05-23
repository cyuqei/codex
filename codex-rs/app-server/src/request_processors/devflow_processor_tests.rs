use std::collections::HashSet;
use std::sync::Arc;

use crate::config_manager::ConfigManager;
use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::OutgoingEnvelope;
use crate::outgoing_message::OutgoingMessage;
use crate::outgoing_message::OutgoingMessageSender;
use crate::request_processors::devflow_processor::DevflowApprovalStatus;
use crate::request_processors::devflow_processor::DevflowRequestProcessor;
use crate::request_processors::devflow_processor::DevflowRunRecord;
use crate::thread_state::ThreadStateManager;
use crate::thread_status::ThreadWatchManager;
use anyhow::Result;
use codex_analytics::AnalyticsEventsClient;
use codex_app_server_protocol::AdditionalFileSystemPermissions;
use codex_app_server_protocol::DevflowApprovalDecision;
use codex_app_server_protocol::DevflowApprovalKind;
use codex_app_server_protocol::DevflowApprovalListParams;
use codex_app_server_protocol::DevflowApprovalRequestedNotification;
use codex_app_server_protocol::DevflowApprovalRespondParams;
use codex_app_server_protocol::DevflowArtifactCreatedNotification;
use codex_app_server_protocol::DevflowArtifactKind;
use codex_app_server_protocol::DevflowRun;
use codex_app_server_protocol::DevflowRunStatus;
use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowTaskKind;
use codex_app_server_protocol::DevflowTaskRiskLevel;
use codex_app_server_protocol::DevflowTaskStatus;
use codex_app_server_protocol::FileSystemAccessMode;
use codex_app_server_protocol::FileSystemPath;
use codex_app_server_protocol::FileSystemSandboxEntry;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::PermissionGrantScope;
use codex_app_server_protocol::PermissionsRequestApprovalParams;
use codex_app_server_protocol::PermissionsRequestApprovalResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::RequestPermissionProfile;
use codex_app_server_protocol::ReviewCodeLocation;
use codex_app_server_protocol::ReviewFinding;
use codex_app_server_protocol::ReviewLineRange;
use codex_app_server_protocol::ReviewOutput;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequestPayload;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnItemsView;
use codex_app_server_protocol::TurnStatus;
use codex_arg0::Arg0DispatchPaths;
use codex_config::CloudRequirementsLoader;
use codex_config::LoaderOverrides;
use codex_core::config::ConfigBuilder;
use codex_exec_server::EnvironmentManager;
use codex_login::CodexAuth;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde_json::from_value;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

fn absolute_path(path: &std::path::Path) -> AbsolutePathBuf {
    AbsolutePathBuf::try_from(path.to_path_buf()).expect("absolute path")
}

async fn build_processor() -> Result<(
    DevflowRequestProcessor,
    Arc<OutgoingMessageSender>,
    mpsc::Receiver<OutgoingEnvelope>,
)> {
    let codex_home = TempDir::new()?;
    let config = Arc::new(
        ConfigBuilder::default()
            .codex_home(codex_home.path().to_path_buf())
            .build()
            .await?,
    );
    let config_manager = ConfigManager::new(
        config.codex_home.to_path_buf(),
        Vec::new(),
        LoaderOverrides::default(),
        CloudRequirementsLoader::default(),
        Arg0DispatchPaths::default(),
        Arc::new(codex_config::NoopThreadConfigLoader),
    );
    let thread_manager = Arc::new(
        codex_core::test_support::thread_manager_with_models_provider_and_home(
            CodexAuth::from_api_key("dummy"),
            config.model_provider.clone(),
            config.codex_home.to_path_buf(),
            Arc::new(EnvironmentManager::default_for_tests()),
        )
        .await,
    );
    let (tx, rx) = mpsc::channel(16);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        tx,
        AnalyticsEventsClient::disabled(),
    ));
    let processor = DevflowRequestProcessor::new(
        Arc::clone(&outgoing),
        Arg0DispatchPaths::default(),
        Arc::clone(&config),
        config_manager,
        thread_manager,
        ThreadStateManager::new(),
        Arc::new(Mutex::new(HashSet::new())),
        AnalyticsEventsClient::disabled(),
        ThreadWatchManager::new(),
        Arc::new(Semaphore::new(1)),
    );
    Ok((processor, outgoing, rx))
}

async fn seed_task_and_run(
    processor: &DevflowRequestProcessor,
    project_root: &str,
) -> Result<(String, String)> {
    seed_task_and_run_for_thread(
        processor,
        project_root,
        "thread-1",
        "turn-1",
        ConnectionId(1),
    )
    .await
}

async fn seed_task_and_run_for_thread(
    processor: &DevflowRequestProcessor,
    project_root: &str,
    thread_id: &str,
    turn_id: &str,
    internal_connection_id: ConnectionId,
) -> Result<(String, String)> {
    let task_id = Uuid::new_v4().to_string();
    let run_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    let task = DevflowTask {
        id: task_id.clone(),
        project_id: project_root.to_string(),
        title: "Approval test".to_string(),
        objective: "Validate projected approvals.".to_string(),
        trigger_source: None,
        status: DevflowTaskStatus::Running,
        kind: DevflowTaskKind::Implementation,
        risk_level: DevflowTaskRiskLevel::Low,
        dependencies: Vec::new(),
        assigned_agent_id: Some("codex-main".to_string()),
        worktree_id: None,
        context_pack_id: None,
        run_ids: vec![run_id.clone()],
        artifact_ids: Vec::new(),
        created_at: now,
        updated_at: now,
    };
    let run = DevflowRun {
        id: run_id.clone(),
        task_id: task_id.clone(),
        agent_id: "codex-main".to_string(),
        thread_id: Some(thread_id.to_string()),
        turn_id: Some(turn_id.to_string()),
        status: DevflowRunStatus::Running,
        started_at: now,
        completed_at: None,
        input: "test".to_string(),
        stream_summary: None,
        command_ids: Vec::new(),
        artifact_ids: Vec::new(),
        exit_reason: None,
    };
    let mut store = processor.store.lock().await;
    store.tasks.insert(task_id.clone(), task);
    store.runs.insert(
        run_id.clone(),
        DevflowRunRecord {
            run,
            project_root: project_root.to_string(),
            internal_connection_id,
            diff_artifact_id: None,
            summary_artifact_id: None,
            output_archive_artifact_id: None,
            review_artifact_id: None,
            quality_gate_id: None,
            review_requested: false,
            review_completed: false,
            auto_repair_attempt: 0,
            auto_integrator_merge: false,
            requested_stop: None,
        },
    );
    store
        .thread_to_run
        .insert(thread_id.to_string(), run_id.clone());
    Ok((task_id, run_id))
}

fn approval_test_permissions_params(
    root: &std::path::Path,
    thread_id: &str,
    turn_id: &str,
    item_id: &str,
) -> PermissionsRequestApprovalParams {
    PermissionsRequestApprovalParams {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.to_string(),
        item_id: item_id.to_string(),
        cwd: absolute_path(root),
        reason: Some("Need write access".to_string()),
        permissions: RequestPermissionProfile {
            network: None,
            file_system: Some(AdditionalFileSystemPermissions {
                read: None,
                write: Some(vec![absolute_path(root)]),
                glob_scan_max_depth: None,
                entries: Some(vec![FileSystemSandboxEntry {
                    path: FileSystemPath::Path {
                        path: absolute_path(root),
                    },
                    access: FileSystemAccessMode::Write,
                }]),
            }),
        },
    }
}

async fn read_devflow_approval_requested(
    rx: &mut mpsc::Receiver<OutgoingEnvelope>,
) -> Result<DevflowApprovalRequestedNotification> {
    timeout(Duration::from_secs(5), async {
        loop {
            let envelope = rx.recv().await.expect("outgoing envelope");
            let OutgoingEnvelope::Broadcast { message } = envelope else {
                continue;
            };
            let OutgoingMessage::AppServerNotification(
                ServerNotification::DevflowApprovalRequested(notification),
            ) = message
            else {
                continue;
            };
            return notification;
        }
    })
    .await
    .map_err(Into::into)
}

#[tokio::test]
async fn projects_permissions_approvals_and_responds() -> Result<()> {
    let tempdir = TempDir::new()?;
    let (processor, outgoing, mut rx) = build_processor().await?;
    let (task_id, run_id) =
        seed_task_and_run(&processor, &tempdir.path().display().to_string()).await?;

    let params = approval_test_permissions_params(tempdir.path(), "thread-1", "turn-1", "call-1");
    let (request_id, response_rx) = outgoing
        .send_request(ServerRequestPayload::PermissionsRequestApproval(params))
        .await;

    let requested = read_devflow_approval_requested(&mut rx).await?;
    let DevflowApprovalRequestedNotification { approval } = requested;
    assert_eq!(approval.task_id, task_id);
    assert_eq!(approval.run_id, run_id);
    assert_eq!(approval.kind, DevflowApprovalKind::Permissions);
    assert_eq!(approval.status, DevflowApprovalStatus::Pending);
    assert!(approval.requested_permissions.is_some());

    let list = processor
        .approval_list(DevflowApprovalListParams {
            task_id: Some(task_id.clone()),
            run_id: Some(run_id.clone()),
            status: Some(DevflowApprovalStatus::Pending),
        })
        .await
        .map_err(|err| anyhow::anyhow!(err.message))?;
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, approval.id);

    let responded = processor
        .approval_respond(DevflowApprovalRespondParams {
            id: approval.id.clone(),
            decision: DevflowApprovalDecision::Accept,
            scope: Some(PermissionGrantScope::Turn),
        })
        .await
        .map_err(|err| anyhow::anyhow!(err.message))?;
    assert_eq!(responded.approval.status, DevflowApprovalStatus::Responded);
    assert_eq!(
        responded.approval.decision,
        Some(DevflowApprovalDecision::Accept)
    );

    let result = timeout(Duration::from_secs(5), response_rx).await??;
    let value = result.map_err(|err: JSONRPCErrorError| anyhow::anyhow!(err.message))?;
    let response: PermissionsRequestApprovalResponse = from_value(value)?;
    assert_eq!(response.scope, PermissionGrantScope::Turn);
    assert!(response.permissions.file_system.is_some());
    assert_eq!(request_id, RequestId::Integer(0));
    Ok(())
}

#[tokio::test]
async fn accept_for_task_auto_responds_to_matching_approval_in_same_task() -> Result<()> {
    let tempdir = TempDir::new()?;
    let (processor, outgoing, mut rx) = build_processor().await?;
    let (task_id, run_id) =
        seed_task_and_run(&processor, &tempdir.path().display().to_string()).await?;

    let first_params =
        approval_test_permissions_params(tempdir.path(), "thread-1", "turn-1", "call-1");
    let (_, first_response_rx) = outgoing
        .send_request(ServerRequestPayload::PermissionsRequestApproval(
            first_params,
        ))
        .await;
    let requested = read_devflow_approval_requested(&mut rx).await?;
    let DevflowApprovalRequestedNotification { approval } = requested;

    let responded = processor
        .approval_respond(DevflowApprovalRespondParams {
            id: approval.id.clone(),
            decision: DevflowApprovalDecision::AcceptForTask,
            scope: None,
        })
        .await
        .map_err(|err| anyhow::anyhow!(err.message))?;
    assert_eq!(
        responded.approval.decision,
        Some(DevflowApprovalDecision::AcceptForTask)
    );
    let first_result = timeout(Duration::from_secs(5), first_response_rx).await??;
    let first_value =
        first_result.map_err(|err: JSONRPCErrorError| anyhow::anyhow!(err.message))?;
    let first_response: PermissionsRequestApprovalResponse = from_value(first_value)?;
    assert_eq!(first_response.scope, PermissionGrantScope::Turn);

    let second_params =
        approval_test_permissions_params(tempdir.path(), "thread-1", "turn-1", "call-2");
    let (_, second_response_rx) = outgoing
        .send_request(ServerRequestPayload::PermissionsRequestApproval(
            second_params,
        ))
        .await;
    let second_result = timeout(Duration::from_secs(5), second_response_rx).await??;
    let second_value =
        second_result.map_err(|err: JSONRPCErrorError| anyhow::anyhow!(err.message))?;
    let second_response: PermissionsRequestApprovalResponse = from_value(second_value)?;
    assert_eq!(second_response.scope, PermissionGrantScope::Turn);
    assert!(second_response.permissions.file_system.is_some());

    let list = processor
        .approval_list(DevflowApprovalListParams {
            task_id: Some(task_id.clone()),
            run_id: Some(run_id.clone()),
            status: Some(DevflowApprovalStatus::Responded),
        })
        .await
        .map_err(|err| anyhow::anyhow!(err.message))?;
    assert_eq!(list.data.len(), 2);
    assert!(
        list.data
            .iter()
            .all(|approval| approval.decision == Some(DevflowApprovalDecision::AcceptForTask))
    );
    Ok(())
}

#[tokio::test]
async fn accept_for_project_auto_responds_to_matching_approval_in_sibling_task() -> Result<()> {
    let tempdir = TempDir::new()?;
    let project_root = tempdir.path().display().to_string();
    let (processor, outgoing, mut rx) = build_processor().await?;
    let (_first_task_id, _first_run_id) = seed_task_and_run(&processor, &project_root).await?;

    let first_params =
        approval_test_permissions_params(tempdir.path(), "thread-1", "turn-1", "call-1");
    let (_, first_response_rx) = outgoing
        .send_request(ServerRequestPayload::PermissionsRequestApproval(
            first_params,
        ))
        .await;
    let requested = read_devflow_approval_requested(&mut rx).await?;
    let DevflowApprovalRequestedNotification { approval } = requested;

    processor
        .approval_respond(DevflowApprovalRespondParams {
            id: approval.id.clone(),
            decision: DevflowApprovalDecision::AcceptForProject,
            scope: None,
        })
        .await
        .map_err(|err| anyhow::anyhow!(err.message))?;
    let first_result = timeout(Duration::from_secs(5), first_response_rx).await??;
    let first_value =
        first_result.map_err(|err: JSONRPCErrorError| anyhow::anyhow!(err.message))?;
    let first_response: PermissionsRequestApprovalResponse = from_value(first_value)?;
    assert_eq!(first_response.scope, PermissionGrantScope::Turn);

    let (second_task_id, second_run_id) = seed_task_and_run_for_thread(
        &processor,
        &project_root,
        "thread-2",
        "turn-2",
        ConnectionId(2),
    )
    .await?;
    let second_params =
        approval_test_permissions_params(tempdir.path(), "thread-2", "turn-2", "call-2");
    let (_, second_response_rx) = outgoing
        .send_request(ServerRequestPayload::PermissionsRequestApproval(
            second_params,
        ))
        .await;
    let second_result = timeout(Duration::from_secs(5), second_response_rx).await??;
    let second_value =
        second_result.map_err(|err: JSONRPCErrorError| anyhow::anyhow!(err.message))?;
    let second_response: PermissionsRequestApprovalResponse = from_value(second_value)?;
    assert_eq!(second_response.scope, PermissionGrantScope::Turn);
    assert!(second_response.permissions.file_system.is_some());

    let list = processor
        .approval_list(DevflowApprovalListParams {
            task_id: Some(second_task_id),
            run_id: Some(second_run_id),
            status: Some(DevflowApprovalStatus::Responded),
        })
        .await
        .map_err(|err| anyhow::anyhow!(err.message))?;
    assert_eq!(list.data.len(), 1);
    assert_eq!(
        list.data[0].decision,
        Some(DevflowApprovalDecision::AcceptForProject)
    );
    Ok(())
}

#[tokio::test]
async fn review_turn_completion_preserves_structured_review_output() -> Result<()> {
    let tempdir = TempDir::new()?;
    let (processor, _outgoing, mut rx) = build_processor().await?;
    let (task_id, run_id) = seed_task_and_run_for_thread(
        &processor,
        &tempdir.path().display().to_string(),
        "thread-1",
        "turn-1",
        ConnectionId(1),
    )
    .await?;

    {
        let mut store = processor.store.lock().await;
        let task = store
            .tasks
            .get_mut(&task_id)
            .expect("seeded task should exist");
        task.kind = DevflowTaskKind::Review;
        task.title = "Codex reviewer".to_string();
        task.objective = "Review the diff and preserve structured findings.".to_string();
        task.assigned_agent_id = Some("codex-reviewer".to_string());
        let run = store
            .runs
            .get_mut(&run_id)
            .expect("seeded run should exist");
        run.run.agent_id = "codex-reviewer".to_string();
    }

    let review_output = ReviewOutput {
        findings: vec![ReviewFinding {
            title: "Prefer explicit review finding state".to_string(),
            body: "Keep structured findings in the persisted ReviewReport.".to_string(),
            confidence_score: 0.95,
            priority: 1,
            code_location: ReviewCodeLocation {
                absolute_file_path: tempdir.path().join("note.txt").display().to_string(),
                line_range: ReviewLineRange { start: 1, end: 1 },
            },
        }],
        overall_correctness: "needs_work".to_string(),
        overall_explanation: "Structured reviewer output should survive turn completion."
            .to_string(),
        overall_confidence_score: 0.82,
    };

    processor
        .handle_item_completed(ItemCompletedNotification {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            completed_at_ms: 1,
            item: ThreadItem::ExitedReviewMode {
                id: "turn-1".to_string(),
                review: "Structured reviewer output should survive turn completion.".to_string(),
                review_output: Some(review_output.clone()),
            },
        })
        .await;

    let review_artifact = timeout(Duration::from_secs(5), async {
        loop {
            let envelope = rx.recv().await.expect("outgoing envelope");
            let OutgoingEnvelope::Broadcast { message } = envelope else {
                continue;
            };
            let OutgoingMessage::AppServerNotification(ServerNotification::DevflowArtifactCreated(
                notification,
            )) = message
            else {
                continue;
            };
            let DevflowArtifactCreatedNotification { artifact, .. } = notification;
            if artifact.kind == DevflowArtifactKind::ReviewReport {
                return Ok::<_, anyhow::Error>(artifact);
            }
        }
    })
    .await??;
    assert!(review_artifact.summary.contains("status=open"));

    let artifact_id = review_artifact.id.clone();
    let before_contents = std::fs::read_to_string(&review_artifact.path)?;
    assert!(before_contents.contains("Prefer explicit review finding state"));
    assert!(before_contents.contains("Structured reviewer output should survive turn completion."));

    processor
        .handle_turn_completed(TurnCompletedNotification {
            thread_id: "thread-1".to_string(),
            turn: Turn {
                id: "turn-1".to_string(),
                items: vec![ThreadItem::AgentMessage {
                    id: "assistant-1".to_string(),
                    text: "fallback text that should not overwrite structured findings".to_string(),
                    phase: None,
                    memory_citation: None,
                }],
                items_view: TurnItemsView::Full,
                status: TurnStatus::Completed,
                error: None,
                started_at: None,
                completed_at: Some(2),
                duration_ms: Some(1),
            },
        })
        .await;

    let after_contents = std::fs::read_to_string(&review_artifact.path)?;
    assert_eq!(before_contents, after_contents);

    let store = processor.store.lock().await;
    let artifact = store
        .artifacts
        .get(&artifact_id)
        .expect("review artifact should remain stored");
    assert_eq!(artifact.summary, review_artifact.summary);
    assert_eq!(artifact.id, review_artifact.id);
    Ok(())
}
