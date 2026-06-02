use std::path::Path;
use std::process::Stdio;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

pub(crate) const DEVFLOW_CLAUDE_CLI_ENV: &str = "CODEX_DEVFLOW_CLAUDE_CLI";
pub(crate) const DEVFLOW_HERMES_CLI_ENV: &str = "CODEX_DEVFLOW_HERMES_CLI";
const DEFAULT_CLAUDE_CLI: &str = "claude";
const DEFAULT_HERMES_CLI: &str = "hermes";

#[derive(Clone, Debug)]
pub(crate) struct ExternalAgentExecution {
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) async fn run_claude_report(
    cwd: &Path,
    prompt: &str,
) -> Result<ExternalAgentExecution, String> {
    let executable = std::env::var(DEVFLOW_CLAUDE_CLI_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CLAUDE_CLI.to_string());
    run_external_command(
        &executable,
        cwd,
        &[
            "-p",
            "--output-format",
            "text",
            "--permission-mode",
            "plan",
            "--tools",
            "",
            prompt,
        ],
        "Claude Code adapter",
    )
    .await
}

pub(crate) async fn run_hermes_command(
    cwd: &Path,
    args: &[&str],
) -> Result<ExternalAgentExecution, String> {
    let executable = std::env::var(DEVFLOW_HERMES_CLI_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HERMES_CLI.to_string());
    run_external_command(&executable, cwd, args, "Hermes adapter").await
}

async fn run_external_command(
    executable: &str,
    cwd: &Path,
    args: &[&str],
    label: &str,
) -> Result<ExternalAgentExecution, String> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = command
        .spawn()
        .map_err(|err| format!("failed to spawn {label}: {err}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("missing {label} stdout pipe"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("missing {label} stderr pipe"))?;

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await.map(|_| buf)
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await.map(|_| buf)
    });

    let status = child
        .wait()
        .await
        .map_err(|err| format!("failed waiting for {label}: {err}"))?;
    let stdout = stdout_task
        .await
        .map_err(|err| format!("failed joining {label} stdout task: {err}"))?
        .map_err(|err| format!("failed reading {label} stdout: {err}"))?;
    let stderr = stderr_task
        .await
        .map_err(|err| format!("failed joining {label} stderr task: {err}"))?
        .map_err(|err| format!("failed reading {label} stderr: {err}"))?;

    Ok(ExternalAgentExecution {
        exit_code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}
