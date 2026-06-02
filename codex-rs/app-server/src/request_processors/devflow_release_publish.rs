use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use chrono::Utc;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowReleaseSubmitMode;
use serde::Deserialize;
use tokio::process::Command;

const RELEASE_PUBLISH_COMMAND_TIMEOUT_SECS: u64 = 30;
const RELEASE_PUBLISH_OUTPUT_SUMMARY_LIMIT: usize = 400;
const RELEASE_PUBLISH_REPORT_OUTPUT_LIMIT: usize = 4000;

pub(crate) struct DevflowReleasePublishResult {
    pub(crate) exit_code: Option<i32>,
    pub(crate) output_summary: String,
    pub(crate) report: String,
    pub(crate) succeeded: bool,
    pub(crate) published_at: i64,
    pub(crate) pull_request_url: Option<String>,
}

struct PublishStepOutput {
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
    steps: &'a [PublishStepOutput],
    succeeded: bool,
    published_at: i64,
    pull_request_url: Option<&'a str>,
}

struct ReleasePublishExecution<'a> {
    project_root: &'a Path,
    mode: DevflowReleaseSubmitMode,
    remote: Option<&'a str>,
    branch: Option<&'a str>,
    commit_message_artifact: &'a DevflowArtifact,
    pr_body_artifact: &'a DevflowArtifact,
    release_note_artifact: &'a DevflowArtifact,
}

struct ReleasePublishPrograms {
    git: String,
    gh: String,
}

#[derive(Deserialize)]
struct GhPullRequestView {
    url: String,
    state: String,
}

impl Default for ReleasePublishPrograms {
    fn default() -> Self {
        Self {
            git: "git".to_string(),
            gh: "gh".to_string(),
        }
    }
}

pub(crate) fn release_publish_command(mode: DevflowReleaseSubmitMode) -> &'static str {
    match mode {
        DevflowReleaseSubmitMode::CommitOnly => {
            "git add -A -- . ':(exclude).codex' && git commit -F <commit_message_artifact>"
        }
        DevflowReleaseSubmitMode::CommitAndPush => {
            "git add -A -- . ':(exclude).codex' && git commit -F <commit_message_artifact> && git push origin <current_branch> && (gh pr view --json number,url,state || gh pr create --fill --body-file <pr_body_artifact>)"
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
    let input = ReleasePublishExecution {
        project_root,
        mode,
        remote,
        branch,
        commit_message_artifact,
        pr_body_artifact,
        release_note_artifact,
    };
    let programs = ReleasePublishPrograms::default();
    run_release_publish_with_programs(input, &programs).await
}

async fn run_release_publish_with_programs(
    input: ReleasePublishExecution<'_>,
    programs: &ReleasePublishPrograms,
) -> DevflowReleasePublishResult {
    let command = release_publish_command(input.mode).to_string();
    let published_at = Utc::now().timestamp();
    let mut steps = Vec::new();
    let mut pull_request_url = None;

    let stage_args = vec![
        "add".to_string(),
        "-A".to_string(),
        "--".to_string(),
        ".".to_string(),
        ":(exclude).codex".to_string(),
    ];
    let stage = run_git_step(input.project_root, "stage changes", &stage_args, programs).await;
    let stage_ok = step_succeeded(&stage);
    steps.push(stage);

    let mut commit_ok = false;
    if stage_ok {
        let commit_args = vec![
            "commit".to_string(),
            "-F".to_string(),
            input.commit_message_artifact.path.clone(),
        ];
        let commit =
            run_git_step(input.project_root, "commit release", &commit_args, programs).await;
        commit_ok = step_succeeded(&commit);
        steps.push(commit);
    }

    if input.mode == DevflowReleaseSubmitMode::CommitAndPush && commit_ok {
        let branch = input.branch.unwrap_or("<current-branch>");
        let push_args = vec!["push".to_string(), "origin".to_string(), branch.to_string()];
        let push = run_git_step(input.project_root, "push release", &push_args, programs).await;
        let push_ok = step_succeeded(&push);
        steps.push(push);
        if push_ok {
            let (pr_steps, pr_url) =
                run_pull_request_publish(input.project_root, input.pr_body_artifact, programs)
                    .await;
            steps.extend(pr_steps);
            pull_request_url = pr_url;
        }
    }

    let pull_request_ready =
        input.mode == DevflowReleaseSubmitMode::CommitOnly || pull_request_url.is_some();
    let succeeded = steps.iter().all(step_succeeded) && pull_request_ready;
    let exit_code = if succeeded {
        Some(0)
    } else {
        steps
            .iter()
            .find(|step| !step_succeeded(step))
            .and_then(|step| step.exit_code)
    };
    let output_summary = truncate(
        &summarize_publish_result(&steps, input.mode, pull_request_url.as_deref()),
        RELEASE_PUBLISH_OUTPUT_SUMMARY_LIMIT,
    );
    let report = render_publish_report(PublishReportInput {
        mode: input.mode,
        project_root: input.project_root,
        remote: input.remote,
        branch: input.branch,
        command: &command,
        commit_message_artifact: input.commit_message_artifact,
        pr_body_artifact: input.pr_body_artifact,
        release_note_artifact: input.release_note_artifact,
        steps: &steps,
        succeeded,
        published_at,
        pull_request_url: pull_request_url.as_deref(),
    });

    DevflowReleasePublishResult {
        exit_code,
        output_summary,
        report,
        succeeded,
        published_at,
        pull_request_url,
    }
}

async fn run_git_step(
    project_root: &Path,
    label: &'static str,
    args: &[String],
    programs: &ReleasePublishPrograms,
) -> PublishStepOutput {
    let command_label = format!("git {}", args.join(" "));
    let mut command = Command::new(&programs.git);
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(project_root)
        .stdin(Stdio::null())
        .kill_on_drop(true);

    run_process_step(label, command_label, command).await
}

async fn run_gh_step(
    project_root: &Path,
    label: &'static str,
    args: &[String],
    programs: &ReleasePublishPrograms,
) -> PublishStepOutput {
    let command_label = format!("gh {}", args.join(" "));
    let mut command = Command::new(&programs.gh);
    command
        .args(args)
        .current_dir(project_root)
        .stdin(Stdio::null())
        .kill_on_drop(true);

    run_process_step(label, command_label, command).await
}

async fn run_process_step(
    label: &'static str,
    command_label: String,
    mut command: Command,
) -> PublishStepOutput {
    match tokio::time::timeout(
        Duration::from_secs(RELEASE_PUBLISH_COMMAND_TIMEOUT_SECS),
        command.output(),
    )
    .await
    {
        Ok(Ok(output)) => PublishStepOutput {
            label,
            command: command_label,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Ok(Err(err)) => PublishStepOutput {
            label,
            command: command_label,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("failed to spawn command: {err}"),
        },
        Err(_) => PublishStepOutput {
            label,
            command: command_label,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("command timed out after {RELEASE_PUBLISH_COMMAND_TIMEOUT_SECS}s"),
        },
    }
}

async fn run_pull_request_publish(
    project_root: &Path,
    pr_body_artifact: &DevflowArtifact,
    programs: &ReleasePublishPrograms,
) -> (Vec<PublishStepOutput>, Option<String>) {
    let view_args = vec![
        "pr".to_string(),
        "view".to_string(),
        "--json".to_string(),
        "number,url,state".to_string(),
    ];
    let view = run_gh_step(project_root, "read pull request", &view_args, programs).await;
    if step_succeeded(&view)
        && let Some(url) = pull_request_url_from_view_output(&view.stdout)
    {
        return (vec![view], Some(url));
    }

    let create_args = vec![
        "pr".to_string(),
        "create".to_string(),
        "--fill".to_string(),
        "--body-file".to_string(),
        pr_body_artifact.path.clone(),
    ];
    let create = run_gh_step(project_root, "create pull request", &create_args, programs).await;
    let pull_request_url =
        step_succeeded(&create).then(|| pull_request_url_from_create_output(&create.stdout));
    (vec![create], pull_request_url.flatten())
}

fn pull_request_url_from_view_output(stdout: &str) -> Option<String> {
    let pull_request = serde_json::from_str::<GhPullRequestView>(stdout).ok()?;
    pull_request
        .state
        .eq_ignore_ascii_case("open")
        .then_some(pull_request.url)
        .filter(|url| !url.trim().is_empty())
}

fn pull_request_url_from_create_output(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("https://") || line.starts_with("http://"))
        .map(str::to_string)
}

fn render_publish_report(input: PublishReportInput<'_>) -> String {
    let status = if input.succeeded {
        "submitted"
    } else {
        "failed"
    };
    let branch = input.branch.unwrap_or("none");
    let remote = input.remote.unwrap_or("none");
    let pull_request = input.pull_request_url.unwrap_or("none");
    let mut report = format!(
        "# Release publish report\n\n- Status: {status}\n- Mode: {}\n- Command template: `{}`\n- Project root: {}\n- Remote: {remote}\n- Branch: {branch}\n- Pull request: {pull_request}\n- Published at: {}\n- Commit message artifact: `{}`\n- PR body artifact: `{}`\n- Release note artifact: `{}`\n\n## Commands\n",
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

fn summarize_publish_result(
    steps: &[PublishStepOutput],
    mode: DevflowReleaseSubmitMode,
    pull_request_url: Option<&str>,
) -> String {
    let mut summary = summarize_steps(steps);
    if mode == DevflowReleaseSubmitMode::CommitAndPush {
        let pull_request = pull_request_url.unwrap_or("missing");
        summary.push_str(&format!("\npull request: {pull_request}"));
    }
    summary
}

fn summarize_steps(steps: &[PublishStepOutput]) -> String {
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

fn step_succeeded(step: &PublishStepOutput) -> bool {
    step.exit_code == Some(0)
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let prefix = value.chars().take(limit).collect::<String>();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::DevflowReleasePublishResult;
    use super::ReleasePublishExecution;
    use super::ReleasePublishPrograms;
    use super::run_release_publish_with_programs;
    use codex_app_server_protocol::DevflowArtifact;
    use codex_app_server_protocol::DevflowArtifactKind;
    use codex_app_server_protocol::DevflowReleaseSubmitMode;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    #[tokio::test]
    async fn commit_and_push_creates_pull_request_after_push() -> Result<()> {
        let repo = TempDir::new()?;
        init_git_repo(repo.path())?;
        let bare_remote = TempDir::new()?;
        run_git(
            None,
            &[
                "init",
                "--bare",
                bare_remote.path().to_str().expect("utf-8 path"),
            ],
        )?;
        run_git(
            Some(repo.path()),
            &[
                "remote",
                "add",
                "origin",
                bare_remote.path().to_str().expect("utf-8 path"),
            ],
        )?;

        write_file(repo.path().join("note.txt"), "before\n")?;
        run_git(Some(repo.path()), &["add", "note.txt"])?;
        run_git(Some(repo.path()), &["commit", "-m", "init"])?;
        run_git(Some(repo.path()), &["checkout", "-b", "feature"])?;
        write_file(repo.path().join("note.txt"), "after\n")?;

        let artifacts_dir = repo.path().join(".codex/devflow/artifacts");
        fs::create_dir_all(&artifacts_dir)?;
        let commit_message_artifact = write_artifact(
            &artifacts_dir,
            "commit-message",
            "Commit message",
            "devflow: publish feature\n\nBody\n",
            "commit-message",
        )?;
        let pr_body_artifact = write_artifact(
            &artifacts_dir,
            "pr-body",
            "PR body",
            "# PR body\n",
            "pr-body",
        )?;
        let release_note_artifact = write_artifact(
            &artifacts_dir,
            "release-notes",
            "Release notes",
            "# Notes\n",
            "release-notes",
        )?;

        let gh_log = repo.path().join("gh.log");
        let gh = repo.path().join("gh");
        fs::write(
            &gh,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"$1\" = \"pr\" ] && [ \"$2\" = \"view\" ]; then\n  exit 1\nfi\nif [ \"$1\" = \"pr\" ] && [ \"$2\" = \"create\" ]; then\n  echo 'https://example.test/yuqei/codex/pull/42'\n  exit 0\nfi\nexit 64\n",
                gh_log.display()
            ),
        )?;
        let mut permissions = fs::metadata(&gh)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&gh, permissions)?;

        let programs = ReleasePublishPrograms {
            git: "git".to_string(),
            gh: gh.display().to_string(),
        };
        let result: DevflowReleasePublishResult = run_release_publish_with_programs(
            ReleasePublishExecution {
                project_root: repo.path(),
                mode: DevflowReleaseSubmitMode::CommitAndPush,
                remote: Some("https://example.test/yuqei/codex.git"),
                branch: Some("feature"),
                commit_message_artifact: &commit_message_artifact,
                pr_body_artifact: &pr_body_artifact,
                release_note_artifact: &release_note_artifact,
            },
            &programs,
        )
        .await;

        assert!(result.succeeded, "{}", result.report);
        assert_eq!(
            result.pull_request_url.as_deref(),
            Some("https://example.test/yuqei/codex/pull/42")
        );
        assert!(
            result
                .report
                .contains("Pull request: https://example.test/yuqei/codex/pull/42")
        );
        assert!(result.report.contains("gh pr create --fill --body-file"));
        let gh_log = fs::read_to_string(&gh_log)?;
        assert!(gh_log.contains("pr view --json number,url,state"));
        assert!(gh_log.contains("pr create --fill --body-file"));
        assert!(gh_log.contains(pr_body_artifact.path.as_str()));
        Ok(())
    }

    fn init_git_repo(path: &Path) -> Result<()> {
        run_git(
            None,
            &["init", "-b", "main", path.to_str().expect("utf-8 path")],
        )?;
        run_git(
            Some(path),
            &["config", "user.email", "devflow@example.test"],
        )?;
        run_git(Some(path), &["config", "user.name", "Devflow"])?;
        Ok(())
    }

    fn write_artifact(
        artifacts_dir: &Path,
        suffix: &str,
        title: &str,
        contents: &str,
        name: &str,
    ) -> Result<DevflowArtifact> {
        let artifact_path = artifacts_dir.join(format!("{name}.md"));
        fs::write(&artifact_path, contents)?;
        Ok(DevflowArtifact {
            id: format!("artifact-{suffix}"),
            task_id: "task-1".to_string(),
            run_id: "run-1".to_string(),
            kind: DevflowArtifactKind::Report,
            title: title.to_string(),
            path: artifact_path.display().to_string(),
            mime_type: "text/markdown".to_string(),
            summary: contents.trim().to_string(),
            created_at: 1,
        })
    }

    fn write_file(path: impl AsRef<Path>, contents: &str) -> Result<()> {
        fs::write(path, contents)?;
        Ok(())
    }

    fn run_git(cwd: Option<&Path>, args: &[&str]) -> Result<()> {
        let mut command = Command::new("git");
        command.args(args).stdin(std::process::Stdio::null());
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let output = command.output()?;
        assert!(
            output.status.success(),
            "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }
}
