use std::path::Path;
use std::path::PathBuf;

use codex_app_server_protocol::DevflowProject;
use codex_core::config::ConfigOverrides;
use codex_git_utils::get_git_repo_root;
use codex_protocol::protocol::AskForApproval;
use tokio::fs;
use tokio::process::Command;

use crate::config_manager::ConfigManager;
use crate::error_code::internal_error;
use crate::error_code::invalid_request;

const GIT_PROJECT_TIMEOUT_SECS: u64 = 15;

pub(crate) async fn diagnose_project(
    config_manager: &ConfigManager,
    project_root: &str,
) -> Result<DevflowProject, codex_app_server_protocol::JSONRPCErrorError> {
    if project_root.trim().is_empty() {
        return Err(invalid_request("project_root is required".to_string()));
    }

    let project_path = PathBuf::from(project_root);
    let root_path = get_git_repo_root(&project_path).unwrap_or(project_path.clone());
    let name = root_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(project_root)
        .to_string();

    let config = config_manager
        .load_with_overrides(
            None::<std::collections::HashMap<String, serde_json::Value>>,
            ConfigOverrides {
                cwd: Some(project_path.clone()),
                approval_policy: Some(AskForApproval::Never),
                ..Default::default()
            },
        )
        .await
        .map_err(|err| internal_error(format!("failed to load project config: {err}")))?;

    let (git_remote, default_branch, current_branch) = if root_path.join(".git").exists() {
        (
            run_git_string(&root_path, &["remote", "get-url", "origin"])
                .await
                .ok(),
            git_default_branch(&root_path).await,
            run_git_string(&root_path, &["branch", "--show-current"])
                .await
                .ok(),
        )
    } else {
        (None, None, None)
    };

    let detected_docs = detect_project_docs(&project_path).await;
    let test_commands = detect_test_commands(&project_path).await;
    let diagnostics = build_project_diagnostics(
        root_path.as_path(),
        git_remote.as_ref(),
        default_branch.as_ref(),
        current_branch.as_ref(),
        config.active_project.is_trusted(),
        &detected_docs,
        &test_commands,
    );

    Ok(DevflowProject {
        id: root_path.display().to_string(),
        name,
        root_path: root_path.display().to_string(),
        git_remote,
        default_branch,
        current_branch,
        is_trusted: config.active_project.is_trusted(),
        test_commands,
        detected_docs,
        diagnostics,
    })
}

async fn detect_project_docs(project_path: &Path) -> Vec<String> {
    let candidates = [
        project_path.join("AGENTS.md"),
        project_path.join("README.md"),
        project_path.join("docs"),
    ];
    let mut docs = Vec::new();
    for path in candidates {
        if fs::metadata(&path).await.is_ok() {
            docs.push(path.display().to_string());
        }
    }
    docs
}

async fn detect_test_commands(project_path: &Path) -> Vec<String> {
    let mut commands = Vec::new();
    if fs::metadata(project_path.join("Cargo.toml")).await.is_ok() {
        commands.push("cargo test".to_string());
    }
    if fs::metadata(project_path.join("package.json"))
        .await
        .is_ok()
    {
        commands.push("npm test".to_string());
    }
    if fs::metadata(project_path.join("pytest.ini")).await.is_ok()
        || fs::metadata(project_path.join("pyproject.toml"))
            .await
            .is_ok()
    {
        commands.push("pytest".to_string());
    }
    if fs::metadata(project_path.join("justfile")).await.is_ok() {
        commands.push("just test".to_string());
    }
    commands
}

fn build_project_diagnostics(
    root_path: &Path,
    git_remote: Option<&String>,
    default_branch: Option<&String>,
    current_branch: Option<&String>,
    is_trusted: bool,
    detected_docs: &[String],
    test_commands: &[String],
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if root_path.join(".git").exists() {
        diagnostics.push(format!(
            "git repository detected at {}",
            root_path.display()
        ));
    } else {
        diagnostics.push("project is not inside a git repository".to_string());
    }
    diagnostics.push(if is_trusted {
        "project is trusted".to_string()
    } else {
        "project is not explicitly trusted".to_string()
    });
    if let Some(git_remote) = git_remote {
        diagnostics.push(format!("origin remote: {git_remote}"));
    }
    if let Some(default_branch) = default_branch {
        diagnostics.push(format!("default branch: {default_branch}"));
    }
    if let Some(current_branch) = current_branch {
        diagnostics.push(format!("current branch: {current_branch}"));
    }
    if detected_docs.is_empty() {
        diagnostics.push("no project docs detected".to_string());
    } else {
        diagnostics.push(format!("detected {} project docs", detected_docs.len()));
    }
    if test_commands.is_empty() {
        diagnostics.push("no obvious test commands detected".to_string());
    } else {
        diagnostics.push(format!("detected {} test commands", test_commands.len()));
    }
    diagnostics
}

async fn git_default_branch(root_path: &Path) -> Option<String> {
    let origin_head = run_git_string(
        root_path,
        &["symbolic-ref", "refs/remotes/origin/HEAD", "--short"],
    )
    .await
    .ok()?;
    origin_head
        .split_once('/')
        .map(|(_, branch)| branch.to_string())
        .or(Some(origin_head))
}

async fn run_git_string(root_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = run_git(root_path, args).await?;
    String::from_utf8(output.stdout)
        .map(|stdout| stdout.trim().to_string())
        .map_err(|err| format!("git output was not utf-8: {err}"))
}

async fn run_git(root_path: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let mut command = Command::new("git");
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(root_path)
        .kill_on_drop(true);
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(GIT_PROJECT_TIMEOUT_SECS),
        command.output(),
    )
    .await
    .map_err(|_| format!("git command timed out after {GIT_PROJECT_TIMEOUT_SECS}s"))?
    .map_err(|err| format!("failed to spawn git command: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git command failed with status {}", output.status)
        } else {
            stderr
        });
    }
    Ok(output)
}
