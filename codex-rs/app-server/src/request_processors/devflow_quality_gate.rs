use std::path::Path;
use std::process::Stdio;
use std::time::Instant;

use codex_app_server_protocol::DevflowQualityGateKind;
use codex_app_server_protocol::DevflowTask;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GateCommand {
    pub(crate) command: String,
    pub(crate) argv: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct QualityGateExecution {
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: i64,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn quality_gate_command(
    project_root: &Path,
    task: &DevflowTask,
    kind: DevflowQualityGateKind,
) -> Result<GateCommand, String> {
    let is_rust_project = project_root.join("Cargo.toml").exists();
    let command = match kind {
        DevflowQualityGateKind::Format => {
            if is_rust_project {
                gate_command("cargo fmt --check", &["cargo", "fmt", "--check"])
            } else {
                package_script_command(project_root, "format:check")
                    .unwrap_or_else(git_diff_check_command)
            }
        }
        DevflowQualityGateKind::Lint => {
            if is_rust_project {
                gate_command(
                    "cargo clippy --workspace --all-targets -- -D warnings",
                    &[
                        "cargo",
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--",
                        "-D",
                        "warnings",
                    ],
                )
            } else {
                package_script_command(project_root, "lint").unwrap_or_else(git_diff_check_command)
            }
        }
        DevflowQualityGateKind::Typecheck => {
            if is_rust_project {
                gate_command(
                    "cargo check --workspace --all-targets",
                    &["cargo", "check", "--workspace", "--all-targets"],
                )
            } else {
                package_script_command(project_root, "typecheck")
                    .unwrap_or_else(git_diff_check_command)
            }
        }
        DevflowQualityGateKind::TargetedTest => targeted_test_gate_command(project_root, task),
        DevflowQualityGateKind::IntegrationTest => {
            if is_rust_project {
                gate_command(
                    "cargo test --workspace --tests",
                    &["cargo", "test", "--workspace", "--tests"],
                )
            } else {
                package_script_command(project_root, "test:integration")
                    .or_else(|| package_script_command(project_root, "integration:test"))
                    .or_else(|| package_script_command(project_root, "integration-test"))
                    .unwrap_or_else(git_diff_check_command)
            }
        }
        DevflowQualityGateKind::Snapshot => {
            if is_rust_project {
                gate_command(
                    "cargo test --workspace snapshot",
                    &["cargo", "test", "--workspace", "snapshot"],
                )
            } else {
                package_script_command(project_root, "test:snapshot")
                    .or_else(|| package_script_command(project_root, "snapshot"))
                    .or_else(|| package_script_command(project_root, "snapshots"))
                    .unwrap_or_else(git_diff_check_command)
            }
        }
        DevflowQualityGateKind::Build => {
            if is_rust_project {
                gate_command(
                    "cargo build --workspace",
                    &["cargo", "build", "--workspace"],
                )
            } else {
                package_script_command(project_root, "build").unwrap_or_else(git_diff_check_command)
            }
        }
        DevflowQualityGateKind::Review => git_diff_check_command(),
        DevflowQualityGateKind::GstackHealth
        | DevflowQualityGateKind::GstackBrowserQa
        | DevflowQualityGateKind::GstackBenchmark
        | DevflowQualityGateKind::GstackCanary
        | DevflowQualityGateKind::GstackWatchdog => {
            return Err(format!(
                "{kind:?} gates are produced by devflowCapabilityPack/run"
            ));
        }
    };
    Ok(command)
}

fn targeted_test_gate_command(project_root: &Path, task: &DevflowTask) -> GateCommand {
    if project_root.join("Cargo.toml").exists() {
        let text = format!("{} {}", task.title, task.objective).to_ascii_lowercase();
        if text.contains("devflow")
            || text.contains("worktree")
            || text.contains("quality gate")
            || text.contains("review")
        {
            return gate_command(
                "cargo test -p codex-app-server --test all devflow",
                &[
                    "cargo",
                    "test",
                    "-p",
                    "codex-app-server",
                    "--test",
                    "all",
                    "devflow",
                ],
            );
        }
        return gate_command("cargo test", &["cargo", "test"]);
    }

    git_diff_check_command()
}

fn gate_command(command: &str, argv: &[&str]) -> GateCommand {
    GateCommand {
        command: command.to_string(),
        argv: argv.iter().copied().map(str::to_string).collect(),
    }
}

fn git_diff_check_command() -> GateCommand {
    gate_command("git diff --check", &["git", "diff", "--check"])
}

fn package_script_command(project_root: &Path, script_name: &str) -> Option<GateCommand> {
    let package_json = std::fs::read_to_string(project_root.join("package.json")).ok()?;
    let package_json = serde_json::from_str::<Value>(&package_json).ok()?;
    let scripts = package_json.get("scripts")?.as_object()?;
    if !scripts.contains_key(script_name) {
        return None;
    }

    let runner = if project_root.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if project_root.join("yarn.lock").exists() {
        "yarn"
    } else if project_root.join("bun.lock").exists() || project_root.join("bun.lockb").exists() {
        "bun"
    } else {
        "npm"
    };
    Some(gate_command(
        &format!("{runner} run {script_name}"),
        &[runner, "run", script_name],
    ))
}

pub(crate) async fn run_gate_command(
    cwd: &Path,
    gate_command: &GateCommand,
) -> Result<QualityGateExecution, String> {
    if gate_command.argv.is_empty() {
        return Err("quality gate command is empty".to_string());
    }

    let started_at = Instant::now();
    let mut command = Command::new(&gate_command.argv[0]);
    command
        .args(gate_command.argv.iter().skip(1))
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|err| format!("failed to spawn quality gate command: {err}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "missing gate stdout pipe".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "missing gate stderr pipe".to_string())?;

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
        .map_err(|err| format!("failed waiting for quality gate command: {err}"))?;
    let stdout = stdout_task
        .await
        .map_err(|err| format!("failed joining quality gate stdout task: {err}"))?
        .map_err(|err| format!("failed reading quality gate stdout: {err}"))?;
    let stderr = stderr_task
        .await
        .map_err(|err| format!("failed joining quality gate stderr task: {err}"))?
        .map_err(|err| format!("failed reading quality gate stderr: {err}"))?;

    Ok(QualityGateExecution {
        exit_code: status.code(),
        duration_ms: started_at.elapsed().as_millis() as i64,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

pub(crate) fn combine_gate_output(execution: &QualityGateExecution) -> String {
    match (execution.stdout.trim(), execution.stderr.trim()) {
        ("", "") => String::new(),
        (stdout, "") => stdout.to_string(),
        ("", stderr) => stderr.to_string(),
        (stdout, stderr) => format!("{stdout}\n\n[stderr]\n{stderr}"),
    }
}
