use std::path::Path;
use std::path::PathBuf;

use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowWorktree;
use codex_app_server_protocol::DevflowWorktreeStatus;
use codex_git_utils::current_branch_name;
use codex_git_utils::get_git_repo_root;
use codex_git_utils::get_head_commit_hash;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;
use tokio::process::Command;

const GIT_WORKTREE_TIMEOUT_SECS: u64 = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PersistedDevflowWorktree {
    pub(crate) worktree: DevflowWorktree,
}

#[derive(Debug, Clone)]
pub(crate) struct WorktreeMergeOutcome {
    pub(crate) worktree: DevflowWorktree,
    pub(crate) merged: bool,
    pub(crate) conflicts: Vec<String>,
    pub(crate) diff: String,
}

pub(crate) async fn create_managed_worktree(
    codex_home: &Path,
    task: &DevflowTask,
) -> Result<DevflowWorktree, String> {
    let existing_path = metadata_file_path(codex_home, &task.id);
    if let Some(existing) = load_persisted_worktree(&existing_path).await? {
        return refresh_worktree_status(existing.worktree).await;
    }

    let repo_root = get_git_repo_root(Path::new(&task.project_id)).ok_or_else(|| {
        format!(
            "project is not inside a git repository: {}",
            task.project_id
        )
    })?;
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("repo root has no valid name: {}", repo_root.display()))?;
    let root_path = codex_home
        .join("devflow")
        .join("worktrees")
        .join(repo_name)
        .join(short_task_slug(&task.id));
    let cwd_path = worktree_cwd_path(&repo_root, Path::new(&task.project_id), &root_path)?;
    let branch = format!("codex/devflow/{}", short_task_slug(&task.id));
    let base_branch = current_branch_name(&repo_root).await;
    let base_commit = get_head_commit_hash(&repo_root)
        .await
        .map(|sha| sha.0)
        .ok_or_else(|| format!("failed to resolve HEAD for {}", repo_root.display()))?;

    if let Some(parent) = root_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create worktree parent: {err}"))?;
    }

    run_git_command(
        &repo_root,
        &[
            "worktree",
            "add",
            "-b",
            branch.as_str(),
            root_path
                .to_str()
                .ok_or_else(|| format!("non-utf8 worktree path: {}", root_path.display()))?,
            "HEAD",
        ],
    )
    .await
    .map_err(|err| format!("failed to create git worktree: {err}"))?;

    let worktree = DevflowWorktree {
        id: task
            .worktree_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        task_id: task.id.clone(),
        project_id: task.project_id.clone(),
        repo_root: repo_root.display().to_string(),
        root_path: root_path.display().to_string(),
        cwd_path: cwd_path.display().to_string(),
        branch,
        base_branch,
        base_commit,
        head_commit: get_head_commit_hash(&root_path).await.map(|sha| sha.0),
        managed: true,
        status: DevflowWorktreeStatus::Active,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    save_persisted_worktree(codex_home, &worktree)
        .await
        .map_err(|err| format!("failed to persist worktree metadata: {err}"))?;
    Ok(worktree)
}

pub(crate) async fn read_managed_worktree(
    codex_home: &Path,
    worktree_id: &str,
) -> Result<DevflowWorktree, String> {
    let metadata_path = metadata_file_path(codex_home, worktree_id);
    let persisted = load_persisted_worktree(&metadata_path)
        .await?
        .ok_or_else(|| format!("unknown devflow worktree id: {worktree_id}"))?;
    refresh_worktree_status(persisted.worktree).await
}

pub(crate) async fn list_managed_worktrees(
    codex_home: &Path,
) -> Result<Vec<DevflowWorktree>, String> {
    let metadata_root = codex_home.join("devflow").join("worktree-metadata");
    let mut entries = match fs::read_dir(&metadata_root).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read worktree metadata dir: {err}")),
    };

    let mut worktrees = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| format!("failed to iterate worktree metadata dir: {err}"))?
    {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Some(persisted) = load_persisted_worktree(&path).await? else {
            continue;
        };
        worktrees.push(refresh_worktree_status(persisted.worktree).await?);
    }
    worktrees.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(worktrees)
}

pub(crate) async fn worktree_diff(
    codex_home: &Path,
    worktree_id: &str,
) -> Result<(DevflowWorktree, String), String> {
    let worktree = read_managed_worktree(codex_home, worktree_id).await?;
    let diff = run_git_output(
        Path::new(&worktree.root_path),
        &[
            "diff",
            "--no-textconv",
            "--no-ext-diff",
            &worktree.base_commit,
        ],
    )
    .await
    .map_err(|err| format!("failed to compute worktree diff: {err}"))?;
    Ok((worktree, diff))
}

pub(crate) async fn cleanup_managed_worktree(
    codex_home: &Path,
    worktree_id: &str,
) -> Result<DevflowWorktree, String> {
    let metadata_path = metadata_file_path(codex_home, worktree_id);
    let persisted = load_persisted_worktree(&metadata_path)
        .await?
        .ok_or_else(|| format!("unknown devflow worktree id: {worktree_id}"))?;
    let mut worktree = persisted.worktree;

    if worktree.task_id.trim().is_empty() {
        return Err(format!(
            "cannot cleanup worktree with unknown owner: {worktree_id}"
        ));
    }
    if !worktree.managed {
        return Err(format!("cannot cleanup unmanaged worktree: {worktree_id}"));
    }

    let repo_root = PathBuf::from(&worktree.repo_root);
    let root_path = PathBuf::from(&worktree.root_path);
    let metadata = fs::metadata(&root_path)
        .await
        .map_err(|err| format!("cannot cleanup unreadable worktree: {err}"))?;
    if !metadata.is_dir() {
        return Err(format!(
            "cannot cleanup unreadable worktree directory: {}",
            root_path.display()
        ));
    }

    let canonical_root = fs::canonicalize(&root_path)
        .await
        .map_err(|err| format!("cannot cleanup unreadable worktree path: {err}"))?;
    let canonical_repo = fs::canonicalize(&repo_root)
        .await
        .map_err(|err| format!("cannot cleanup unreadable repo root: {err}"))?;
    if canonical_root == canonical_repo {
        return Err(format!(
            "cannot cleanup primary worktree: {}",
            root_path.display()
        ));
    }

    if !is_linked_worktree_for_repo(&root_path, &repo_root).await? {
        return Err(format!(
            "cannot cleanup unmanaged worktree: {}",
            root_path.display()
        ));
    }

    if worktree_is_dirty(&root_path).await? {
        return Err(format!(
            "cannot cleanup dirty worktree: {}",
            root_path.display()
        ));
    }

    run_git_command(
        &repo_root,
        &[
            "worktree",
            "remove",
            root_path
                .to_str()
                .ok_or_else(|| format!("non-utf8 worktree path: {}", root_path.display()))?,
        ],
    )
    .await
    .map_err(|err| format!("failed to remove worktree: {err}"))?;

    worktree.status = DevflowWorktreeStatus::Cleaned;
    worktree.updated_at = chrono::Utc::now().timestamp();
    worktree.head_commit = None;
    save_persisted_worktree(codex_home, &worktree)
        .await
        .map_err(|err| format!("failed to persist cleaned worktree metadata: {err}"))?;
    Ok(worktree)
}

pub(crate) async fn merge_managed_worktree(
    codex_home: &Path,
    worktree_id: &str,
) -> Result<WorktreeMergeOutcome, String> {
    let worktree = read_managed_worktree(codex_home, worktree_id).await?;
    if !worktree.managed {
        return Err(format!("cannot merge unmanaged worktree: {worktree_id}"));
    }

    let repo_root = PathBuf::from(&worktree.repo_root);
    let primary_status = run_git_output(
        &repo_root,
        &[
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--",
            ".",
            ":(exclude).codex",
        ],
    )
    .await?;
    if !primary_status.trim().is_empty() {
        return Err(format!(
            "cannot merge into dirty primary worktree: {}",
            repo_root.display()
        ));
    }

    let diff = run_git_output(
        Path::new(&worktree.root_path),
        &[
            "diff",
            "--binary",
            "--no-textconv",
            "--no-ext-diff",
            &worktree.base_commit,
        ],
    )
    .await
    .map_err(|err| format!("failed to compute worktree diff for merge: {err}"))?;
    if diff.trim().is_empty() {
        return Ok(WorktreeMergeOutcome {
            worktree,
            merged: true,
            conflicts: Vec::new(),
            diff,
        });
    }

    let check_output = run_git_command_with_input(
        &repo_root,
        &["apply", "--check", "--3way", "--index", "-"],
        diff.as_bytes(),
    )
    .await?;
    if !check_output.status.success() {
        let conflicts = extract_apply_conflicts(&String::from_utf8_lossy(&check_output.stderr));
        return Ok(WorktreeMergeOutcome {
            worktree,
            merged: false,
            conflicts,
            diff,
        });
    }

    let apply_output = run_git_command_with_input(
        &repo_root,
        &["apply", "--3way", "--index", "-"],
        diff.as_bytes(),
    )
    .await?;
    if !apply_output.status.success() {
        run_git_command(&repo_root, &["reset", "--hard", "HEAD"])
            .await
            .map_err(|err| {
                format!("failed to restore primary worktree after merge conflict: {err}")
            })?;
        let conflicts = extract_apply_conflicts(&String::from_utf8_lossy(&apply_output.stderr));
        return Ok(WorktreeMergeOutcome {
            worktree,
            merged: false,
            conflicts,
            diff,
        });
    }

    Ok(WorktreeMergeOutcome {
        worktree,
        merged: true,
        conflicts: Vec::new(),
        diff,
    })
}

pub(crate) async fn save_persisted_worktree(
    codex_home: &Path,
    worktree: &DevflowWorktree,
) -> Result<(), std::io::Error> {
    let path = metadata_file_path(codex_home, &worktree.id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(&PersistedDevflowWorktree {
        worktree: worktree.clone(),
    })
    .map_err(std::io::Error::other)?;
    fs::write(path, json).await
}

pub(crate) fn metadata_file_path(codex_home: &Path, worktree_id: &str) -> PathBuf {
    codex_home
        .join("devflow")
        .join("worktree-metadata")
        .join(format!("{worktree_id}.json"))
}

async fn load_persisted_worktree(
    metadata_path: &Path,
) -> Result<Option<PersistedDevflowWorktree>, String> {
    match fs::read_to_string(metadata_path).await {
        Ok(contents) => serde_json::from_str(&contents)
            .map(Some)
            .map_err(|err| format!("invalid worktree metadata: {err}")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("failed to read worktree metadata: {err}")),
    }
}

async fn refresh_worktree_status(mut worktree: DevflowWorktree) -> Result<DevflowWorktree, String> {
    if worktree.status == DevflowWorktreeStatus::Cleaned {
        return Ok(worktree);
    }
    let root_path = PathBuf::from(&worktree.root_path);
    if fs::metadata(&root_path).await.is_err() {
        worktree.status = DevflowWorktreeStatus::Missing;
        worktree.head_commit = None;
        worktree.updated_at = chrono::Utc::now().timestamp();
        return Ok(worktree);
    }

    let is_dirty = worktree_is_dirty(&root_path).await?;
    worktree.status = if is_dirty {
        DevflowWorktreeStatus::Dirty
    } else {
        DevflowWorktreeStatus::Active
    };
    worktree.head_commit = get_head_commit_hash(&root_path).await.map(|sha| sha.0);
    worktree.updated_at = chrono::Utc::now().timestamp();
    Ok(worktree)
}

async fn worktree_is_dirty(root_path: &Path) -> Result<bool, String> {
    let output = run_git_output(root_path, &["status", "--porcelain"]).await?;
    Ok(!output.trim().is_empty())
}

async fn is_linked_worktree_for_repo(root_path: &Path, repo_root: &Path) -> Result<bool, String> {
    let dot_git_path = root_path.join(".git");
    let contents = fs::read_to_string(&dot_git_path)
        .await
        .map_err(|err| format!("failed to read worktree .git file: {err}"))?;
    let Some(gitdir_value) = contents.trim().strip_prefix("gitdir:") else {
        return Ok(false);
    };
    let gitdir_value = gitdir_value.trim();
    if gitdir_value.is_empty() {
        return Ok(false);
    }
    let gitdir_path = if Path::new(gitdir_value).is_absolute() {
        PathBuf::from(gitdir_value)
    } else {
        root_path.join(gitdir_value)
    };
    let expected_prefix = fs::canonicalize(repo_root.join(".git").join("worktrees"))
        .await
        .map_err(|err| format!("failed to resolve repo worktrees dir: {err}"))?;
    let canonical_gitdir = fs::canonicalize(&gitdir_path)
        .await
        .map_err(|err| format!("failed to resolve worktree gitdir path: {err}"))?;
    Ok(canonical_gitdir.starts_with(expected_prefix))
}

fn worktree_cwd_path(
    repo_root: &Path,
    task_project_root: &Path,
    root_path: &Path,
) -> Result<PathBuf, String> {
    let relative = task_project_root
        .strip_prefix(repo_root)
        .map_err(|err| format!("task project root is outside repo root: {err}"))?;
    Ok(if relative.as_os_str().is_empty() {
        root_path.to_path_buf()
    } else {
        root_path.join(relative)
    })
}

fn short_task_slug(task_id: &str) -> String {
    task_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect::<String>()
        .to_ascii_lowercase()
}

async fn run_git_output(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = run_git_command(cwd, args).await?;
    String::from_utf8(output.stdout).map_err(|err| format!("git output was not utf-8: {err}"))
}

async fn run_git_command(cwd: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let mut command = Command::new("git");
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(cwd)
        .kill_on_drop(true);
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(GIT_WORKTREE_TIMEOUT_SECS),
        command.output(),
    )
    .await
    .map_err(|_| format!("git command timed out after {GIT_WORKTREE_TIMEOUT_SECS}s"))?
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

async fn run_git_command_with_input(
    cwd: &Path,
    args: &[&str],
    stdin_bytes: &[u8],
) -> Result<std::process::Output, String> {
    let mut command = Command::new("git");
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|err| format!("failed to spawn git command: {err}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(stdin_bytes)
            .await
            .map_err(|err| format!("failed to write git stdin: {err}"))?;
    }
    tokio::time::timeout(
        std::time::Duration::from_secs(GIT_WORKTREE_TIMEOUT_SECS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| format!("git command timed out after {GIT_WORKTREE_TIMEOUT_SECS}s"))?
    .map_err(|err| format!("failed waiting for git command: {err}"))
}

fn extract_apply_conflicts(stderr: &str) -> Vec<String> {
    let mut conflicts = stderr
        .lines()
        .filter_map(|line| {
            line.split_once(": ")
                .map(|(_, rest)| rest.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .collect::<Vec<_>>();
    conflicts.sort();
    conflicts.dedup();
    if conflicts.is_empty() && !stderr.trim().is_empty() {
        conflicts.push(stderr.trim().to_string());
    }
    conflicts
}
