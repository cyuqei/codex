use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use chrono::Utc;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowCapabilityPack;
use codex_app_server_protocol::DevflowCapabilityPackListParams;
use codex_app_server_protocol::DevflowCapabilityPackListResponse;
use codex_app_server_protocol::DevflowCapabilityPackReadParams;
use codex_app_server_protocol::DevflowCapabilityPackReadResponse;
use codex_app_server_protocol::DevflowCapabilityPackRunParams;
use codex_app_server_protocol::DevflowCapabilityPackRunResponse;
use codex_app_server_protocol::DevflowCapabilityPackRunStatus;
use codex_app_server_protocol::DevflowPackStatus;
use codex_app_server_protocol::DevflowPolicyPack;
use codex_app_server_protocol::DevflowPolicyPackApplyParams;
use codex_app_server_protocol::DevflowPolicyPackApplyResponse;
use codex_app_server_protocol::DevflowPolicyPackListParams;
use codex_app_server_protocol::DevflowPolicyPackListResponse;
use codex_app_server_protocol::DevflowPolicyPackReadParams;
use codex_app_server_protocol::DevflowPolicyPackReadResponse;
use codex_app_server_protocol::DevflowQualityGateKind;
use codex_app_server_protocol::DevflowQualityGateStatus;
use codex_app_server_protocol::DevflowTaskRiskLevel;
use codex_app_server_protocol::DevflowWatchdogAlert;
use codex_app_server_protocol::DevflowWatchdogAlertSeverity;
use codex_app_server_protocol::DevflowWatchdogAlertsParams;
use codex_app_server_protocol::DevflowWatchdogAlertsResponse;
use codex_app_server_protocol::DevflowWatchdogReadParams;
use codex_app_server_protocol::DevflowWatchdogReadResponse;
use codex_app_server_protocol::DevflowWatchdogStatus;
use codex_app_server_protocol::JSONRPCErrorError;
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::error_code::invalid_request;

use super::devflow_processor::DevflowCapabilityPackGateOutcome;
use super::devflow_processor::DevflowCapabilityPackTarget;
use super::devflow_processor::DevflowRequestProcessor;
use super::devflow_processor::DevflowWatchdogQueueSnapshot;

const DEFAULT_SUPERPOWERS_ROOT: &str = "/Users/yuqei/superpowers";
const DEFAULT_GSTACK_ROOT: &str = "/Users/yuqei/gstack";
const PACK_RUNNER_SCHEMA_VERSION: u32 = 1;
const GSTACK_HEALTH_GIT_TIMEOUT_SECS: u64 = 30;
const GSTACK_HEALTH_STATUS_TIMEOUT_SECS: u64 = 30;
const GSTACK_HEALTH_CARGO_TIMEOUT_SECS: u64 = 120;
const GSTACK_BROWSE_GOTO_TIMEOUT_SECS: u64 = 20;
const GSTACK_BROWSE_SCREENSHOT_TIMEOUT_SECS: u64 = 20;
const GSTACK_REVIEW_GIT_TIMEOUT_SECS: u64 = 30;
const COMMAND_OUTPUT_TAIL_LIMIT: usize = 4000;

#[derive(Clone)]
pub(super) struct AllowedPackCommand {
    pub(super) name: &'static str,
    pub(super) argv: Vec<String>,
    pub(super) timeout_secs: u64,
    pub(super) expectation: AllowedCommandExpectation,
}

#[derive(Clone, Copy)]
pub(super) enum AllowedCommandExpectation {
    ExitZero,
    EmptyStdout,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PackCommandReport {
    pub(super) name: String,
    pub(super) command: String,
    pub(super) argv: Vec<String>,
    pub(super) cwd: String,
    pub(super) status: String,
    pub(super) score: Option<u8>,
    pub(super) details: String,
    pub(super) exit_code: Option<i32>,
    pub(super) duration_ms: i64,
    pub(super) timeout_secs: u64,
    pub(super) stdout_tail: String,
    pub(super) stderr_tail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PackDashboardDimension {
    name: String,
    status: String,
    score: Option<u8>,
    details: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrowseQaTargetCandidate {
    kind: String,
    value: String,
    source: String,
    confidence: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewRiskHotspot {
    kind: String,
    file: String,
    severity: String,
    reason: String,
}

impl DevflowRequestProcessor {
    pub(crate) async fn policy_pack_list(
        &self,
        params: DevflowPolicyPackListParams,
    ) -> Result<DevflowPolicyPackListResponse, JSONRPCErrorError> {
        let mut data = default_policy_packs();
        if !params.include_disabled {
            data.retain(|pack| pack.status != DevflowPackStatus::Disabled);
        }
        Ok(DevflowPolicyPackListResponse { data })
    }

    pub(crate) async fn policy_pack_read(
        &self,
        params: DevflowPolicyPackReadParams,
    ) -> Result<DevflowPolicyPackReadResponse, JSONRPCErrorError> {
        let pack = find_policy_pack(&params.id)?;
        Ok(DevflowPolicyPackReadResponse { pack })
    }

    pub(crate) async fn policy_pack_apply(
        &self,
        params: DevflowPolicyPackApplyParams,
    ) -> Result<DevflowPolicyPackApplyResponse, JSONRPCErrorError> {
        let pack = find_policy_pack(&params.id)?;
        let mut diagnostics = pack.diagnostics.clone();
        if let Some(task_id) = &params.task_id {
            diagnostics.push(format!("policy pack scoped to task {task_id}"));
        }
        if let Some(risk_level) = params.risk_level {
            diagnostics.push(format!(
                "policy profile selected for {risk_level:?} risk task"
            ));
        }
        let application = self
            .record_policy_pack_application(
                &pack,
                params.task_id.as_deref(),
                params.risk_level,
                diagnostics,
            )
            .await?;
        Ok(DevflowPolicyPackApplyResponse {
            pack,
            applied: true,
            required_artifacts: application.required_artifacts,
            diagnostics: application.diagnostics,
            artifact: application.artifact,
        })
    }

    pub(crate) async fn capability_pack_list(
        &self,
        params: DevflowCapabilityPackListParams,
    ) -> Result<DevflowCapabilityPackListResponse, JSONRPCErrorError> {
        let mut data = default_capability_packs();
        if !params.include_disabled {
            data.retain(|pack| pack.status != DevflowPackStatus::Disabled);
        }
        Ok(DevflowCapabilityPackListResponse { data })
    }

    pub(crate) async fn capability_pack_read(
        &self,
        params: DevflowCapabilityPackReadParams,
    ) -> Result<DevflowCapabilityPackReadResponse, JSONRPCErrorError> {
        let pack = find_capability_pack(&params.id)?;
        Ok(DevflowCapabilityPackReadResponse { pack })
    }

    pub(crate) async fn capability_pack_run(
        &self,
        params: DevflowCapabilityPackRunParams,
    ) -> Result<DevflowCapabilityPackRunResponse, JSONRPCErrorError> {
        let pack = find_capability_pack(&params.id)?;
        let capability = params.capability.unwrap_or_else(|| {
            pack.capabilities
                .first()
                .cloned()
                .unwrap_or_else(|| "default".to_string())
        });
        if !pack.capabilities.contains(&capability) {
            return Err(invalid_request(format!(
                "unknown capability `{capability}` for devflow capability pack {}",
                pack.id
            )));
        }
        let target = self
            .resolve_capability_pack_target(
                params.task_id.as_deref(),
                params.project_root.as_deref(),
            )
            .await?;
        let (status, summary, artifact) = match capability.as_str() {
            "health" => self.run_gstack_health_capability(&target).await?,
            "browseQa" => self.run_gstack_browse_qa_capability(&target).await?,
            "review" => self.run_gstack_review_capability(&target).await?,
            "benchmark" => self.run_gstack_benchmark_capability(&target).await?,
            "canary" => self.run_gstack_canary_capability(&target).await?,
            "watchdogQueue" => self.run_gstack_watchdog_queue_capability(&target).await?,
            _ => {
                return Ok(DevflowCapabilityPackRunResponse {
                    pack,
                    status: DevflowCapabilityPackRunStatus::Skipped,
                    summary: format!(
                        "Capability `{capability}` is registered but not wired into the Codex-owned pack runner yet."
                    ),
                    artifact: None,
                });
            }
        };
        Ok(DevflowCapabilityPackRunResponse {
            pack,
            status,
            summary,
            artifact: Some(artifact),
        })
    }

    async fn run_gstack_health_capability(
        &self,
        target: &DevflowCapabilityPackTarget,
    ) -> Result<(DevflowCapabilityPackRunStatus, String, DevflowArtifact), JSONRPCErrorError> {
        let dimensions = gstack_health_dimensions(target);
        let commands = gstack_health_commands(target);
        let mut reports = Vec::new();
        for command in &commands {
            reports.push(run_allowed_pack_command(&target.cwd_path, command).await);
        }

        let scores = dimensions
            .iter()
            .filter_map(|dimension| dimension.score)
            .chain(reports.iter().filter_map(|report| report.score))
            .collect::<Vec<_>>();
        let attempted = scores.len();
        let passed = scores.iter().filter(|score| **score >= 7).count();
        let failed = scores.iter().filter(|score| **score < 7).count();
        let status = if attempted == 0 {
            DevflowCapabilityPackRunStatus::Skipped
        } else if failed == 0 {
            DevflowCapabilityPackRunStatus::Completed
        } else {
            DevflowCapabilityPackRunStatus::Failed
        };
        let summary = match status {
            DevflowCapabilityPackRunStatus::Completed => {
                format!("gstack health completed: {passed}/{attempted} allowlisted checks passed")
            }
            DevflowCapabilityPackRunStatus::Failed => {
                format!("gstack health failed: {failed}/{attempted} allowlisted checks failed")
            }
            DevflowCapabilityPackRunStatus::Skipped => {
                "gstack health skipped: no allowlisted checks were applicable".to_string()
            }
        };
        let score = (!scores.is_empty()).then(|| {
            (scores.iter().map(|score| f64::from(*score)).sum::<f64>() / scores.len() as f64)
                .round()
        });
        let timed_out = reports.iter().any(|report| report.status == "timed_out");
        let failed_gate_report = reports
            .iter()
            .find(|report| report.score.is_some_and(|score| score < 7))
            .map(|report| {
                (
                    report.command.clone(),
                    report.exit_code,
                    Some(report.duration_ms),
                    report.details.clone(),
                )
            });
        let report = serde_json::json!({
            "schemaVersion": PACK_RUNNER_SCHEMA_VERSION,
            "runner": "codex-owned-pack-runner",
            "packId": "gstack-engineering",
            "capability": "health",
            "status": format!("{status:?}").to_ascii_lowercase(),
            "summary": summary.clone(),
            "score": score,
            "policy": {
                "approval": "No arbitrary shell execution. The runner only executes hard-coded allowlisted commands and returns skipped for capabilities that are not wired yet.",
                    "commandAllowlist": command_allowlist(),
                    "timeoutSeconds": {
                    "gitStatus": GSTACK_HEALTH_STATUS_TIMEOUT_SECS,
                    "gitDiffCheck": GSTACK_HEALTH_GIT_TIMEOUT_SECS,
                    "cargoCheck": GSTACK_HEALTH_CARGO_TIMEOUT_SECS,
                },
                "scope": {
                    "projectRoot": target.project_root,
                    "cwd": target.cwd_path.display().to_string(),
                    "worktreeId": target.worktree_id.clone(),
                    "writes": "Only Devflow artifact files are written by the runner itself. Cargo checks are only run when the task is scoped to a managed worktree.",
                },
                "artifactFormat": "application/json; schemaVersion=1; command reports include argv, cwd, status, exitCode, durationMs, timeoutSecs, stdoutTail, and stderrTail.",
            },
            "dimensions": dimensions,
            "commands": reports,
        });
        let content = serde_json::to_string_pretty(&report).map_err(|err| {
            crate::error_code::internal_error(format!(
                "failed to serialize gstack health report: {err}"
            ))
        })?;
        let artifact = self
            .write_capability_pack_artifact(target, "health", &content, summary.clone())
            .await?;
        if status == DevflowCapabilityPackRunStatus::Failed {
            let (alert_status, severity) = if timed_out {
                (
                    DevflowWatchdogStatus::TimedOut,
                    DevflowWatchdogAlertSeverity::Critical,
                )
            } else {
                (
                    DevflowWatchdogStatus::NoProgress,
                    DevflowWatchdogAlertSeverity::Warning,
                )
            };
            self.record_watchdog_alert(
                alert_status,
                severity,
                Some(target.project_root.clone()),
                Some(target.task_id.clone()),
                Some(target.run_id.clone()),
                format!("{summary}; see artifact {}", artifact.id),
            )
            .await;
        }
        if let Some(gate_status) = capability_pack_gate_status(status) {
            let (command, exit_code, duration_ms, gate_summary) =
                if let Some((command, exit_code, duration_ms, details)) = failed_gate_report {
                    (
                        command,
                        exit_code,
                        duration_ms,
                        format!("{summary}; {details}; see artifact {}", artifact.id),
                    )
                } else {
                    (
                        capability_pack_gate_command("health"),
                        None,
                        None,
                        format!("{summary}; see artifact {}", artifact.id),
                    )
                };
            self.record_capability_pack_quality_gate(
                target,
                DevflowCapabilityPackGateOutcome {
                    kind: DevflowQualityGateKind::GstackHealth,
                    capability: "health",
                    status: gate_status,
                    command,
                    exit_code,
                    duration_ms,
                    summary: gate_summary,
                },
                &artifact,
            )
            .await;
        }
        Ok((status, summary, artifact))
    }

    async fn run_gstack_browse_qa_capability(
        &self,
        target: &DevflowCapabilityPackTarget,
    ) -> Result<(DevflowCapabilityPackRunStatus, String, DevflowArtifact), JSONRPCErrorError> {
        let candidates = browse_qa_target_candidates(target);
        let selected_target_url = browse_qa_selected_target_url(&candidates);
        let browser_binary = browse_qa_browser_binary(target);
        let screenshot_path = browse_qa_screenshot_path(target);
        let mut browser_commands = Vec::new();
        let mut setup_error = None;
        if let (Some(target_url), Some(binary)) = (&selected_target_url, &browser_binary) {
            if let Some(parent) = screenshot_path.parent()
                && let Err(err) = fs::create_dir_all(parent)
            {
                setup_error = Some(format!(
                    "failed to prepare screenshot artifact directory: {err}"
                ));
            }
            if setup_error.is_none() {
                let goto = AllowedPackCommand {
                    name: "browseGoto",
                    argv: vec!["browse".to_string(), "goto".to_string(), target_url.clone()],
                    timeout_secs: GSTACK_BROWSE_GOTO_TIMEOUT_SECS,
                    expectation: AllowedCommandExpectation::ExitZero,
                };
                browser_commands.push(
                    run_allowed_pack_command_with_binary(&target.cwd_path, &goto, binary).await,
                );
                if browser_commands
                    .last()
                    .is_some_and(|report| report.status == "completed")
                {
                    let screenshot = AllowedPackCommand {
                        name: "browseScreenshot",
                        argv: vec![
                            "browse".to_string(),
                            "screenshot".to_string(),
                            screenshot_path.display().to_string(),
                        ],
                        timeout_secs: GSTACK_BROWSE_SCREENSHOT_TIMEOUT_SECS,
                        expectation: AllowedCommandExpectation::ExitZero,
                    };
                    browser_commands.push(
                        run_allowed_pack_command_with_binary(&target.cwd_path, &screenshot, binary)
                            .await,
                    );
                }
            }
        }
        let screenshot_captured = screenshot_path
            .metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0);
        let browser_command_failed = browser_commands
            .iter()
            .any(|report| report.score.is_some_and(|score| score < 7));
        let browser_attempted = selected_target_url.is_some() && browser_binary.is_some();
        let status = if setup_error.is_some()
            || browser_command_failed
            || (browser_attempted && !screenshot_captured)
        {
            DevflowCapabilityPackRunStatus::Failed
        } else if !browser_attempted {
            DevflowCapabilityPackRunStatus::Skipped
        } else {
            DevflowCapabilityPackRunStatus::Completed
        };
        let dimensions = gstack_browse_qa_dimensions(
            target,
            &candidates,
            browser_binary.as_deref(),
            selected_target_url.as_deref(),
            screenshot_captured,
            status == DevflowCapabilityPackRunStatus::Failed,
        );
        let summary = match status {
            DevflowCapabilityPackRunStatus::Completed => format!(
                "gstack browseQa completed: captured browser screenshot for {}",
                selected_target_url.as_deref().unwrap_or("selected target")
            ),
            DevflowCapabilityPackRunStatus::Skipped => {
                if candidates.is_empty() {
                    "gstack browseQa skipped: no web surface candidates were detected".to_string()
                } else if selected_target_url.is_none() {
                    "gstack browseQa skipped: no local browser-safe target URL was selected"
                        .to_string()
                } else {
                    "gstack browseQa skipped: gstack browse binary was not found".to_string()
                }
            }
            DevflowCapabilityPackRunStatus::Failed => {
                "gstack browseQa failed: controlled browser capture did not complete".to_string()
            }
        };
        let screenshot_artifact_status = if screenshot_captured {
            "captured"
        } else if browser_attempted {
            "missing"
        } else {
            "not_started"
        };
        let timed_out = browser_commands
            .iter()
            .any(|report| report.status == "timed_out");
        let failed_gate_report = browser_commands
            .iter()
            .find(|report| report.score.is_some_and(|score| score < 7))
            .map(|report| {
                (
                    report.command.clone(),
                    report.exit_code,
                    Some(report.duration_ms),
                    report.details.clone(),
                )
            })
            .or_else(|| {
                if status != DevflowCapabilityPackRunStatus::Failed {
                    return None;
                }
                let details = setup_error
                    .clone()
                    .unwrap_or_else(|| "screenshot artifact was not created".to_string());
                let command = if setup_error.is_some() {
                    "browseQa setup".to_string()
                } else {
                    "browse screenshot <devflow-artifact-png>".to_string()
                };
                Some((command, None, None, details))
            });
        let report = serde_json::json!({
            "schemaVersion": PACK_RUNNER_SCHEMA_VERSION,
            "runner": "codex-owned-pack-runner",
            "packId": "gstack-engineering",
            "capability": "browseQa",
            "status": format!("{status:?}").to_ascii_lowercase(),
            "summary": summary.clone(),
            "policy": {
                "approval": "No arbitrary shell execution. The runner only invokes a detected gstack browse binary with hard-coded goto and screenshot commands against a selected local target URL.",
                "commandAllowlist": browse_qa_command_allowlist(selected_target_url.as_deref()),
                "browserDaemon": {
                    "owner": "gstack-browse-cli",
                    "ownership": "Codex owns the Devflow invocation and screenshot artifact file. The gstack browse CLI owns daemon startup, connection reuse, and idle shutdown.",
                    "stateFile": target.cwd_path.join(".gstack").join("browse.json").display().to_string(),
                    "binary": browser_binary.clone(),
                },
                "targetSelection": {
                    "selectedUrl": selected_target_url.clone(),
                    "rules": [
                        "Prefer explicit http://127.0.0.1 or http://localhost URL candidates discovered from package scripts or README files.",
                        "Fall back to file:// URLs for static entrypoints such as index.html or public/index.html.",
                        "Do not execute package.json scripts or start application servers in this runner stage."
                    ],
                },
                "screenshotCapture": {
                    "format": "png",
                    "mimeType": "image/png",
                    "status": screenshot_artifact_status,
                    "path": screenshot_path.display().to_string(),
                },
                "timeoutSeconds": {
                    "goto": GSTACK_BROWSE_GOTO_TIMEOUT_SECS,
                    "screenshot": GSTACK_BROWSE_SCREENSHOT_TIMEOUT_SECS,
                },
                "cleanup": {
                    "daemon": "left to gstack browse idle shutdown; Codex does not kill shared browser daemons from this capability.",
                    "files": "Screenshot files are stored under .codex/devflow/artifacts and are owned by Devflow cleanup with other run artifacts.",
                },
                "scope": {
                    "projectRoot": target.project_root,
                    "cwd": target.cwd_path.display().to_string(),
                    "worktreeId": target.worktree_id.clone(),
                    "writes": "Only Devflow artifact files are written by the runner itself. Package manager scripts are never started by browseQa.",
                },
                "artifactFormat": "application/json; schemaVersion=1; target candidates include kind, value, source, and confidence; screenshotArtifact records status, path, mimeType, format, and sourceUrl.",
            },
            "dimensions": dimensions,
            "targetCandidates": candidates,
            "selectedTargetUrl": selected_target_url.clone(),
            "browserCommands": browser_commands,
            "setupError": setup_error,
            "screenshotArtifact": {
                "status": screenshot_artifact_status,
                "kind": "screenshot",
                "path": screenshot_path.display().to_string(),
                "mimeType": "image/png",
                "format": "png",
                "sourceUrl": selected_target_url.clone(),
                "capturedAt": screenshot_captured.then(|| Utc::now().timestamp()),
            },
        });
        let content = serde_json::to_string_pretty(&report).map_err(|err| {
            crate::error_code::internal_error(format!(
                "failed to serialize gstack browseQa report: {err}"
            ))
        })?;
        let artifact = self
            .write_capability_pack_artifact(target, "browseQa", &content, summary.clone())
            .await?;
        if status == DevflowCapabilityPackRunStatus::Failed {
            let (alert_status, severity) = if timed_out {
                (
                    DevflowWatchdogStatus::TimedOut,
                    DevflowWatchdogAlertSeverity::Critical,
                )
            } else {
                (
                    DevflowWatchdogStatus::NoProgress,
                    DevflowWatchdogAlertSeverity::Warning,
                )
            };
            self.record_watchdog_alert(
                alert_status,
                severity,
                Some(target.project_root.clone()),
                Some(target.task_id.clone()),
                Some(target.run_id.clone()),
                format!("{summary}; see artifact {}", artifact.id),
            )
            .await;
        }
        if let Some(gate_status) = capability_pack_gate_status(status) {
            let (command, exit_code, duration_ms, gate_summary) =
                if let Some((command, exit_code, duration_ms, details)) = failed_gate_report {
                    (
                        command,
                        exit_code,
                        duration_ms,
                        format!("{summary}; {details}; see artifact {}", artifact.id),
                    )
                } else {
                    (
                        capability_pack_gate_command("browseQa"),
                        None,
                        None,
                        format!("{summary}; see artifact {}", artifact.id),
                    )
                };
            self.record_capability_pack_quality_gate(
                target,
                DevflowCapabilityPackGateOutcome {
                    kind: DevflowQualityGateKind::GstackBrowserQa,
                    capability: "browseQa",
                    status: gate_status,
                    command,
                    exit_code,
                    duration_ms,
                    summary: gate_summary,
                },
                &artifact,
            )
            .await;
        }
        Ok((status, summary, artifact))
    }

    async fn run_gstack_review_capability(
        &self,
        target: &DevflowCapabilityPackTarget,
    ) -> Result<(DevflowCapabilityPackRunStatus, String, DevflowArtifact), JSONRPCErrorError> {
        let commands = gstack_review_commands(target);
        let mut reports = Vec::new();
        for command in &commands {
            reports.push(run_allowed_pack_command(&target.cwd_path, command).await);
        }
        let changed_files = review_changed_files(&reports);
        let risk_hotspots = review_risk_hotspots(&changed_files);
        let failed_command = reports
            .iter()
            .find(|report| report.score.is_some_and(|score| score < 7));
        let status = if failed_command.is_some() {
            DevflowCapabilityPackRunStatus::Failed
        } else if commands.is_empty() || changed_files.is_empty() {
            DevflowCapabilityPackRunStatus::Skipped
        } else {
            DevflowCapabilityPackRunStatus::Completed
        };
        let summary = match status {
            DevflowCapabilityPackRunStatus::Completed => format!(
                "gstack review completed: {} changed files inspected, {} risk hotspots surfaced",
                changed_files.len(),
                risk_hotspots.len()
            ),
            DevflowCapabilityPackRunStatus::Failed => {
                "gstack review failed: diff review intake command failed".to_string()
            }
            DevflowCapabilityPackRunStatus::Skipped => {
                if commands.is_empty() {
                    "gstack review skipped: target is not a git checkout".to_string()
                } else {
                    "gstack review skipped: no git diff was available to review".to_string()
                }
            }
        };
        let dimensions = gstack_review_dimensions(
            target,
            !commands.is_empty(),
            &changed_files,
            &risk_hotspots,
            &reports,
        );
        let report = serde_json::json!({
            "schemaVersion": PACK_RUNNER_SCHEMA_VERSION,
            "runner": "codex-owned-pack-runner",
            "packId": "gstack-engineering",
            "capability": "review",
            "status": format!("{status:?}").to_ascii_lowercase(),
            "summary": summary.clone(),
            "policy": {
                "approval": "No arbitrary shell execution and no external reviewer invocation. The runner only executes fixed read-only git diff commands and writes the Devflow report artifact.",
                "reviewMode": "static_diff_intake",
                "commandAllowlist": gstack_review_command_allowlist(),
                "timeoutSeconds": {
                    "gitDiffNameOnly": GSTACK_REVIEW_GIT_TIMEOUT_SECS,
                    "gitDiffStat": GSTACK_REVIEW_GIT_TIMEOUT_SECS,
                    "gitDiffCheck": GSTACK_REVIEW_GIT_TIMEOUT_SECS,
                },
                "scope": {
                    "projectRoot": target.project_root,
                    "cwd": target.cwd_path.display().to_string(),
                    "worktreeId": target.worktree_id.clone(),
                    "writes": "Only Devflow artifact files are written by the runner itself.",
                },
                "artifactFormat": "application/json; schemaVersion=1; changedFiles is a git diff --name-only list, riskHotspots are filename-based review prompts, and commands include stdout/stderr tails.",
            },
            "dimensions": dimensions,
            "changedFiles": &changed_files,
            "riskHotspots": &risk_hotspots,
            "commands": &reports,
            "findings": [],
            "findingSemantics": "This capability produces a structured pre-review intake artifact. It does not assert semantic code-review findings until a dedicated Codex reviewer pass is wired.",
        });
        let content = serde_json::to_string_pretty(&report).map_err(|err| {
            crate::error_code::internal_error(format!(
                "failed to serialize gstack review report: {err}"
            ))
        })?;
        let artifact = self
            .write_capability_pack_artifact(target, "review", &content, summary.clone())
            .await?;
        if status == DevflowCapabilityPackRunStatus::Failed {
            let timed_out = reports.iter().any(|report| report.status == "timed_out");
            let (alert_status, severity) = if timed_out {
                (
                    DevflowWatchdogStatus::TimedOut,
                    DevflowWatchdogAlertSeverity::Critical,
                )
            } else {
                (
                    DevflowWatchdogStatus::NoProgress,
                    DevflowWatchdogAlertSeverity::Warning,
                )
            };
            self.record_watchdog_alert(
                alert_status,
                severity,
                Some(target.project_root.clone()),
                Some(target.task_id.clone()),
                Some(target.run_id.clone()),
                format!("{summary}; see artifact {}", artifact.id),
            )
            .await;
        }
        if let Some(gate_status) = capability_pack_gate_status(status) {
            let (command, exit_code, duration_ms, gate_summary) =
                if let Some(report) = failed_command {
                    (
                        report.command.clone(),
                        report.exit_code,
                        Some(report.duration_ms),
                        format!(
                            "{summary}; {}; see artifact {}",
                            report.details, artifact.id
                        ),
                    )
                } else {
                    (
                        capability_pack_gate_command("review"),
                        None,
                        None,
                        format!("{summary}; see artifact {}", artifact.id),
                    )
                };
            self.record_capability_pack_quality_gate(
                target,
                DevflowCapabilityPackGateOutcome {
                    kind: DevflowQualityGateKind::Review,
                    capability: "review",
                    status: gate_status,
                    command,
                    exit_code,
                    duration_ms,
                    summary: gate_summary,
                },
                &artifact,
            )
            .await;
        }
        Ok((status, summary, artifact))
    }

    async fn run_gstack_watchdog_queue_capability(
        &self,
        target: &DevflowCapabilityPackTarget,
    ) -> Result<(DevflowCapabilityPackRunStatus, String, DevflowArtifact), JSONRPCErrorError> {
        let snapshot = self
            .watchdog_queue_snapshot(Some(&target.project_root))
            .await;
        let queue_status = watchdog_status_label(snapshot.status);
        let dimensions = gstack_watchdog_queue_dimensions(&snapshot);
        let summary = format!(
            "gstack watchdogQueue completed: status {queue_status}; {} running, {} no-progress, {} timed-out, {} recovering, {} blocked, {} alerts",
            snapshot.counts.running,
            snapshot.counts.no_progress,
            snapshot.counts.timed_out,
            snapshot.counts.recovering,
            snapshot.counts.blocked,
            snapshot.counts.alerts
        );
        let report = serde_json::json!({
            "schemaVersion": PACK_RUNNER_SCHEMA_VERSION,
            "runner": "codex-owned-pack-runner",
            "packId": "gstack-engineering",
            "capability": "watchdogQueue",
            "status": "completed",
            "summary": summary.clone(),
            "queueStatus": queue_status,
            "policy": {
                "approval": "Read-only Devflow control-plane projection. No shell commands are executed and no task, gate, or alert state is mutated by this queue view.",
                "queueSemantics": {
                    "running": "Tasks whose task status is running or whose latest run is queued/running.",
                    "noProgress": "Watchdog alerts with no_progress status, projected back to their task/run when available.",
                    "timedOut": "Watchdog alerts with timed_out status, projected back to their task/run when available.",
                    "recovering": "Watchdog alerts with recovering status, including startup recovery failures that may not belong to a single project.",
                    "blocked": "Tasks explicitly marked blocked by dependencies, approvals, or recovery.",
                },
                "scope": {
                    "projectRoot": target.project_root,
                    "cwd": target.cwd_path.display().to_string(),
                    "worktreeId": target.worktree_id.clone(),
                    "writes": "Only this Devflow artifact file is written by the capability runner.",
                },
                "artifactFormat": "application/json; schemaVersion=1; queue contains status, counts, running, noProgress, timedOut, recovering, blocked, alerts, and checkedAt.",
            },
            "dimensions": &dimensions,
            "queue": &snapshot,
        });
        let content = serde_json::to_string_pretty(&report).map_err(|err| {
            crate::error_code::internal_error(format!(
                "failed to serialize gstack watchdogQueue report: {err}"
            ))
        })?;
        let artifact = self
            .write_capability_pack_artifact(target, "watchdogQueue", &content, summary.clone())
            .await?;
        Ok((DevflowCapabilityPackRunStatus::Completed, summary, artifact))
    }

    pub(crate) async fn watchdog_read(
        &self,
        _params: DevflowWatchdogReadParams,
    ) -> Result<DevflowWatchdogReadResponse, JSONRPCErrorError> {
        let alerts = self.watchdog_alert_snapshot().await;
        let status = watchdog_status_for_alerts(&alerts);
        Ok(DevflowWatchdogReadResponse {
            status,
            alerts,
            checked_at: Utc::now().timestamp(),
        })
    }

    pub(crate) async fn watchdog_alerts(
        &self,
        params: DevflowWatchdogAlertsParams,
    ) -> Result<DevflowWatchdogAlertsResponse, JSONRPCErrorError> {
        let mut data = self.watchdog_alert_snapshot().await;
        if let Some(status) = params.status {
            data.retain(|alert| alert.status == status);
        }
        if let Some(severity) = params.severity {
            data.retain(|alert| alert.severity == severity);
        }
        data.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        let start = params
            .cursor
            .as_deref()
            .and_then(|cursor| cursor.parse::<usize>().ok())
            .unwrap_or(0);
        let limit = params.limit.unwrap_or(100).min(500) as usize;
        let page = data
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor =
            (start + page.len() < data.len()).then(|| (start + page.len()).to_string());
        Ok(DevflowWatchdogAlertsResponse {
            data: page,
            next_cursor,
        })
    }
}

fn watchdog_status_for_alerts(alerts: &[DevflowWatchdogAlert]) -> DevflowWatchdogStatus {
    if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::TimedOut)
    {
        DevflowWatchdogStatus::TimedOut
    } else if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::Quarantined)
    {
        DevflowWatchdogStatus::Quarantined
    } else if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::Recovering)
    {
        DevflowWatchdogStatus::Recovering
    } else if alerts
        .iter()
        .any(|alert| alert.status == DevflowWatchdogStatus::NoProgress)
    {
        DevflowWatchdogStatus::NoProgress
    } else {
        DevflowWatchdogStatus::Idle
    }
}

fn gstack_watchdog_queue_dimensions(
    snapshot: &DevflowWatchdogQueueSnapshot,
) -> Vec<PackDashboardDimension> {
    let running_dimension = PackDashboardDimension {
        name: "runningQueue".to_string(),
        status: "completed".to_string(),
        score: Some(10),
        details: if snapshot.counts.running == 0 {
            "no active running tasks".to_string()
        } else {
            format!("{} active running tasks", snapshot.counts.running)
        },
    };
    let no_progress_dimension = if snapshot.counts.no_progress == 0 {
        PackDashboardDimension {
            name: "noProgressQueue".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "no no-progress watchdog alerts".to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "noProgressQueue".to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: format!(
                "{} tasks or runs need no-progress attention",
                snapshot.counts.no_progress
            ),
        }
    };
    let timed_out_dimension = if snapshot.counts.timed_out == 0 {
        PackDashboardDimension {
            name: "timedOutQueue".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "no timed-out watchdog alerts".to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "timedOutQueue".to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: format!(
                "{} tasks or runs exceeded their watchdog timeout",
                snapshot.counts.timed_out
            ),
        }
    };
    let recovering_dimension = if snapshot.counts.recovering == 0 {
        PackDashboardDimension {
            name: "recoveringQueue".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "no recovery watchdog alerts".to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "recoveringQueue".to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: format!(
                "{} recovery issues need operator attention",
                snapshot.counts.recovering
            ),
        }
    };
    let blocked_dimension = PackDashboardDimension {
        name: "blockedQueue".to_string(),
        status: "completed".to_string(),
        score: Some(if snapshot.counts.blocked == 0 { 10 } else { 7 }),
        details: if snapshot.counts.blocked == 0 {
            "no blocked tasks".to_string()
        } else {
            format!(
                "{} blocked tasks are waiting for unblock",
                snapshot.counts.blocked
            )
        },
    };
    let alert_backlog_dimension = PackDashboardDimension {
        name: "alertBacklog".to_string(),
        status: "completed".to_string(),
        score: Some(if snapshot.counts.alerts == 0 { 10 } else { 7 }),
        details: if snapshot.counts.alerts == 0 {
            "watchdog alert backlog is empty".to_string()
        } else {
            format!(
                "{} watchdog alerts are visible in the queue",
                snapshot.counts.alerts
            )
        },
    };
    vec![
        running_dimension,
        no_progress_dimension,
        timed_out_dimension,
        recovering_dimension,
        blocked_dimension,
        alert_backlog_dimension,
    ]
}

fn watchdog_status_label(status: DevflowWatchdogStatus) -> &'static str {
    match status {
        DevflowWatchdogStatus::Idle => "idle",
        DevflowWatchdogStatus::Running => "running",
        DevflowWatchdogStatus::NoProgress => "no_progress",
        DevflowWatchdogStatus::TimedOut => "timed_out",
        DevflowWatchdogStatus::Recovering => "recovering",
        DevflowWatchdogStatus::Quarantined => "quarantined",
    }
}

fn gstack_review_dimensions(
    target: &DevflowCapabilityPackTarget,
    has_git_checkout: bool,
    changed_files: &[String],
    risk_hotspots: &[ReviewRiskHotspot],
    reports: &[PackCommandReport],
) -> Vec<PackDashboardDimension> {
    let diff_dimension = if !has_git_checkout {
        PackDashboardDimension {
            name: "diffInventory".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "target is not a git checkout".to_string(),
        }
    } else if changed_files.is_empty() {
        PackDashboardDimension {
            name: "diffInventory".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "no changed files were present in git diff --name-only".to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "diffInventory".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!("detected {} changed files", changed_files.len()),
        }
    };
    let diff_check_dimension = reports
        .iter()
        .find(|report| report.name == "gitDiffCheck")
        .map(|report| PackDashboardDimension {
            name: "diffHygiene".to_string(),
            status: report.status.clone(),
            score: report.score,
            details: report.details.clone(),
        })
        .unwrap_or(PackDashboardDimension {
            name: "diffHygiene".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "git diff --check was not run".to_string(),
        });
    let hotspot_dimension = if risk_hotspots.is_empty() {
        PackDashboardDimension {
            name: "riskHotspots".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "no filename-based review hotspots detected".to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "riskHotspots".to_string(),
            status: "completed".to_string(),
            score: Some(7),
            details: format!(
                "surfaced {} filename-based review hotspots",
                risk_hotspots.len()
            ),
        }
    };
    let scope_details = if let Some(worktree_id) = &target.worktree_id {
        format!("static review intake running inside managed worktree {worktree_id}")
    } else {
        "static review intake running in project root".to_string()
    };
    vec![
        diff_dimension,
        diff_check_dimension,
        hotspot_dimension,
        PackDashboardDimension {
            name: "executionScope".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: scope_details,
        },
    ]
}

fn review_changed_files(reports: &[PackCommandReport]) -> Vec<String> {
    reports
        .iter()
        .find(|report| report.name == "gitDiffNameOnly" && report.status == "completed")
        .map(|report| {
            report
                .stdout_tail
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with("...[truncated]"))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn review_risk_hotspots(changed_files: &[String]) -> Vec<ReviewRiskHotspot> {
    let mut hotspots = Vec::new();
    for file in changed_files {
        let lower = file.to_ascii_lowercase();
        if lower.contains("migration") || lower.ends_with(".sql") {
            hotspots.push(ReviewRiskHotspot {
                kind: "dataMigration".to_string(),
                file: file.clone(),
                severity: "high".to_string(),
                reason:
                    "database or migration changes should be checked for rollback and data safety"
                        .to_string(),
            });
        }
        if lower.contains("auth")
            || lower.contains("permission")
            || lower.contains("approval")
            || lower.contains("secret")
        {
            hotspots.push(ReviewRiskHotspot {
                kind: "trustBoundary".to_string(),
                file: file.clone(),
                severity: "high".to_string(),
                reason: "auth, permission, approval, or secret paths need trust-boundary review"
                    .to_string(),
            });
        }
        if lower.contains("provider") || lower.contains("model") || lower.contains("api") {
            hotspots.push(ReviewRiskHotspot {
                kind: "providerRuntime".to_string(),
                file: file.clone(),
                severity: "medium".to_string(),
                reason: "provider or API changes can affect runtime compatibility".to_string(),
            });
        }
        if lower.contains("devflow")
            || lower.contains("worktree")
            || lower.contains("quality_gate")
            || lower.contains("watchdog")
        {
            hotspots.push(ReviewRiskHotspot {
                kind: "devflowControlPlane".to_string(),
                file: file.clone(),
                severity: "medium".to_string(),
                reason: "Devflow control-plane changes can affect automation state or gates"
                    .to_string(),
            });
        }
        if lower.contains("test") || lower.contains("snapshot") {
            hotspots.push(ReviewRiskHotspot {
                kind: "testSurface".to_string(),
                file: file.clone(),
                severity: "low".to_string(),
                reason: "test or snapshot changes should match intentional behavior changes"
                    .to_string(),
            });
        }
    }
    hotspots
}

fn gstack_review_commands(target: &DevflowCapabilityPackTarget) -> Vec<AllowedPackCommand> {
    if !target.cwd_path.join(".git").exists() {
        return Vec::new();
    }
    vec![
        AllowedPackCommand {
            name: "gitDiffNameOnly",
            argv: vec![
                "git".to_string(),
                "diff".to_string(),
                "--name-only".to_string(),
            ],
            timeout_secs: GSTACK_REVIEW_GIT_TIMEOUT_SECS,
            expectation: AllowedCommandExpectation::ExitZero,
        },
        AllowedPackCommand {
            name: "gitDiffStat",
            argv: vec!["git".to_string(), "diff".to_string(), "--stat".to_string()],
            timeout_secs: GSTACK_REVIEW_GIT_TIMEOUT_SECS,
            expectation: AllowedCommandExpectation::ExitZero,
        },
        AllowedPackCommand {
            name: "gitDiffCheck",
            argv: vec!["git".to_string(), "diff".to_string(), "--check".to_string()],
            timeout_secs: GSTACK_REVIEW_GIT_TIMEOUT_SECS,
            expectation: AllowedCommandExpectation::ExitZero,
        },
    ]
}

fn gstack_review_command_allowlist() -> Vec<String> {
    vec![
        "git diff --name-only".to_string(),
        "git diff --stat".to_string(),
        "git diff --check".to_string(),
    ]
}

pub(super) fn capability_pack_gate_status(
    status: DevflowCapabilityPackRunStatus,
) -> Option<DevflowQualityGateStatus> {
    match status {
        DevflowCapabilityPackRunStatus::Completed => Some(DevflowQualityGateStatus::Passed),
        DevflowCapabilityPackRunStatus::Failed => Some(DevflowQualityGateStatus::Failed),
        DevflowCapabilityPackRunStatus::Skipped => None,
    }
}

pub(super) fn capability_pack_gate_command(capability: &str) -> String {
    format!("devflowCapabilityPack/run gstack-engineering {capability}")
}

fn gstack_browse_qa_dimensions(
    target: &DevflowCapabilityPackTarget,
    candidates: &[BrowseQaTargetCandidate],
    browser_binary: Option<&str>,
    selected_target_url: Option<&str>,
    screenshot_captured: bool,
    browser_failed: bool,
) -> Vec<PackDashboardDimension> {
    let surface_dimension = if candidates.is_empty() {
        PackDashboardDimension {
            name: "webSurfaceInventory".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "no package script, localhost URL, or static web entrypoint detected"
                .to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "webSurfaceInventory".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!("detected {} candidate browser QA targets", candidates.len()),
        }
    };
    let scope_details = if let Some(worktree_id) = &target.worktree_id {
        format!("controlled browser QA running inside managed worktree {worktree_id}")
    } else {
        "controlled browser QA running in project root".to_string()
    };
    let browser_daemon_dimension = match (selected_target_url, browser_binary) {
        (None, _) => PackDashboardDimension {
            name: "browserDaemonReadiness".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "no local target URL was selected for the browser daemon".to_string(),
        },
        (Some(_), None) => PackDashboardDimension {
            name: "browserDaemonReadiness".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "gstack browse binary was not found in project, user, or PATH locations"
                .to_string(),
        },
        (Some(target_url), Some(binary)) => PackDashboardDimension {
            name: "browserDaemonReadiness".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!("selected {target_url} via {binary}"),
        },
    };
    let screenshot_dimension = if screenshot_captured {
        PackDashboardDimension {
            name: "screenshotCapture".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "captured PNG screenshot artifact".to_string(),
        }
    } else if browser_failed {
        PackDashboardDimension {
            name: "screenshotCapture".to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: "browser command completed unsuccessfully or did not create a screenshot"
                .to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "screenshotCapture".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "screenshot capture was not attempted".to_string(),
        }
    };
    vec![
        surface_dimension,
        PackDashboardDimension {
            name: "executionScope".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: scope_details,
        },
        browser_daemon_dimension,
        screenshot_dimension,
    ]
}

pub(super) fn browse_qa_selected_target_url(
    candidates: &[BrowseQaTargetCandidate],
) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| {
            candidate.kind == "url" && browse_qa_is_local_browser_url(&candidate.value)
        })
        .map(|candidate| candidate.value.clone())
        .or_else(|| {
            candidates
                .iter()
                .find(|candidate| candidate.kind == "file")
                .map(|candidate| format!("file://{}", candidate.value))
        })
}

fn browse_qa_is_local_browser_url(url: &str) -> bool {
    url.starts_with("http://localhost")
        || url.starts_with("http://127.0.0.1")
        || url.starts_with("https://localhost")
        || url.starts_with("https://127.0.0.1")
}

pub(super) fn browse_qa_browser_binary(target: &DevflowCapabilityPackTarget) -> Option<String> {
    let mut candidates = vec![
        target
            .cwd_path
            .join(".codex")
            .join("skills")
            .join("gstack")
            .join("browse")
            .join("dist")
            .join("browse"),
        target
            .cwd_path
            .join(".agents")
            .join(".claude")
            .join("skills")
            .join("gstack")
            .join("browse")
            .join("dist")
            .join("browse"),
        PathBuf::from(DEFAULT_GSTACK_ROOT)
            .join("browse")
            .join("dist")
            .join("browse"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for marker in [".codex", ".claude", ".agents"] {
            candidates.push(
                home.join(marker)
                    .join("skills")
                    .join("gstack")
                    .join("browse")
                    .join("dist")
                    .join("browse"),
            );
        }
    }
    if let Some(path_binary) = browse_qa_path_binary("browse") {
        candidates.push(path_binary);
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.display().to_string())
}

fn browse_qa_path_binary(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn browse_qa_screenshot_path(target: &DevflowCapabilityPackTarget) -> PathBuf {
    target
        .cwd_path
        .join(".codex")
        .join("devflow")
        .join("artifacts")
        .join(format!("{}-browseQa-screenshot.png", target.run_id))
}

pub(super) fn browse_qa_command_allowlist(selected_target_url: Option<&str>) -> Vec<String> {
    let target = selected_target_url.unwrap_or("<selected-local-target-url>");
    vec![
        format!("browse goto {target}"),
        "browse screenshot <devflow-artifact-png>".to_string(),
    ]
}

pub(super) fn browse_qa_target_candidates(
    target: &DevflowCapabilityPackTarget,
) -> Vec<BrowseQaTargetCandidate> {
    let mut candidates: Vec<BrowseQaTargetCandidate> = Vec::new();
    {
        let mut push_candidate = |kind: &str, value: String, source: String, confidence: &str| {
            if !candidates
                .iter()
                .any(|candidate| candidate.kind == kind && candidate.value == value)
            {
                candidates.push(BrowseQaTargetCandidate {
                    kind: kind.to_string(),
                    value,
                    source,
                    confidence: confidence.to_string(),
                });
            }
        };

        for relative_path in [
            "index.html",
            "public/index.html",
            "app/page.tsx",
            "app/page.jsx",
            "src/App.tsx",
            "src/App.jsx",
        ] {
            let path = target.cwd_path.join(relative_path);
            if path.exists() {
                push_candidate(
                    "file",
                    path.display().to_string(),
                    relative_path.to_string(),
                    "medium",
                );
            }
        }

        let package_path = target.cwd_path.join("package.json");
        if let Ok(package_json) = fs::read_to_string(&package_path)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&package_json)
            && let Some(scripts) = value.get("scripts").and_then(serde_json::Value::as_object)
        {
            for script_name in ["dev", "start", "preview"] {
                let Some(script) = scripts.get(script_name).and_then(serde_json::Value::as_str)
                else {
                    continue;
                };
                push_candidate(
                    "command",
                    format!("npm run {script_name}"),
                    format!("package.json#scripts.{script_name}"),
                    "high",
                );

                let script_lower = script.to_ascii_lowercase();
                let port = ["--port", "-p", "localhost:", "127.0.0.1:"]
                    .into_iter()
                    .find_map(|marker| {
                        let index = script_lower.find(marker)?;
                        let after = &script_lower[index + marker.len()..];
                        let digits = after
                            .chars()
                            .skip_while(|ch| ch.is_whitespace() || *ch == '=')
                            .take_while(char::is_ascii_digit)
                            .collect::<String>();
                        (!digits.is_empty()).then_some(digits)
                    })
                    .or_else(|| {
                        if script_lower.contains("vite") {
                            Some("5173".to_string())
                        } else if script_lower.contains("next")
                            || script_lower.contains("react-scripts")
                            || script_lower.contains("webpack")
                        {
                            Some("3000".to_string())
                        } else {
                            None
                        }
                    });
                if let Some(port) = port {
                    push_candidate(
                        "url",
                        format!("http://127.0.0.1:{port}"),
                        format!("package.json#scripts.{script_name}"),
                        "medium",
                    );
                }
            }
        }

        for readme_path in ["README.md", "readme.md"] {
            let path = target.cwd_path.join(readme_path);
            if let Ok(contents) = fs::read_to_string(path) {
                for token in contents.split_whitespace() {
                    let url = token.trim_end_matches([',', '.', ')', ']', '"', '\'']);
                    if url.starts_with("http://localhost") || url.starts_with("http://127.0.0.1") {
                        push_candidate("url", url.to_string(), readme_path.to_string(), "low");
                    }
                }
            }
        }
    }
    candidates
}

fn gstack_health_dimensions(target: &DevflowCapabilityPackTarget) -> Vec<PackDashboardDimension> {
    let manifests = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "deno.json",
        "deno.jsonc",
    ]
    .into_iter()
    .filter(|manifest| target.cwd_path.join(manifest).exists())
    .collect::<Vec<_>>();
    let manifest_dimension = if manifests.is_empty() {
        PackDashboardDimension {
            name: "manifestInventory".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "no known project manifest detected".to_string(),
        }
    } else {
        PackDashboardDimension {
            name: "manifestInventory".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!("detected {}", manifests.join(", ")),
        }
    };
    let scope_details = if let Some(worktree_id) = &target.worktree_id {
        format!("running inside managed worktree {worktree_id}")
    } else {
        "running in project root; write-capable checks are limited".to_string()
    };
    vec![
        manifest_dimension,
        PackDashboardDimension {
            name: "executionScope".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: scope_details,
        },
    ]
}

fn gstack_health_commands(target: &DevflowCapabilityPackTarget) -> Vec<AllowedPackCommand> {
    let mut commands = Vec::new();
    if target.cwd_path.join(".git").exists() {
        commands.push(AllowedPackCommand {
            name: "gitStatus",
            argv: vec![
                "git".to_string(),
                "status".to_string(),
                "--short".to_string(),
                "--".to_string(),
                ".".to_string(),
                ":(exclude).codex".to_string(),
            ],
            timeout_secs: GSTACK_HEALTH_STATUS_TIMEOUT_SECS,
            expectation: AllowedCommandExpectation::EmptyStdout,
        });
        commands.push(AllowedPackCommand {
            name: "gitDiffCheck",
            argv: vec!["git".to_string(), "diff".to_string(), "--check".to_string()],
            timeout_secs: GSTACK_HEALTH_GIT_TIMEOUT_SECS,
            expectation: AllowedCommandExpectation::ExitZero,
        });
    }
    if target.cwd_path.join("Cargo.toml").exists() && target.worktree_id.is_some() {
        commands.push(AllowedPackCommand {
            name: "cargoCheck",
            argv: vec![
                "cargo".to_string(),
                "check".to_string(),
                "--workspace".to_string(),
                "--all-targets".to_string(),
            ],
            timeout_secs: GSTACK_HEALTH_CARGO_TIMEOUT_SECS,
            expectation: AllowedCommandExpectation::ExitZero,
        });
    }
    commands
}

fn command_allowlist() -> Vec<String> {
    vec![
        "git status --short -- . :(exclude).codex".to_string(),
        "git diff --check".to_string(),
        "cargo check --workspace --all-targets".to_string(),
    ]
}

pub(super) async fn run_allowed_pack_command_with_binary(
    cwd: &Path,
    command: &AllowedPackCommand,
    binary: &str,
) -> PackCommandReport {
    let mut executable_command = command.clone();
    if let Some(program) = executable_command.argv.first_mut() {
        *program = binary.to_string();
    }
    let mut report = run_allowed_pack_command(cwd, &executable_command).await;
    report.command = command.argv.join(" ");
    report.argv = command.argv.clone();
    report
}

async fn run_allowed_pack_command(cwd: &Path, command: &AllowedPackCommand) -> PackCommandReport {
    let started_at = Instant::now();
    let command_text = command.argv.join(" ");
    if command.argv.is_empty() {
        return PackCommandReport {
            name: command.name.to_string(),
            command: command_text,
            argv: Vec::new(),
            cwd: cwd.display().to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "command argv is empty".to_string(),
            exit_code: None,
            duration_ms: 0,
            timeout_secs: command.timeout_secs,
            stdout_tail: String::new(),
            stderr_tail: "command argv is empty".to_string(),
        };
    }

    let mut child = match Command::new(&command.argv[0])
        .args(command.argv.iter().skip(1))
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return PackCommandReport {
                name: command.name.to_string(),
                command: command_text,
                argv: command.argv.clone(),
                cwd: cwd.display().to_string(),
                status: "skipped".to_string(),
                score: None,
                details: format!("command not found: {}", command.argv[0]),
                exit_code: None,
                duration_ms: started_at.elapsed().as_millis() as i64,
                timeout_secs: command.timeout_secs,
                stdout_tail: String::new(),
                stderr_tail: format!("command not found: {}", command.argv[0]),
            };
        }
        Err(err) => {
            return PackCommandReport {
                name: command.name.to_string(),
                command: command_text,
                argv: command.argv.clone(),
                cwd: cwd.display().to_string(),
                status: "failed".to_string(),
                score: Some(0),
                details: format!("failed to spawn command: {err}"),
                exit_code: None,
                duration_ms: started_at.elapsed().as_millis() as i64,
                timeout_secs: command.timeout_secs,
                stdout_tail: String::new(),
                stderr_tail: format!("failed to spawn command: {err}"),
            };
        }
    };

    let Some(mut stdout) = child.stdout.take() else {
        return PackCommandReport {
            name: command.name.to_string(),
            command: command_text,
            argv: command.argv.clone(),
            cwd: cwd.display().to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: "missing stdout pipe".to_string(),
            exit_code: None,
            duration_ms: started_at.elapsed().as_millis() as i64,
            timeout_secs: command.timeout_secs,
            stdout_tail: String::new(),
            stderr_tail: "missing stdout pipe".to_string(),
        };
    };
    let Some(mut stderr) = child.stderr.take() else {
        return PackCommandReport {
            name: command.name.to_string(),
            command: command_text,
            argv: command.argv.clone(),
            cwd: cwd.display().to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: "missing stderr pipe".to_string(),
            exit_code: None,
            duration_ms: started_at.elapsed().as_millis() as i64,
            timeout_secs: command.timeout_secs,
            stdout_tail: String::new(),
            stderr_tail: "missing stderr pipe".to_string(),
        };
    };

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await.map(|_| buf)
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await.map(|_| buf)
    });

    let timeout = Duration::from_secs(command.timeout_secs);
    let wait_result = tokio::time::timeout(timeout, child.wait()).await;
    let (wait_failed, timed_out, exit_code) = match wait_result {
        Ok(Ok(status)) => (false, false, status.code()),
        Ok(Err(err)) => {
            tracing::warn!(error = %err, command = command_text, "pack runner failed waiting for command");
            (true, false, None)
        }
        Err(_) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            (false, true, None)
        }
    };
    let stdout = join_output_task(stdout_task, "stdout").await;
    let stderr = join_output_task(stderr_task, "stderr").await;
    let (status, score, details) = if timed_out {
        (
            "timed_out".to_string(),
            Some(0),
            format!("command timed out after {}s", command.timeout_secs),
        )
    } else if wait_failed {
        (
            "failed".to_string(),
            Some(0),
            "failed waiting for command".to_string(),
        )
    } else if exit_code != Some(0) {
        (
            "failed".to_string(),
            Some(0),
            format!("command exited with {exit_code:?}"),
        )
    } else {
        match command.expectation {
            AllowedCommandExpectation::ExitZero => (
                "completed".to_string(),
                Some(10),
                "command exited successfully".to_string(),
            ),
            AllowedCommandExpectation::EmptyStdout if stdout.trim().is_empty() => (
                "completed".to_string(),
                Some(10),
                "command produced no findings".to_string(),
            ),
            AllowedCommandExpectation::EmptyStdout => (
                "failed".to_string(),
                Some(0),
                "command output indicates findings".to_string(),
            ),
        }
    };

    PackCommandReport {
        name: command.name.to_string(),
        command: command_text,
        argv: command.argv.clone(),
        cwd: cwd.display().to_string(),
        status,
        score,
        details,
        exit_code,
        duration_ms: started_at.elapsed().as_millis() as i64,
        timeout_secs: command.timeout_secs,
        stdout_tail: tail_text(&stdout, COMMAND_OUTPUT_TAIL_LIMIT),
        stderr_tail: tail_text(&stderr, COMMAND_OUTPUT_TAIL_LIMIT),
    }
}

async fn join_output_task(
    task: tokio::task::JoinHandle<std::io::Result<Vec<u8>>>,
    name: &str,
) -> String {
    match task.await {
        Ok(Ok(output)) => String::from_utf8_lossy(&output).into_owned(),
        Ok(Err(err)) => format!("failed reading {name}: {err}"),
        Err(err) => format!("failed joining {name} reader: {err}"),
    }
}

fn tail_text(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    let tail = text
        .chars()
        .skip(char_count.saturating_sub(max_chars))
        .collect::<String>();
    format!("...[truncated]\n{tail}")
}

fn default_policy_packs() -> Vec<DevflowPolicyPack> {
    vec![DevflowPolicyPack {
        id: "superpowers-discipline".to_string(),
        name: "Superpowers Engineering Discipline".to_string(),
        source_path: Some(DEFAULT_SUPERPOWERS_ROOT.to_string()),
        status: path_pack_status(DEFAULT_SUPERPOWERS_ROOT),
        policies: vec![
            "writingPlans".to_string(),
            "worktreeIsolation".to_string(),
            "systematicDebugging".to_string(),
            "verificationBeforeCompletion".to_string(),
            "requestingCodeReview".to_string(),
            "finishBranch".to_string(),
        ],
        applies_to_risk_levels: vec![
            DevflowTaskRiskLevel::Low,
            DevflowTaskRiskLevel::Medium,
            DevflowTaskRiskLevel::High,
        ],
        diagnostics: vec![
            "Codex-only main path uses this pack as policy metadata before hard gates are enforced."
                .to_string(),
        ],
    }]
}

fn default_capability_packs() -> Vec<DevflowCapabilityPack> {
    vec![DevflowCapabilityPack {
        id: "gstack-engineering".to_string(),
        name: "gstack Engineering Capabilities".to_string(),
        source_path: Some(DEFAULT_GSTACK_ROOT.to_string()),
        status: path_pack_status(DEFAULT_GSTACK_ROOT),
        capabilities: vec![
            "health".to_string(),
            "browseQa".to_string(),
            "review".to_string(),
            "benchmark".to_string(),
            "canary".to_string(),
            "watchdogQueue".to_string(),
        ],
        diagnostics: vec![
            "health, browseQa, review, benchmark, canary, and watchdogQueue are wired through the Codex-owned pack runner; health, browseQa, benchmark, and canary can create watchdog alerts and failed quality gates, review records a static diff-intake artifact, watchdogQueue projects running/no-progress/timed-out/blocked queue summaries, and devflowWatchdog/reconcile provides the bounded recovery action for repairable Integrator conflicts."
                .to_string(),
        ],
    }]
}

fn find_policy_pack(id: &str) -> Result<DevflowPolicyPack, JSONRPCErrorError> {
    default_policy_packs()
        .into_iter()
        .find(|pack| pack.id == id)
        .ok_or_else(|| invalid_request(format!("unknown devflow policy pack id: {id}")))
}

fn find_capability_pack(id: &str) -> Result<DevflowCapabilityPack, JSONRPCErrorError> {
    default_capability_packs()
        .into_iter()
        .find(|pack| pack.id == id)
        .ok_or_else(|| invalid_request(format!("unknown devflow capability pack id: {id}")))
}

fn path_pack_status(path: &str) -> DevflowPackStatus {
    if Path::new(path).exists() {
        DevflowPackStatus::Available
    } else {
        DevflowPackStatus::Missing
    }
}
