use std::path::Path;

use chrono::Utc;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowArtifactKind;
use codex_app_server_protocol::DevflowProject;
use codex_app_server_protocol::DevflowQualityGate;
use codex_app_server_protocol::DevflowQualityGateKind;
use codex_app_server_protocol::DevflowQualityGateStatus;
use codex_app_server_protocol::DevflowReleasePrepStatus;
use codex_app_server_protocol::DevflowRun;
use codex_app_server_protocol::DevflowRunStatus;
use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowTaskKind;
use codex_app_server_protocol::DevflowTaskStatus;
use serde::Serialize;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

use super::devflow_policy_requirements::required_quality_gates;
use super::devflow_review_findings::review_artifact_all_findings_addressed;
use super::devflow_review_findings::review_artifact_has_finding_state;
use super::devflow_root_cause::root_cause_artifact_has_state;
use super::devflow_root_cause::root_cause_artifact_is_accepted;
use super::devflow_root_cause::task_requires_root_cause;

const RELEASE_PREP_GIT_TIMEOUT_SECS: u64 = 15;

pub(crate) struct DevflowReleasePrepInput {
    pub(crate) project: DevflowProject,
    pub(crate) anchor_task: DevflowTask,
    pub(crate) run_id: String,
    pub(crate) tasks: Vec<DevflowTask>,
    pub(crate) runs: Vec<DevflowRun>,
    pub(crate) quality_gates: Vec<DevflowQualityGate>,
    pub(crate) artifacts: Vec<DevflowArtifact>,
    pub(crate) store_snapshot_load_error: Option<String>,
    pub(crate) store_snapshot_persist_error: Option<String>,
}

pub(crate) struct DevflowReleasePrepDraft {
    pub(crate) status: DevflowReleasePrepStatus,
    pub(crate) summary: String,
    pub(crate) blockers: Vec<String>,
    pub(crate) commit_message_artifact: DevflowArtifact,
    pub(crate) pr_body_artifact: DevflowArtifact,
    pub(crate) release_note_artifact: DevflowArtifact,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DevflowReleaseGitSnapshot {
    branch: Option<String>,
    origin: Option<String>,
    status_short: String,
    diff_stat: String,
    diff_check: String,
    diff_check_ok: bool,
    diagnostics: Vec<String>,
}

pub(crate) async fn create_devflow_release_prep(
    input: DevflowReleasePrepInput,
) -> Result<DevflowReleasePrepDraft, String> {
    let git = git_snapshot(Path::new(&input.project.root_path)).await;
    let blockers = finish_branch_blockers(&input, &git);
    let status = if blockers.is_empty() {
        DevflowReleasePrepStatus::Ready
    } else {
        DevflowReleasePrepStatus::Blocked
    };
    let summary = format!(
        "Release prep {} with {} tasks, {} quality gates, {} artifacts",
        release_status_label(status),
        input.tasks.len(),
        input.quality_gates.len(),
        input.artifacts.len()
    );

    let commit_message = render_commit_message(&input, status, &blockers);
    let pr_body = render_pr_body(&input, status, &blockers, &git);
    let release_notes = render_release_notes(&input, status);

    let commit_message_artifact =
        write_release_artifact(&input, "commit-message", "Commit message", &commit_message).await?;
    let pr_body_artifact = write_release_artifact(&input, "pr-body", "PR body", &pr_body).await?;
    let release_note_artifact =
        write_release_artifact(&input, "release-notes", "Release notes", &release_notes).await?;

    Ok(DevflowReleasePrepDraft {
        status,
        summary,
        blockers,
        commit_message_artifact,
        pr_body_artifact,
        release_note_artifact,
    })
}

async fn write_release_artifact(
    input: &DevflowReleasePrepInput,
    suffix: &str,
    title: &str,
    contents: &str,
) -> Result<DevflowArtifact, String> {
    let artifact_id = Uuid::new_v4().to_string();
    let run_id = &input.run_id;
    let path = Path::new(&input.project.root_path)
        .join(".codex")
        .join("devflow")
        .join("artifacts")
        .join(format!("{run_id}-{suffix}-{artifact_id}.md"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create release prep artifact dir: {err}"))?;
    }
    fs::write(&path, contents)
        .await
        .map_err(|err| format!("failed to write release prep artifact: {err}"))?;

    Ok(DevflowArtifact {
        id: artifact_id,
        task_id: input.anchor_task.id.clone(),
        run_id: input.run_id.clone(),
        kind: DevflowArtifactKind::Report,
        title: format!("{title} for {}", input.anchor_task.title),
        path: path.display().to_string(),
        mime_type: "text/markdown".to_string(),
        summary: {
            const LIMIT: usize = 400;
            if contents.len() <= LIMIT {
                contents.to_string()
            } else {
                let prefix = contents.chars().take(LIMIT).collect::<String>();
                format!("{prefix}...")
            }
        },
        created_at: Utc::now().timestamp(),
    })
}

fn finish_branch_blockers(
    input: &DevflowReleasePrepInput,
    git: &DevflowReleaseGitSnapshot,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !Path::new(&input.project.root_path).join(".git").exists() {
        blockers.push("project is not inside a git repository".to_string());
    }
    if !git.diff_check_ok {
        blockers.push("git diff --check did not pass".to_string());
    }
    if let Some(error) = input.store_snapshot_load_error.as_ref() {
        blockers.push(format!(
            "devflow store snapshot could not be restored; runtime indexes may be incomplete: {error}"
        ));
    }
    if let Some(error) = input.store_snapshot_persist_error.as_ref() {
        blockers.push(format!(
            "devflow store snapshot could not be persisted; recent runtime indexes may not survive restart: {error}"
        ));
    }
    for task in &input.tasks {
        match task.status {
            DevflowTaskStatus::ReadyToMerge => {}
            DevflowTaskStatus::ReadyForReview if task.kind != DevflowTaskKind::Implementation => {}
            DevflowTaskStatus::ReadyForReview => {
                blockers.push(format!(
                    "task '{}' is ready for review but not ready to merge",
                    task.title
                ));
            }
            DevflowTaskStatus::Planned => {
                blockers.push(format!("task '{}' is still planned", task.title));
            }
            DevflowTaskStatus::Running => {
                blockers.push(format!("task '{}' is still running", task.title));
            }
            DevflowTaskStatus::Paused => {
                blockers.push(format!("task '{}' is paused", task.title));
            }
            DevflowTaskStatus::Failed => {
                blockers.push(format!("task '{}' failed", task.title));
            }
            DevflowTaskStatus::Blocked => {
                blockers.push(format!("task '{}' is blocked", task.title));
            }
            DevflowTaskStatus::Cancelled => {
                blockers.push(format!("task '{}' was cancelled", task.title));
            }
        }
        if task.kind == DevflowTaskKind::Implementation
            && task.worktree_id.is_some()
            && !has_successful_integrator_merge_artifact(input, task)
        {
            blockers.push(format!(
                "task '{}' has not been merged by Integrator",
                task.title
            ));
        }
        if task_requires_root_cause(task)
            && matches!(
                task.status,
                DevflowTaskStatus::ReadyForReview | DevflowTaskStatus::ReadyToMerge
            )
        {
            match root_cause_artifact_for_task(input, task) {
                Some(artifact) if root_cause_artifact_is_accepted(artifact) => {}
                Some(artifact) if root_cause_artifact_has_state(artifact) => {
                    blockers.push(format!(
                        "task '{}' has no identified root cause",
                        task.title
                    ));
                }
                Some(_) => {
                    blockers.push(format!(
                        "task '{}' root cause artifact has no state",
                        task.title
                    ));
                }
                None => {
                    blockers.push(format!("task '{}' has no root cause artifact", task.title));
                }
            }
        }
        if task.kind == DevflowTaskKind::Implementation
            && !has_task_verification_artifact(input, task)
        {
            if has_task_passed_or_waived_gate(input, task) {
                blockers.push(format!(
                    "task '{}' has no verification artifact",
                    task.title
                ));
            } else {
                blockers.push(format!(
                    "task '{}' has no passed or waived verification gate",
                    task.title
                ));
            }
        }
        for required_gate in required_quality_gates(task) {
            if has_task_passed_or_waived_gate_with_artifact(input, task, required_gate.kind) {
                continue;
            }
            if has_task_passed_or_waived_gate_kind(input, task, required_gate.kind) {
                blockers.push(format!(
                    "task '{}' has no {} verification artifact",
                    task.title, required_gate.display_name
                ));
            } else {
                blockers.push(format!(
                    "task '{}' has no passed or waived {} gate",
                    task.title, required_gate.display_name
                ));
            }
        }
        if task.kind == DevflowTaskKind::Implementation
            && matches!(
                task.status,
                DevflowTaskStatus::ReadyForReview | DevflowTaskStatus::ReadyToMerge
            )
        {
            match review_artifact_for_task(input, task) {
                Some(artifact) if review_artifact_all_findings_addressed(artifact) => {}
                Some(artifact) if review_artifact_has_finding_state(artifact) => {
                    blockers.push(format!(
                        "task '{}' has unresolved review findings",
                        task.title
                    ));
                }
                Some(_) => {
                    blockers.push(format!(
                        "task '{}' review artifact has no finding state",
                        task.title
                    ));
                }
                None => {
                    blockers.push(format!("task '{}' has no review artifact", task.title));
                }
            }
        }
    }
    for run in &input.runs {
        match run.status {
            DevflowRunStatus::ReadyForReview
            | DevflowRunStatus::ReadyToMerge
            | DevflowRunStatus::Cancelled => {}
            DevflowRunStatus::Queued => {
                blockers.push(format!("run {} is still queued", run.id));
            }
            DevflowRunStatus::Running => {
                blockers.push(format!("run {} is still running", run.id));
            }
            DevflowRunStatus::Failed => {
                blockers.push(format!("run {} failed", run.id));
            }
        }
    }
    for gate in &input.quality_gates {
        match gate.status {
            DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Waived => {
                if let Some(artifact_id) = gate.artifact_id.as_deref() {
                    if quality_gate_verification_artifact(input, gate).is_none() {
                        blockers.push(format!(
                            "quality gate {} verification artifact {} is missing from artifact store",
                            gate.id, artifact_id
                        ));
                    }
                } else {
                    blockers.push(format!(
                        "quality gate {} has no verification artifact",
                        gate.id
                    ));
                }
            }
            DevflowQualityGateStatus::Queued => {
                blockers.push(format!("quality gate {} is still queued", gate.id));
            }
            DevflowQualityGateStatus::Running => {
                blockers.push(format!("quality gate {} is still running", gate.id));
            }
            DevflowQualityGateStatus::Failed => {
                blockers.push(format!("quality gate {} failed", gate.id));
            }
        }
    }
    if input.quality_gates.is_empty() {
        blockers.push("no quality gate evidence is recorded".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn render_commit_message(
    input: &DevflowReleasePrepInput,
    status: DevflowReleasePrepStatus,
    blockers: &[String],
) -> String {
    let mut lines = vec![
        format!("devflow: {}", input.anchor_task.title),
        String::new(),
        format!("Release prep status: {}", release_status_label(status)),
        String::new(),
        "Tasks:".to_string(),
    ];
    for task in &input.tasks {
        lines.push(format!("- {} ({:?})", task.title, task.status));
    }
    if !blockers.is_empty() {
        lines.push(String::new());
        lines.push("Finish branch blockers:".to_string());
        for blocker in blockers {
            lines.push(format!("- {blocker}"));
        }
    }
    lines.join("\n")
}

fn render_pr_body(
    input: &DevflowReleasePrepInput,
    status: DevflowReleasePrepStatus,
    blockers: &[String],
    git: &DevflowReleaseGitSnapshot,
) -> String {
    let mut body = String::new();
    body.push_str("## Summary\n");
    for task in &input.tasks {
        body.push_str(&format!("- {}: {}\n", task.title, task.objective));
    }
    body.push_str("\n## Validation\n");
    if input.quality_gates.is_empty() {
        body.push_str("- No quality gate evidence is recorded yet.\n");
    } else {
        for gate in &input.quality_gates {
            let artifact_summary =
                if let Some(artifact) = quality_gate_verification_artifact(input, gate) {
                    format!("artifact `{}`", artifact.path)
                } else if let Some(artifact_id) = gate.artifact_id.as_deref() {
                    format!("missing artifact `{artifact_id}`")
                } else {
                    "no verification artifact".to_string()
                };
            body.push_str(&format!(
                "- {:?}: {:?} via `{}` ({artifact_summary})\n",
                gate.kind, gate.status, gate.command
            ));
        }
    }
    body.push_str("\n## Root Cause\n");
    let mut wrote_root_cause_task = false;
    for task in input
        .tasks
        .iter()
        .filter(|task| task_requires_root_cause(task))
    {
        wrote_root_cause_task = true;
        match root_cause_artifact_for_task(input, task) {
            Some(artifact) if root_cause_artifact_is_accepted(artifact) => {
                body.push_str(&format!(
                    "- {}: identified via `{}`\n",
                    task.title, artifact.path
                ));
            }
            Some(artifact) if root_cause_artifact_has_state(artifact) => {
                body.push_str(&format!(
                    "- {}: root cause not identified in `{}`\n",
                    task.title, artifact.path
                ));
            }
            Some(artifact) => {
                body.push_str(&format!(
                    "- {}: root cause artifact has no state in `{}`\n",
                    task.title, artifact.path
                ));
            }
            None => {
                body.push_str(&format!("- {}: missing root cause artifact\n", task.title));
            }
        }
    }
    if !wrote_root_cause_task {
        body.push_str("- No root-cause-gated tasks.\n");
    }
    body.push_str("\n## Integrator\n");
    for task in input
        .tasks
        .iter()
        .filter(|task| task.kind == DevflowTaskKind::Implementation)
    {
        if let Some(artifact) = successful_integrator_merge_artifact(input, task) {
            body.push_str(&format!(
                "- {}: merged via `{}`\n",
                task.title, artifact.path
            ));
        } else if task.worktree_id.is_some() {
            body.push_str(&format!(
                "- {}: pending Integrator merge evidence\n",
                task.title
            ));
        } else {
            body.push_str(&format!("- {}: no managed worktree\n", task.title));
        }
    }
    body.push_str("\n## Artifacts\n");
    if input.artifacts.is_empty() {
        body.push_str("- No prior artifacts are recorded yet.\n");
    } else {
        for artifact in &input.artifacts {
            body.push_str(&format!("- {}: `{}`\n", artifact.title, artifact.path));
        }
    }
    body.push_str("\n## Persistence\n");
    if input.store_snapshot_load_error.is_none() && input.store_snapshot_persist_error.is_none() {
        body.push_str("- Store snapshot: healthy\n");
    } else {
        if let Some(error) = input.store_snapshot_load_error.as_ref() {
            body.push_str(&format!("- Store snapshot load error: {error}\n"));
        }
        if let Some(error) = input.store_snapshot_persist_error.as_ref() {
            body.push_str(&format!("- Store snapshot persist error: {error}\n"));
        }
    }
    body.push_str("\n## Finish Branch Gate\n");
    body.push_str(&format!("- Status: {}\n", release_status_label(status)));
    if blockers.is_empty() {
        body.push_str("- Blockers: none\n");
    } else {
        for blocker in blockers {
            body.push_str(&format!("- Blocker: {blocker}\n"));
        }
    }
    body.push_str("\n## Git Snapshot\n");
    body.push_str(&format!(
        "- Branch: {}\n",
        git.branch.as_deref().unwrap_or("unknown")
    ));
    body.push_str(&format!(
        "- Origin: {}\n",
        git.origin.as_deref().unwrap_or("unknown")
    ));
    body.push_str("- Status:\n\n```text\n");
    body.push_str(if git.status_short.trim().is_empty() {
        "clean\n"
    } else {
        &git.status_short
    });
    body.push_str("\n```\n");
    body.push_str("- Diff stat:\n\n```text\n");
    body.push_str(if git.diff_stat.trim().is_empty() {
        "no diff\n"
    } else {
        &git.diff_stat
    });
    body.push_str("\n```\n");
    body
}

fn render_release_notes(
    input: &DevflowReleasePrepInput,
    status: DevflowReleasePrepStatus,
) -> String {
    let mut notes = vec![
        "# Release Notes".to_string(),
        String::new(),
        format!("Status: {}", release_status_label(status)),
        String::new(),
        "Changes:".to_string(),
    ];
    for task in &input.tasks {
        notes.push(format!("- {}", task.title));
    }
    notes.join("\n")
}

fn has_successful_integrator_merge_artifact(
    input: &DevflowReleasePrepInput,
    task: &DevflowTask,
) -> bool {
    successful_integrator_merge_artifact(input, task).is_some()
}

fn successful_integrator_merge_artifact<'a>(
    input: &'a DevflowReleasePrepInput,
    task: &DevflowTask,
) -> Option<&'a DevflowArtifact> {
    input.artifacts.iter().find(|artifact| {
        artifact.task_id == task.id
            && artifact.title.starts_with("Integrator merge report")
            && artifact.summary.starts_with("Integrator merged ")
    })
}

fn review_artifact_for_task<'a>(
    input: &'a DevflowReleasePrepInput,
    task: &DevflowTask,
) -> Option<&'a DevflowArtifact> {
    input.artifacts.iter().find(|artifact| {
        artifact.task_id == task.id && artifact.kind == DevflowArtifactKind::ReviewReport
    })
}

fn root_cause_artifact_for_task<'a>(
    input: &'a DevflowReleasePrepInput,
    task: &DevflowTask,
) -> Option<&'a DevflowArtifact> {
    input.artifacts.iter().find(|artifact| {
        artifact.task_id == task.id
            && matches!(
                artifact.kind,
                DevflowArtifactKind::Report | DevflowArtifactKind::RunSummary
            )
            && (artifact.title.starts_with("Root cause") || root_cause_artifact_has_state(artifact))
    })
}

fn has_task_verification_artifact(input: &DevflowReleasePrepInput, task: &DevflowTask) -> bool {
    input.quality_gates.iter().any(|gate| {
        gate.task_id == task.id
            && matches!(
                gate.status,
                DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Waived
            )
            && quality_gate_verification_artifact(input, gate).is_some()
    })
}

fn has_task_passed_or_waived_gate(input: &DevflowReleasePrepInput, task: &DevflowTask) -> bool {
    input.quality_gates.iter().any(|gate| {
        gate.task_id == task.id
            && matches!(
                gate.status,
                DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Waived
            )
    })
}

fn has_task_passed_or_waived_gate_kind(
    input: &DevflowReleasePrepInput,
    task: &DevflowTask,
    kind: DevflowQualityGateKind,
) -> bool {
    input.quality_gates.iter().any(|gate| {
        gate.task_id == task.id
            && gate.kind == kind
            && matches!(
                gate.status,
                DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Waived
            )
    })
}

fn has_task_passed_or_waived_gate_with_artifact(
    input: &DevflowReleasePrepInput,
    task: &DevflowTask,
    kind: DevflowQualityGateKind,
) -> bool {
    input.quality_gates.iter().any(|gate| {
        gate.task_id == task.id
            && gate.kind == kind
            && matches!(
                gate.status,
                DevflowQualityGateStatus::Passed | DevflowQualityGateStatus::Waived
            )
            && quality_gate_verification_artifact(input, gate).is_some()
    })
}

fn quality_gate_verification_artifact<'a>(
    input: &'a DevflowReleasePrepInput,
    gate: &DevflowQualityGate,
) -> Option<&'a DevflowArtifact> {
    let artifact_id = gate.artifact_id.as_ref()?;
    input
        .artifacts
        .iter()
        .find(|artifact| artifact.id == *artifact_id)
}

async fn git_snapshot(root_path: &Path) -> DevflowReleaseGitSnapshot {
    let mut diagnostics = Vec::new();
    let branch = run_git_string(root_path, &["branch", "--show-current"], &mut diagnostics)
        .await
        .filter(|value| !value.is_empty());
    let origin = run_git_string(
        root_path,
        &["remote", "get-url", "origin"],
        &mut diagnostics,
    )
    .await
    .filter(|value| !value.is_empty());
    let status_short = run_git_string(root_path, &["status", "--short"], &mut diagnostics)
        .await
        .unwrap_or_default();
    let diff_stat = run_git_string(
        root_path,
        &["diff", "--stat", "--", ".", ":(exclude).codex"],
        &mut diagnostics,
    )
    .await
    .unwrap_or_default();
    let diff_check = run_git_string(
        root_path,
        &["diff", "--check", "--", ".", ":(exclude).codex"],
        &mut diagnostics,
    )
    .await
    .unwrap_or_default();
    let diff_check_ok = !diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("git diff --check"));
    DevflowReleaseGitSnapshot {
        branch,
        origin,
        status_short,
        diff_stat,
        diff_check,
        diff_check_ok,
        diagnostics,
    }
}

async fn run_git_string(
    root_path: &Path,
    args: &[&str],
    diagnostics: &mut Vec<String>,
) -> Option<String> {
    let mut command = Command::new("git");
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(root_path)
        .kill_on_drop(true);
    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(RELEASE_PREP_GIT_TIMEOUT_SECS),
        command.output(),
    )
    .await
    {
        Ok(Ok(output)) => output,
        Ok(Err(err)) => {
            diagnostics.push(format!("failed to spawn git {}: {err}", args.join(" ")));
            return None;
        }
        Err(_) => {
            diagnostics.push(format!(
                "git {} timed out after {RELEASE_PREP_GIT_TIMEOUT_SECS}s",
                args.join(" ")
            ));
            return None;
        }
    };
    if output.status.success() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    diagnostics.push(if stderr.is_empty() {
        format!(
            "git {} failed with status {}",
            args.join(" "),
            output.status
        )
    } else {
        format!("git {} failed: {stderr}", args.join(" "))
    });
    None
}

fn release_status_label(status: DevflowReleasePrepStatus) -> &'static str {
    match status {
        DevflowReleasePrepStatus::Ready => "ready",
        DevflowReleasePrepStatus::Blocked => "blocked",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::DevflowQualityGateKind;
    use codex_app_server_protocol::DevflowTaskRiskLevel;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    const NOW: i64 = 1_700_000_000;

    #[test]
    fn finish_branch_blocks_passed_gate_without_artifact_id() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let input = release_input(
            project_root.path().display().to_string(),
            vec![quality_gate(None)],
            Vec::new(),
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(blockers.contains(&"quality gate gate-1 has no verification artifact".to_string()));
        assert!(
            blockers.contains(
                &"task 'Verified implementation' has no verification artifact".to_string()
            )
        );
    }

    #[test]
    fn finish_branch_blocks_gate_with_missing_artifact_record() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let input = release_input(
            project_root.path().display().to_string(),
            vec![quality_gate(Some("missing-artifact"))],
            Vec::new(),
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(blockers.contains(
            &"quality gate gate-1 verification artifact missing-artifact is missing from artifact store"
                .to_string()
        ));
        assert!(
            blockers.contains(
                &"task 'Verified implementation' has no verification artifact".to_string()
            )
        );
    }

    #[test]
    fn finish_branch_accepts_gate_with_verification_artifact_record() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let artifact = verification_artifact(project_root.path().display().to_string());
        let review_artifact = review_artifact(
            project_root.path().display().to_string(),
            "Review finding state: status=clear; open=0; resolved=0; waived=0; followUp=0",
        );
        let input = release_input(
            project_root.path().display().to_string(),
            vec![quality_gate(Some(&artifact.id))],
            vec![artifact, review_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert_eq!(blockers, Vec::<String>::new());
    }

    #[test]
    fn finish_branch_blocks_high_risk_without_integration_test_gate() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let artifact = verification_artifact(project_root.path().display().to_string());
        let review_artifact = review_artifact(
            project_root.path().display().to_string(),
            "Review finding state: status=clear; open=0; resolved=0; waived=0; followUp=0",
        );
        let input = release_input_with_risk(
            project_root.path().display().to_string(),
            DevflowTaskRiskLevel::High,
            vec![quality_gate(Some(&artifact.id))],
            vec![artifact, review_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(
            blockers.contains(
                &"task 'Verified implementation' has no passed or waived integration test gate"
                    .to_string()
            )
        );
    }

    #[test]
    fn finish_branch_accepts_high_risk_with_integration_test_artifact() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let artifact = verification_artifact(project_root.path().display().to_string());
        let integration_artifact = quality_gate_artifact(
            project_root.path().display().to_string(),
            "integration-artifact",
            "run-1-integration-test.txt",
        );
        let review_artifact = review_artifact(
            project_root.path().display().to_string(),
            "Review finding state: status=clear; open=0; resolved=0; waived=0; followUp=0",
        );
        let input = release_input_with_risk(
            project_root.path().display().to_string(),
            DevflowTaskRiskLevel::High,
            vec![
                quality_gate(Some(&artifact.id)),
                quality_gate_with_id_kind(
                    "gate-integration",
                    DevflowQualityGateKind::IntegrationTest,
                    Some(&integration_artifact.id),
                ),
            ],
            vec![artifact, integration_artifact, review_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert_eq!(blockers, Vec::<String>::new());
    }

    #[test]
    fn finish_branch_blocks_snapshot_sensitive_task_without_snapshot_gate() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let artifact = verification_artifact(project_root.path().display().to_string());
        let review_artifact = review_artifact(
            project_root.path().display().to_string(),
            "Review finding state: status=clear; open=0; resolved=0; waived=0; followUp=0",
        );
        let mut input = release_input(
            project_root.path().display().to_string(),
            vec![quality_gate(Some(&artifact.id))],
            vec![artifact, review_artifact],
        );
        input.tasks[0].title = "Update provider settings UI".to_string();
        input.anchor_task = input.tasks[0].clone();

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(blockers.contains(
            &"task 'Update provider settings UI' has no passed or waived snapshot gate".to_string()
        ));
    }

    #[test]
    fn finish_branch_accepts_snapshot_sensitive_task_with_snapshot_artifact() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let artifact = verification_artifact(project_root.path().display().to_string());
        let snapshot_artifact = quality_gate_artifact(
            project_root.path().display().to_string(),
            "snapshot-artifact",
            "run-1-snapshot.txt",
        );
        let review_artifact = review_artifact(
            project_root.path().display().to_string(),
            "Review finding state: status=clear; open=0; resolved=0; waived=0; followUp=0",
        );
        let mut input = release_input(
            project_root.path().display().to_string(),
            vec![
                quality_gate(Some(&artifact.id)),
                quality_gate_with_id_kind(
                    "gate-snapshot",
                    DevflowQualityGateKind::Snapshot,
                    Some(&snapshot_artifact.id),
                ),
            ],
            vec![artifact, snapshot_artifact, review_artifact],
        );
        input.tasks[0].title = "Update provider settings UI".to_string();
        input.anchor_task = input.tasks[0].clone();

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert_eq!(blockers, Vec::<String>::new());
    }

    #[test]
    fn finish_branch_blocks_review_artifact_with_open_findings() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let artifact = verification_artifact(project_root.path().display().to_string());
        let review_artifact = review_artifact(
            project_root.path().display().to_string(),
            "Review finding state: status=open; open=1; resolved=0; waived=0; followUp=0",
        );
        let input = release_input(
            project_root.path().display().to_string(),
            vec![quality_gate(Some(&artifact.id))],
            vec![artifact, review_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(blockers.contains(
            &"task 'Verified implementation' has unresolved review findings".to_string()
        ));
    }

    #[test]
    fn finish_branch_blocks_diagnostic_without_root_cause_artifact() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let gate_artifact = diagnostic_gate_artifact(project_root.path().display().to_string());
        let input = diagnostic_release_input(
            project_root.path().display().to_string(),
            vec![quality_gate_for_task(
                "diagnostic-task",
                "diagnostic-run",
                Some(&gate_artifact.id),
            )],
            vec![gate_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(
            blockers.contains(&"task 'Diagnose login bug' has no root cause artifact".to_string())
        );
    }

    #[test]
    fn finish_branch_blocks_root_cause_artifact_without_state() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let gate_artifact = diagnostic_gate_artifact(project_root.path().display().to_string());
        let root_cause_artifact = root_cause_artifact(
            project_root.path().display().to_string(),
            "plain diagnostic summary",
        );
        let input = diagnostic_release_input(
            project_root.path().display().to_string(),
            vec![quality_gate_for_task(
                "diagnostic-task",
                "diagnostic-run",
                Some(&gate_artifact.id),
            )],
            vec![gate_artifact, root_cause_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(
            blockers.contains(
                &"task 'Diagnose login bug' root cause artifact has no state".to_string()
            )
        );
    }

    #[test]
    fn finish_branch_blocks_missing_identified_root_cause() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let gate_artifact = diagnostic_gate_artifact(project_root.path().display().to_string());
        let root_cause_artifact = root_cause_artifact(
            project_root.path().display().to_string(),
            "Root cause state: status=missing; rootCause=missing",
        );
        let input = diagnostic_release_input(
            project_root.path().display().to_string(),
            vec![quality_gate_for_task(
                "diagnostic-task",
                "diagnostic-run",
                Some(&gate_artifact.id),
            )],
            vec![gate_artifact, root_cause_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert!(
            blockers
                .contains(&"task 'Diagnose login bug' has no identified root cause".to_string())
        );
    }

    #[test]
    fn finish_branch_accepts_diagnostic_with_root_cause_artifact() {
        let project_root = TempDir::new().expect("temp dir");
        std::fs::create_dir(project_root.path().join(".git")).expect("git dir");
        let gate_artifact = diagnostic_gate_artifact(project_root.path().display().to_string());
        let root_cause_artifact = root_cause_artifact(
            project_root.path().display().to_string(),
            "Root cause state: status=identified; rootCause=base URL was doubled",
        );
        let input = diagnostic_release_input(
            project_root.path().display().to_string(),
            vec![quality_gate_for_task(
                "diagnostic-task",
                "diagnostic-run",
                Some(&gate_artifact.id),
            )],
            vec![gate_artifact, root_cause_artifact],
        );

        let blockers = finish_branch_blockers(&input, &clean_git_snapshot());

        assert_eq!(blockers, Vec::<String>::new());
    }

    fn release_input(
        project_root: String,
        quality_gates: Vec<DevflowQualityGate>,
        artifacts: Vec<DevflowArtifact>,
    ) -> DevflowReleasePrepInput {
        release_input_with_risk(
            project_root,
            DevflowTaskRiskLevel::Low,
            quality_gates,
            artifacts,
        )
    }

    fn release_input_with_risk(
        project_root: String,
        risk_level: DevflowTaskRiskLevel,
        quality_gates: Vec<DevflowQualityGate>,
        artifacts: Vec<DevflowArtifact>,
    ) -> DevflowReleasePrepInput {
        let task = DevflowTask {
            id: "task-1".to_string(),
            project_id: project_root.clone(),
            title: "Verified implementation".to_string(),
            objective: "Ship a verified implementation task.".to_string(),
            trigger_source: None,
            status: DevflowTaskStatus::ReadyToMerge,
            kind: DevflowTaskKind::Implementation,
            risk_level,
            dependencies: Vec::new(),
            assigned_agent_id: Some("codex-main".to_string()),
            worktree_id: None,
            context_pack_id: None,
            run_ids: vec!["run-1".to_string()],
            artifact_ids: artifacts
                .iter()
                .map(|artifact| artifact.id.clone())
                .collect(),
            created_at: NOW,
            updated_at: NOW,
        };
        DevflowReleasePrepInput {
            project: DevflowProject {
                id: "project-1".to_string(),
                name: "Project".to_string(),
                root_path: project_root,
                git_remote: None,
                default_branch: Some("main".to_string()),
                current_branch: Some("main".to_string()),
                is_trusted: true,
                test_commands: Vec::new(),
                detected_docs: Vec::new(),
                diagnostics: Vec::new(),
            },
            anchor_task: task.clone(),
            run_id: "run-1".to_string(),
            tasks: vec![task],
            runs: vec![DevflowRun {
                id: "run-1".to_string(),
                task_id: "task-1".to_string(),
                agent_id: "codex-main".to_string(),
                thread_id: None,
                turn_id: None,
                status: DevflowRunStatus::ReadyToMerge,
                started_at: NOW,
                completed_at: Some(NOW + 1),
                input: "Implement the task.".to_string(),
                stream_summary: Some("done".to_string()),
                command_ids: Vec::new(),
                artifact_ids: artifacts
                    .iter()
                    .map(|artifact| artifact.id.clone())
                    .collect(),
                exit_reason: None,
            }],
            quality_gates,
            artifacts,
            store_snapshot_load_error: None,
            store_snapshot_persist_error: None,
        }
    }

    fn diagnostic_release_input(
        project_root: String,
        quality_gates: Vec<DevflowQualityGate>,
        artifacts: Vec<DevflowArtifact>,
    ) -> DevflowReleasePrepInput {
        let task = DevflowTask {
            id: "diagnostic-task".to_string(),
            project_id: project_root.clone(),
            title: "Diagnose login bug".to_string(),
            objective: "Identify why login requests fail.".to_string(),
            trigger_source: Some("bug-report".to_string()),
            status: DevflowTaskStatus::ReadyForReview,
            kind: DevflowTaskKind::Diagnostic,
            risk_level: DevflowTaskRiskLevel::Low,
            dependencies: Vec::new(),
            assigned_agent_id: Some("codex-main".to_string()),
            worktree_id: None,
            context_pack_id: None,
            run_ids: vec!["diagnostic-run".to_string()],
            artifact_ids: artifacts
                .iter()
                .map(|artifact| artifact.id.clone())
                .collect(),
            created_at: NOW,
            updated_at: NOW,
        };
        DevflowReleasePrepInput {
            project: DevflowProject {
                id: "project-1".to_string(),
                name: "Project".to_string(),
                root_path: project_root,
                git_remote: None,
                default_branch: Some("main".to_string()),
                current_branch: Some("main".to_string()),
                is_trusted: true,
                test_commands: Vec::new(),
                detected_docs: Vec::new(),
                diagnostics: Vec::new(),
            },
            anchor_task: task.clone(),
            run_id: "diagnostic-run".to_string(),
            tasks: vec![task],
            runs: vec![DevflowRun {
                id: "diagnostic-run".to_string(),
                task_id: "diagnostic-task".to_string(),
                agent_id: "codex-main".to_string(),
                thread_id: None,
                turn_id: None,
                status: DevflowRunStatus::ReadyForReview,
                started_at: NOW,
                completed_at: Some(NOW + 1),
                input: "Diagnose the bug.".to_string(),
                stream_summary: Some("Root cause: base URL was doubled.".to_string()),
                command_ids: Vec::new(),
                artifact_ids: artifacts
                    .iter()
                    .map(|artifact| artifact.id.clone())
                    .collect(),
                exit_reason: None,
            }],
            quality_gates,
            artifacts,
            store_snapshot_load_error: None,
            store_snapshot_persist_error: None,
        }
    }

    fn quality_gate(artifact_id: Option<&str>) -> DevflowQualityGate {
        quality_gate_for_task("task-1", "run-1", artifact_id)
    }

    fn quality_gate_for_task(
        task_id: &str,
        run_id: &str,
        artifact_id: Option<&str>,
    ) -> DevflowQualityGate {
        quality_gate_for_task_kind(
            "gate-1",
            task_id,
            run_id,
            DevflowQualityGateKind::TargetedTest,
            artifact_id,
        )
    }

    fn quality_gate_with_id_kind(
        id: &str,
        kind: DevflowQualityGateKind,
        artifact_id: Option<&str>,
    ) -> DevflowQualityGate {
        quality_gate_for_task_kind(id, "task-1", "run-1", kind, artifact_id)
    }

    fn quality_gate_for_task_kind(
        id: &str,
        task_id: &str,
        run_id: &str,
        kind: DevflowQualityGateKind,
        artifact_id: Option<&str>,
    ) -> DevflowQualityGate {
        DevflowQualityGate {
            id: id.to_string(),
            task_id: task_id.to_string(),
            run_id: run_id.to_string(),
            kind,
            status: DevflowQualityGateStatus::Passed,
            command: "git diff --check".to_string(),
            cwd: "/tmp/project".to_string(),
            exit_code: Some(0),
            duration_ms: Some(10),
            summary: Some("passed".to_string()),
            artifact_id: artifact_id.map(str::to_string),
            waived_reason: None,
            created_at: NOW,
            updated_at: NOW,
        }
    }

    fn verification_artifact(project_root: String) -> DevflowArtifact {
        quality_gate_artifact(project_root, "artifact-1", "run-1-quality-gate.txt")
    }

    fn quality_gate_artifact(project_root: String, id: &str, file_name: &str) -> DevflowArtifact {
        DevflowArtifact {
            id: id.to_string(),
            task_id: "task-1".to_string(),
            run_id: "run-1".to_string(),
            kind: DevflowArtifactKind::QualityGateOutput,
            title: "Quality gate output for Verified implementation".to_string(),
            path: Path::new(&project_root)
                .join(".codex")
                .join("devflow")
                .join("artifacts")
                .join(file_name)
                .display()
                .to_string(),
            mime_type: "text/plain".to_string(),
            summary: "passed".to_string(),
            created_at: NOW,
        }
    }

    fn diagnostic_gate_artifact(project_root: String) -> DevflowArtifact {
        DevflowArtifact {
            id: "diagnostic-gate-artifact".to_string(),
            task_id: "diagnostic-task".to_string(),
            run_id: "diagnostic-run".to_string(),
            kind: DevflowArtifactKind::QualityGateOutput,
            title: "Quality gate output for Diagnose login bug".to_string(),
            path: Path::new(&project_root)
                .join(".codex")
                .join("devflow")
                .join("artifacts")
                .join("diagnostic-run-quality-gate.txt")
                .display()
                .to_string(),
            mime_type: "text/plain".to_string(),
            summary: "passed".to_string(),
            created_at: NOW,
        }
    }

    fn root_cause_artifact(project_root: String, summary: &str) -> DevflowArtifact {
        DevflowArtifact {
            id: "root-cause-artifact".to_string(),
            task_id: "diagnostic-task".to_string(),
            run_id: "diagnostic-run".to_string(),
            kind: DevflowArtifactKind::RunSummary,
            title: "Root cause summary for Diagnose login bug".to_string(),
            path: Path::new(&project_root)
                .join(".codex")
                .join("devflow")
                .join("artifacts")
                .join("diagnostic-run-summary.md")
                .display()
                .to_string(),
            mime_type: "text/markdown".to_string(),
            summary: summary.to_string(),
            created_at: NOW,
        }
    }

    fn review_artifact(project_root: String, summary: &str) -> DevflowArtifact {
        DevflowArtifact {
            id: "review-artifact-1".to_string(),
            task_id: "task-1".to_string(),
            run_id: "run-1".to_string(),
            kind: DevflowArtifactKind::ReviewReport,
            title: "Review report for Verified implementation".to_string(),
            path: Path::new(&project_root)
                .join(".codex")
                .join("devflow")
                .join("artifacts")
                .join("run-1-review.md")
                .display()
                .to_string(),
            mime_type: "text/markdown".to_string(),
            summary: summary.to_string(),
            created_at: NOW,
        }
    }

    fn clean_git_snapshot() -> DevflowReleaseGitSnapshot {
        DevflowReleaseGitSnapshot {
            branch: Some("main".to_string()),
            origin: None,
            status_short: String::new(),
            diff_stat: String::new(),
            diff_check: String::new(),
            diff_check_ok: true,
            diagnostics: Vec::new(),
        }
    }
}
