use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowCapabilityPackRunStatus;
use codex_app_server_protocol::DevflowQualityGateKind;
use codex_app_server_protocol::DevflowWatchdogAlertSeverity;
use codex_app_server_protocol::DevflowWatchdogStatus;
use codex_app_server_protocol::JSONRPCErrorError;
use serde::Serialize;

use crate::error_code::internal_error;

use super::devflow_pack::AllowedCommandExpectation;
use super::devflow_pack::AllowedPackCommand;
use super::devflow_pack::PackCommandReport;
use super::devflow_pack::browse_qa_browser_binary;
use super::devflow_pack::browse_qa_command_allowlist;
use super::devflow_pack::browse_qa_selected_target_url;
use super::devflow_pack::browse_qa_target_candidates;
use super::devflow_pack::capability_pack_gate_command;
use super::devflow_pack::capability_pack_gate_status;
use super::devflow_pack::run_allowed_pack_command_with_binary;
use super::devflow_processor::DevflowCapabilityPackGateOutcome;
use super::devflow_processor::DevflowCapabilityPackTarget;
use super::devflow_processor::DevflowRequestProcessor;

const CANARY_RUNNER_SCHEMA_VERSION: u32 = 1;
const GSTACK_CANARY_GOTO_TIMEOUT_SECS: u64 = 20;
const GSTACK_CANARY_SCREENSHOT_TIMEOUT_SECS: u64 = 20;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanaryDashboardDimension {
    name: String,
    status: String,
    score: Option<u8>,
    details: String,
}

impl DevflowRequestProcessor {
    pub(super) async fn run_gstack_canary_capability(
        &self,
        target: &DevflowCapabilityPackTarget,
    ) -> Result<(DevflowCapabilityPackRunStatus, String, DevflowArtifact), JSONRPCErrorError> {
        let candidates = browse_qa_target_candidates(target);
        let selected_target_url = browse_qa_selected_target_url(&candidates);
        let browser_binary = browse_qa_browser_binary(target);
        let screenshot_path = canary_screenshot_path(target);
        let mut browser_commands = Vec::new();
        let mut setup_error = None;
        if let (Some(target_url), Some(binary)) = (&selected_target_url, &browser_binary) {
            if let Some(parent) = screenshot_path.parent()
                && let Err(err) = fs::create_dir_all(parent)
            {
                setup_error = Some(format!(
                    "failed to prepare canary screenshot artifact directory: {err}"
                ));
            }
            if setup_error.is_none() {
                let goto = AllowedPackCommand {
                    name: "canaryGoto",
                    argv: vec!["browse".to_string(), "goto".to_string(), target_url.clone()],
                    timeout_secs: GSTACK_CANARY_GOTO_TIMEOUT_SECS,
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
                        name: "canaryScreenshot",
                        argv: vec![
                            "browse".to_string(),
                            "screenshot".to_string(),
                            screenshot_path.display().to_string(),
                        ],
                        timeout_secs: GSTACK_CANARY_SCREENSHOT_TIMEOUT_SECS,
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
        let canary_attempted =
            selected_target_url.is_some() && browser_binary.is_some() && setup_error.is_none();
        let status = if setup_error.is_some()
            || browser_command_failed
            || (canary_attempted && !screenshot_captured)
        {
            DevflowCapabilityPackRunStatus::Failed
        } else if !canary_attempted {
            DevflowCapabilityPackRunStatus::Skipped
        } else {
            DevflowCapabilityPackRunStatus::Completed
        };
        let dimensions = canary_dimensions(
            target,
            candidates.len(),
            selected_target_url.as_deref(),
            browser_binary.as_deref(),
            &browser_commands,
            screenshot_captured,
            status == DevflowCapabilityPackRunStatus::Failed,
        );
        let summary = match status {
            DevflowCapabilityPackRunStatus::Completed => format!(
                "gstack canary completed: controlled browser probe captured {}",
                selected_target_url.as_deref().unwrap_or("selected target")
            ),
            DevflowCapabilityPackRunStatus::Failed => {
                "gstack canary failed: controlled browser probe breached canary thresholds"
                    .to_string()
            }
            DevflowCapabilityPackRunStatus::Skipped => {
                if selected_target_url.is_none() {
                    "gstack canary skipped: no local-safe target URL was selected".to_string()
                } else {
                    "gstack canary skipped: gstack browse binary was not found".to_string()
                }
            }
        };
        let screenshot_artifact_status = if screenshot_captured {
            "captured"
        } else if canary_attempted {
            "missing"
        } else {
            "not_started"
        };
        let timed_out = browser_commands
            .iter()
            .any(|report| report.status == "timed_out");
        let failed_gate_report = canary_failed_gate_report(
            status,
            &browser_commands,
            setup_error.as_deref(),
            screenshot_captured,
        );
        let report = serde_json::json!({
            "schemaVersion": CANARY_RUNNER_SCHEMA_VERSION,
            "runner": "codex-owned-pack-runner",
            "packId": "gstack-engineering",
            "capability": "canary",
            "canaryType": "controlled_local_browser_probe",
            "status": format!("{status:?}").to_ascii_lowercase(),
            "summary": summary.clone(),
            "policy": {
                "approval": "No deployment, package-manager scripts, arbitrary shell execution, or remote network targets. The runner only invokes a detected gstack browse binary with fixed goto and screenshot commands against a selected local-safe target.",
                "commandAllowlist": browse_qa_command_allowlist(selected_target_url.as_deref()),
                "targetSelection": {
                    "selectedUrl": selected_target_url.clone(),
                    "rules": [
                        "Prefer explicit http://127.0.0.1 or http://localhost URL candidates discovered from package scripts or README files.",
                        "Fall back to file:// URLs for static entrypoints such as index.html or public/index.html.",
                        "Reject remote URLs and never start package-manager scripts or application servers."
                    ],
                },
                "deploymentSource": {
                    "kind": canary_deployment_source_kind(selected_target_url.as_deref()),
                    "ownership": "Canary observes an already-running local URL or a static file URL. Codex does not deploy, start, stop, or mutate application runtime state in this MVP.",
                },
                "browserDaemon": {
                    "owner": "gstack-browse-cli",
                    "ownership": "Codex owns the canary invocation and report artifact. The gstack browse CLI owns daemon startup, connection reuse, and idle shutdown.",
                    "stateFile": target.cwd_path.join(".gstack").join("browse.json").display().to_string(),
                    "binary": browser_binary.clone(),
                },
                "thresholds": {
                    "gotoExitZeroWithinSeconds": GSTACK_CANARY_GOTO_TIMEOUT_SECS,
                    "screenshotExitZeroWithinSeconds": GSTACK_CANARY_SCREENSHOT_TIMEOUT_SECS,
                    "screenshotMustExist": true,
                    "screenshotMustBeNonEmpty": true,
                    "remoteUrlsAllowed": false,
                },
                "cleanup": {
                    "daemon": "left to gstack browse idle shutdown; Codex does not kill shared browser daemons from this capability.",
                    "files": "Screenshot files are stored under .codex/devflow/artifacts and are owned by Devflow cleanup with other run artifacts.",
                },
                "scope": {
                    "projectRoot": target.project_root,
                    "cwd": target.cwd_path.display().to_string(),
                    "worktreeId": target.worktree_id.clone(),
                    "writes": "Only Devflow report and screenshot artifact files are written by this runner.",
                },
                "artifactFormat": "application/json; schemaVersion=1; commands include argv, cwd, status, exitCode, durationMs, timeoutSecs, stdoutTail, and stderrTail.",
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
            internal_error(format!("failed to serialize gstack canary report: {err}"))
        })?;
        let artifact = self
            .write_capability_pack_artifact(target, "canary", &content, summary.clone())
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
                        capability_pack_gate_command("canary"),
                        None,
                        None,
                        format!("{summary}; see artifact {}", artifact.id),
                    )
                };
            self.record_capability_pack_quality_gate(
                target,
                DevflowCapabilityPackGateOutcome {
                    kind: DevflowQualityGateKind::GstackCanary,
                    capability: "canary",
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
}

fn canary_dimensions(
    target: &DevflowCapabilityPackTarget,
    candidate_count: usize,
    selected_target_url: Option<&str>,
    browser_binary: Option<&str>,
    reports: &[PackCommandReport],
    screenshot_captured: bool,
    canary_failed: bool,
) -> Vec<CanaryDashboardDimension> {
    let target_dimension = if selected_target_url.is_some() {
        CanaryDashboardDimension {
            name: "targetSelection".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!(
                "selected local-safe target from {candidate_count} discovered candidates"
            ),
        }
    } else if candidate_count == 0 {
        CanaryDashboardDimension {
            name: "targetSelection".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "no URL, package script, or static file candidates were detected".to_string(),
        }
    } else {
        CanaryDashboardDimension {
            name: "targetSelection".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "candidates existed, but none were local-safe canary targets".to_string(),
        }
    };
    let deployment_dimension = match selected_target_url {
        Some(url) if url.starts_with("file://") => CanaryDashboardDimension {
            name: "deploymentSource".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "static file URL selected; no deployment process is started".to_string(),
        },
        Some(url) => CanaryDashboardDimension {
            name: "deploymentSource".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!("observing already-running local target {url}"),
        },
        None => CanaryDashboardDimension {
            name: "deploymentSource".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "deployment source was not selected".to_string(),
        },
    };
    let browser_dimension = match (selected_target_url, browser_binary) {
        (None, _) => CanaryDashboardDimension {
            name: "browserDaemonReadiness".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "no selected target requires a browser daemon".to_string(),
        },
        (Some(_), None) => CanaryDashboardDimension {
            name: "browserDaemonReadiness".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "gstack browse binary was not found in project, user, or PATH locations"
                .to_string(),
        },
        (Some(_), Some(binary)) => CanaryDashboardDimension {
            name: "browserDaemonReadiness".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!("using gstack browse binary {binary}"),
        },
    };
    let failed_command = reports
        .iter()
        .find(|report| report.score.is_some_and(|score| score < 7));
    let probe_dimension = if let Some(report) = failed_command {
        CanaryDashboardDimension {
            name: "canaryProbe".to_string(),
            status: report.status.clone(),
            score: report.score,
            details: report.details.clone(),
        }
    } else if reports.is_empty() {
        CanaryDashboardDimension {
            name: "canaryProbe".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "canary browser commands were not attempted".to_string(),
        }
    } else {
        CanaryDashboardDimension {
            name: "canaryProbe".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "goto and screenshot probes met canary thresholds".to_string(),
        }
    };
    let screenshot_dimension = if screenshot_captured {
        CanaryDashboardDimension {
            name: "screenshotEvidence".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "captured non-empty PNG screenshot evidence".to_string(),
        }
    } else if canary_failed {
        CanaryDashboardDimension {
            name: "screenshotEvidence".to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: "screenshot evidence was required but missing or empty".to_string(),
        }
    } else {
        CanaryDashboardDimension {
            name: "screenshotEvidence".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "screenshot evidence was not attempted".to_string(),
        }
    };
    let scope_details = if let Some(worktree_id) = &target.worktree_id {
        format!("canary running inside managed worktree {worktree_id}")
    } else {
        "canary running in project root".to_string()
    };
    vec![
        target_dimension,
        deployment_dimension,
        browser_dimension,
        probe_dimension,
        screenshot_dimension,
        CanaryDashboardDimension {
            name: "executionScope".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: scope_details,
        },
    ]
}

fn canary_failed_gate_report(
    status: DevflowCapabilityPackRunStatus,
    reports: &[PackCommandReport],
    setup_error: Option<&str>,
    screenshot_captured: bool,
) -> Option<(String, Option<i32>, Option<i64>, String)> {
    reports
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
            let details = setup_error.map(str::to_string).unwrap_or_else(|| {
                if screenshot_captured {
                    "canary threshold failed".to_string()
                } else {
                    "canary screenshot evidence was not created".to_string()
                }
            });
            let command = if setup_error.is_some() {
                "canary setup".to_string()
            } else {
                "browse screenshot <devflow-canary-png>".to_string()
            };
            Some((command, None, None, details))
        })
}

fn canary_screenshot_path(target: &DevflowCapabilityPackTarget) -> PathBuf {
    target
        .cwd_path
        .join(".codex")
        .join("devflow")
        .join("artifacts")
        .join(format!("{}-canary-screenshot.png", target.run_id))
}

fn canary_deployment_source_kind(selected_target_url: Option<&str>) -> &'static str {
    match selected_target_url {
        Some(url) if url.starts_with("file://") => "static_file",
        Some(_) => "detected_local_url",
        None => "none",
    }
}
