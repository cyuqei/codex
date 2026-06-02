use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use chrono::Utc;
use codex_app_server_protocol::DevflowApproval;
use codex_app_server_protocol::DevflowApprovalPolicy;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowProject;
use codex_app_server_protocol::DevflowQualityGate;
use codex_app_server_protocol::DevflowRun;
use codex_app_server_protocol::DevflowSupportBundle;
use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowTaskKind;
use codex_app_server_protocol::DevflowWatchdogAlert;
use codex_app_server_protocol::DevflowWorktree;
use serde_json::Value;
use serde_json::json;
use tokio::fs;
use uuid::Uuid;

pub(crate) struct DevflowSupportBundleInput {
    pub(crate) project: DevflowProject,
    pub(crate) task_id: Option<String>,
    pub(crate) tasks: Vec<DevflowTask>,
    pub(crate) runs: Vec<DevflowRun>,
    pub(crate) quality_gates: Vec<DevflowQualityGate>,
    pub(crate) approvals: Vec<DevflowApproval>,
    pub(crate) artifacts: Vec<DevflowArtifact>,
    pub(crate) worktrees: Vec<DevflowWorktree>,
    pub(crate) watchdog_alerts: Vec<DevflowWatchdogAlert>,
    pub(crate) watchdog_queue: Value,
    pub(crate) approval_policy: DevflowApprovalPolicy,
    pub(crate) approval_policy_load_error: Option<String>,
    pub(crate) store_snapshot_path: String,
    pub(crate) store_snapshot_load_error: Option<String>,
    pub(crate) store_snapshot_persist_error: Option<String>,
}

pub(crate) async fn create_devflow_support_bundle(
    input: DevflowSupportBundleInput,
) -> Result<DevflowSupportBundle, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().timestamp();
    let project_id = input.project.id.clone();
    let path = support_bundle_path(&input.project.root_path, &id);
    let diagnostics = support_bundle_diagnostics(&input);
    let release_prep = release_prep_diagnostics(&input);
    let persistence = persistence_diagnostics(&input).await;
    let summary = format!(
        "Devflow support bundle with {} tasks, {} runs, {} artifacts, {} watchdog alerts",
        input.tasks.len(),
        input.runs.len(),
        input.artifacts.len(),
        input.watchdog_alerts.len()
    );
    let content = serde_json::to_string_pretty(&serde_json::json!({
        "schemaVersion": 1,
        "runner": "codex-devflow-support-bundle",
        "id": id,
        "project": &input.project,
        "scope": {
            "taskId": &input.task_id,
        },
        "counts": {
            "tasks": input.tasks.len(),
            "runs": input.runs.len(),
            "qualityGates": input.quality_gates.len(),
            "approvals": input.approvals.len(),
            "artifacts": input.artifacts.len(),
            "worktrees": input.worktrees.len(),
            "watchdogAlerts": input.watchdog_alerts.len(),
        },
        "tasks": &input.tasks,
        "runs": &input.runs,
        "qualityGates": &input.quality_gates,
        "approvals": &input.approvals,
        "artifacts": &input.artifacts,
        "worktrees": &input.worktrees,
        "watchdog": {
            "alerts": &input.watchdog_alerts,
            "queue": &input.watchdog_queue,
        },
        "releasePrep": release_prep,
        "persistence": persistence,
        "approvalPolicy": &input.approval_policy,
        "approvalPolicyLoadError": &input.approval_policy_load_error,
        "storeSnapshotLoadError": &input.store_snapshot_load_error,
        "storeSnapshotPersistError": &input.store_snapshot_persist_error,
        "diagnostics": &diagnostics,
        "reproduction": {
            "projectDiagnose": "devflowProject/diagnose",
            "taskList": "devflowTask/list",
            "artifactList": "devflowArtifact/list",
            "releasePrepCreate": "devflowReleasePrep/create",
            "releasePrepSubmit": "devflowReleasePrep/submit",
            "watchdogRead": "devflowWatchdog/read",
        },
        "createdAt": created_at,
    }))
    .map_err(|err| format!("failed to serialize support bundle: {err}"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create support bundle dir: {err}"))?;
    }
    fs::write(&path, content)
        .await
        .map_err(|err| format!("failed to write support bundle: {err}"))?;

    Ok(DevflowSupportBundle {
        id,
        project_id,
        path: path.display().to_string(),
        mime_type: "application/json".to_string(),
        summary,
        diagnostics,
        created_at,
    })
}

fn support_bundle_path(project_root: &str, id: &str) -> PathBuf {
    PathBuf::from(project_root)
        .join(".codex")
        .join("devflow")
        .join("support-bundles")
        .join(format!("support-bundle-{id}.json"))
}

fn support_bundle_diagnostics(input: &DevflowSupportBundleInput) -> Vec<String> {
    let mut diagnostics = vec![
        format!(
            "project diagnostics included for {}",
            input.project.root_path
        ),
        "artifact contents are not inlined; artifact metadata includes source paths".to_string(),
    ];
    if let Some(task_id) = &input.task_id {
        diagnostics.push(format!("bundle scoped to task {task_id}"));
    } else {
        diagnostics.push("bundle scoped to project".to_string());
    }
    if let Some(error) = input.approval_policy_load_error.as_ref() {
        diagnostics.push(format!(
            "approval policy could not be loaded; Devflow execution should fail closed until CODEX_HOME/devflow/approval-policy.json is fixed: {error}"
        ));
    }
    if let Some(error) = input.store_snapshot_load_error.as_ref() {
        diagnostics.push(format!(
            "devflow store snapshot could not be restored; in-memory task/run/gate/artifact indexes started empty: {error}"
        ));
    }
    if let Some(error) = input.store_snapshot_persist_error.as_ref() {
        diagnostics.push(format!(
            "devflow store snapshot could not be persisted; recent task/run/gate/artifact updates may not survive restart: {error}"
        ));
    }
    if input.tasks.is_empty() {
        diagnostics.push("no in-memory devflow tasks matched the bundle scope".to_string());
    }
    if input.worktrees.iter().any(|worktree| !worktree.managed) {
        diagnostics.push(
            "unmanaged worktrees are reported but never cleaned up by this bundle".to_string(),
        );
    }
    if input.tasks.iter().any(|task| {
        task.kind == DevflowTaskKind::Implementation
            && task.worktree_id.is_some()
            && successful_integrator_merge_artifact(input, task).is_none()
    }) {
        diagnostics.push(
            "one or more implementation tasks are missing successful Integrator merge evidence"
                .to_string(),
        );
    }
    diagnostics
}

async fn persistence_diagnostics(input: &DevflowSupportBundleInput) -> Value {
    let status = if input.store_snapshot_load_error.is_some()
        || input.store_snapshot_persist_error.is_some()
    {
        "degraded"
    } else {
        "ok"
    };
    let snapshot_file = snapshot_file_diagnostics(&input.store_snapshot_path).await;
    json!({
        "status": status,
        "storeSnapshotPath": &input.store_snapshot_path,
        "snapshotFile": snapshot_file,
        "loadError": &input.store_snapshot_load_error,
        "persistError": &input.store_snapshot_persist_error,
        "recoverableIndexes": {
            "tasks": input.tasks.len(),
            "runs": input.runs.len(),
            "qualityGates": input.quality_gates.len(),
            "approvals": input.approvals.len(),
            "artifacts": input.artifacts.len(),
            "watchdogAlerts": input.watchdog_alerts.len(),
        },
        "volatileProcessState": [
            "approval grants",
            "active approval callbacks",
            "live thread subscriptions",
        ],
        "recoverySemantics": {
            "pendingApprovals": "restored fail-closed as responded/cancelled audit records",
            "activeRunsAndGates": "restored fail-closed as failed/blocked records",
            "persistFailureAlerts": "projected as non-persisted recovering watchdog alerts while active",
        },
    })
}

async fn snapshot_file_diagnostics(path: &str) -> Value {
    match fs::metadata(path).await {
        Ok(metadata) => json!({
            "path": path,
            "metadataAvailable": true,
            "sizeBytes": metadata.len(),
            "modifiedAt": metadata
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| i64::try_from(duration.as_secs()).ok()),
            "metadataError": Option::<String>::None,
        }),
        Err(err) => json!({
            "path": path,
            "metadataAvailable": false,
            "sizeBytes": Option::<u64>::None,
            "modifiedAt": Option::<i64>::None,
            "metadataError": err.to_string(),
        }),
    }
}

fn release_prep_diagnostics(input: &DevflowSupportBundleInput) -> Value {
    let release_artifacts = input
        .artifacts
        .iter()
        .filter(|artifact| is_release_prep_artifact(artifact))
        .map(|artifact| {
            json!({
                "id": &artifact.id,
                "title": &artifact.title,
                "path": &artifact.path,
                "summary": &artifact.summary,
            })
        })
        .collect::<Vec<_>>();
    let publish_reports = input
        .artifacts
        .iter()
        .filter(|artifact| artifact.title.starts_with("Release publish report for "))
        .map(|artifact| {
            json!({
                "id": &artifact.id,
                "title": &artifact.title,
                "path": &artifact.path,
                "summary": &artifact.summary,
            })
        })
        .collect::<Vec<_>>();
    let integrator_tasks = input
        .tasks
        .iter()
        .filter(|task| task.kind == DevflowTaskKind::Implementation)
        .map(|task| {
            let merge_artifact = successful_integrator_merge_artifact(input, task);
            let status = if merge_artifact.is_some() {
                "merged"
            } else if task.worktree_id.is_some() {
                "pending_integrator_merge"
            } else {
                "no_managed_worktree"
            };
            json!({
                "taskId": &task.id,
                "title": &task.title,
                "status": status,
                "worktreeId": &task.worktree_id,
                "mergeArtifactId": merge_artifact.map(|artifact| artifact.id.clone()),
                "mergeArtifactPath": merge_artifact.map(|artifact| artifact.path.clone()),
            })
        })
        .collect::<Vec<_>>();
    let merged = integrator_tasks
        .iter()
        .filter(|task| task["status"] == "merged")
        .count();
    let pending = integrator_tasks
        .iter()
        .filter(|task| task["status"] == "pending_integrator_merge")
        .count();
    let no_managed_worktree = integrator_tasks
        .iter()
        .filter(|task| task["status"] == "no_managed_worktree")
        .count();
    json!({
        "artifacts": release_artifacts,
        "publishReports": publish_reports,
        "integrator": {
            "counts": {
                "merged": merged,
                "pending": pending,
                "noManagedWorktree": no_managed_worktree,
            },
            "tasks": integrator_tasks,
        },
        "reproduction": {
            "create": "devflowReleasePrep/create",
            "submit": "devflowReleasePrep/submit",
            "readArtifacts": "devflowArtifact/read",
        },
    })
}

fn is_release_prep_artifact(artifact: &DevflowArtifact) -> bool {
    artifact.title.starts_with("Commit message for ")
        || artifact.title.starts_with("PR body for ")
        || artifact.title.starts_with("Release notes for ")
}

fn successful_integrator_merge_artifact<'a>(
    input: &'a DevflowSupportBundleInput,
    task: &DevflowTask,
) -> Option<&'a DevflowArtifact> {
    input.artifacts.iter().find(|artifact| {
        artifact.task_id == task.id
            && artifact.title.starts_with("Integrator merge report")
            && artifact.summary.starts_with("Integrator merged ")
    })
}
