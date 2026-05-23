use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use chrono::Utc;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowReleaseSubmitMode;
use tokio::process::Command;

const RELEASE_PUBLISH_GIT_TIMEOUT_SECS: u64 = 30;
const RELEASE_PUBLISH_OUTPUT_SUMMARY_LIMIT: usize = 400;
const RELEASE_PUBLISH_REPORT_OUTPUT_LIMIT: usize = 4000;

pub(crate) struct DevflowReleasePublishResult {
    pub(crate) exit_code: Option<i32>,
    pub(crate) output_summary: String,
    pub(crate) report: String,
    pub(crate) succeeded: bool,
    pub(crate) published_at: i64,
}

struct GitStepOutput {
    label: &'static str,
    command: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

struct PublishReportInput<'a> {
    mode: DevflowReleaseSubmitMode,
    project_root: &'a Path,
    remote: Option<&'a str>,
    branch: Option<&'a str>,
    command: &'a str,
    commit_message_artifact: &'a DevflowArtifact,
    pr_body_artifact: &'a DevflowArtifact,
    release_note_artifact: &'a DevflowArtifact,
    steps: &'a [GitStepOutput],
    succeeded: bool,
    published_at: i64,
}

pub(crate) fn release_publish_command(mode: DevflowReleaseSubmitMode) -> &'static str {
    match mode {
        DevflowReleaseSubmitMode::CommitOnly => {
            "git add -A -- . ':(exclude).codex' && git commit -F <commit_message_artifact>"
        }
        DevflowReleaseSubmitMode::CommitAndPush => {
            "git add -A -- . ':(exclude).codex' && git commit -F <commit_message_artifact> && git push origin <current_branch>"
        }
    }
}

pub(crate) fn release_submit_mode_label(mode: DevflowReleaseSubmitMode) -> &'static str {
    match mode {
        DevflowReleaseSubmitMode::CommitOnly => "commit_only",
        DevflowReleaseSubmitMode::CommitAndPush => "commit_and_push",
    }
}

pub(crate) async fn run_release_publish(
    project_root: &Path,
    mode: DevflowReleaseSubmitMode,
    remote: Option<&str>,
    branch: Option<&str>,
    commit_message_artifact: &DevflowArtifact,
    pr_body_artifact: &DevflowArtifact,
    release_note_artifact: &DevflowArtifact,
) -> DevflowReleasePublishResult {
    let command = release_publish_command(mode).to_string();
    let published_at = Utc::now().timestamp();
    let mut steps = Vec::new();

    let stage_args = vec![
        "add".to_string(),
        "-A".to_string(),
        "--".to_string(),
        ".".to_string(),
        ":(exclude).codex".to_string(),
    ];
    let stage = run_git_step(project_root, "stage changes", &stage_args).await;
    let stage_ok = step_succeeded(&stage);
    steps.push(stage);

    let mut commit_ok = false;
    if stage_ok {
        let commit_args = vec![
            "commit".to_string(),
            "-F".to_string(),
            commit_message_artifact.path.clone(),
        ];
        let commit = run_git_step(project_root, "commit release", &commit_args).await;
        commit_ok = step_succeeded(&commit);
        steps.push(commit);
    }

    if mode == DevflowReleaseSubmitMode::CommitAndPush && commit_ok {
        let branch = branch.unwrap_or("<current-branch>");
        let push_args = vec!["push".to_string(), "origin".to_string(), branch.to_string()];
        steps.push(run_git_step(project_root, "push release", &push_args).await);
    }

    let succeeded = steps.iter().all(step_succeeded);
    let exit_code = if succeeded {
        Some(0)
    } else {
        steps
            .iter()
            .find(|step| !step_succeeded(step))
            .and_then(|step| step.exit_code)
    };
    let output_summary = truncate(
        &summarize_steps(&steps),
        RELEASE_PUBLISH_OUTPUT_SUMMARY_LIMIT,
    );
    let report = render_publish_report(PublishReportInput {
        mode,
        project_root,
        remote,
        branch,
        command: &command,
        commit_message_artifact,
        pr_body_artifact,
        release_note_artifact,
        steps: &steps,
        succeeded,
        published_at,
    });

    DevflowReleasePublishResult {
        exit_code,
        output_summary,
        report,
        succeeded,
        published_at,
    }
}

async fn run_git_step(project_root: &Path, label: &'static str, args: &[String]) -> GitStepOutput {
    let command_label = format!("git {}", args.join(" "));
    let mut command = Command::new("git");
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(project_root)
        .stdin(Stdio::null())
        .kill_on_drop(true);

    match tokio::time::timeout(
        Duration::from_secs(RELEASE_PUBLISH_GIT_TIMEOUT_SECS),
        command.output(),
    )
    .await
    {
        Ok(Ok(output)) => GitStepOutput {
            label,
            command: command_label,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Ok(Err(err)) => GitStepOutput {
            label,
            command: command_label,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("failed to spawn git command: {err}"),
        },
        Err(_) => GitStepOutput {
            label,
            command: command_label,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("git command timed out after {RELEASE_PUBLISH_GIT_TIMEOUT_SECS}s"),
        },
    }
}

fn render_publish_report(input: PublishReportInput<'_>) -> String {
    let status = if input.succeeded {
        "submitted"
    } else {
        "failed"
    };
    let branch = input.branch.unwrap_or("none");
    let remote = input.remote.unwrap_or("none");
    let mut report = format!(
        "# Release publish report\n\n- Status: {status}\n- Mode: {}\n- Command template: `{}`\n- Project root: {}\n- Remote: {remote}\n- Branch: {branch}\n- Published at: {}\n- Commit message artifact: `{}`\n- PR body artifact: `{}`\n- Release note artifact: `{}`\n\n## Commands\n",
        release_submit_mode_label(input.mode),
        input.command,
        input.project_root.display(),
        input.published_at,
        input.commit_message_artifact.path,
        input.pr_body_artifact.path,
        input.release_note_artifact.path
    );
    for step in input.steps {
        report.push_str(&format!(
            "\n### {}\n\n- Command: `{}`\n- Exit code: {}\n\n",
            step.label,
            step.command,
            step.exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
        report.push_str("#### stdout\n\n```text\n");
        report.push_str(&truncate(
            step.stdout.trim(),
            RELEASE_PUBLISH_REPORT_OUTPUT_LIMIT,
        ));
        report.push_str("\n```\n\n#### stderr\n\n```text\n");
        report.push_str(&truncate(
            step.stderr.trim(),
            RELEASE_PUBLISH_REPORT_OUTPUT_LIMIT,
        ));
        report.push_str("\n```\n");
    }
    report
}

fn summarize_steps(steps: &[GitStepOutput]) -> String {
    steps
        .iter()
        .map(|step| {
            let output = match (step.stdout.trim(), step.stderr.trim()) {
                ("", "") => "no output".to_string(),
                (stdout, "") => stdout.to_string(),
                ("", stderr) => stderr.to_string(),
                (stdout, stderr) => format!("{stdout}\n\n[stderr]\n{stderr}"),
            };
            let exit_code = step
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "none".to_string());
            format!("{} exited {exit_code}: {output}", step.label)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn step_succeeded(step: &GitStepOutput) -> bool {
    step.exit_code == Some(0)
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let prefix = value.chars().take(limit).collect::<String>();
    format!("{prefix}...")
}
