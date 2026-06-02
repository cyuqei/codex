//! Utility to compute the current Git status summary for the working directory.
//!
//! Returns the short porcelain status plus branch header, matching the kind of
//! context a commit-drafting workflow needs before inspecting a full diff.

use std::path::Path;
use std::time::Duration;

use crate::workspace_command::WorkspaceCommand;
use crate::workspace_command::WorkspaceCommandExecutor;
use crate::workspace_command::WorkspaceCommandOutput;

const STATUS_COMMAND_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 30);

/// Return value of [`get_git_status`].
///
/// * `bool` – Whether the current working directory is inside a Git repo.
/// * `String` – The git status summary (may be empty).
pub(crate) async fn get_git_status(
    runner: &dyn WorkspaceCommandExecutor,
    cwd: &Path,
) -> Result<(bool, String), String> {
    if !inside_git_repo(runner, cwd).await? {
        return Ok((false, String::new()));
    }

    let output = run_git_command(
        runner,
        cwd,
        &["status", "--short", "--branch", "--untracked-files=all"],
    )
    .await?;
    if output.success() {
        Ok((true, output.stdout))
    } else {
        Err(format!(
            "git status failed with status {}",
            output.exit_code
        ))
    }
}

async fn inside_git_repo(
    runner: &dyn WorkspaceCommandExecutor,
    cwd: &Path,
) -> Result<bool, String> {
    let output = run_git_command(runner, cwd, &["rev-parse", "--is-inside-work-tree"]).await?;
    Ok(output.success())
}

async fn run_git_command(
    runner: &dyn WorkspaceCommandExecutor,
    cwd: &Path,
    args: &[&str],
) -> Result<WorkspaceCommandOutput, String> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("git".to_string());
    argv.extend(args.iter().map(|arg| (*arg).to_string()));
    runner
        .run(
            WorkspaceCommand::new(argv)
                .cwd(cwd.to_path_buf())
                .timeout(STATUS_COMMAND_TIMEOUT)
                .disable_output_cap(),
        )
        .await
        .map_err(|err| err.to_string())
}
