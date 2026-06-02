use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowCapabilityPackRunStatus;
use codex_app_server_protocol::DevflowQualityGateKind;
use codex_app_server_protocol::DevflowWatchdogAlertSeverity;
use codex_app_server_protocol::DevflowWatchdogStatus;
use codex_app_server_protocol::JSONRPCErrorError;
use serde::Serialize;

use crate::error_code::internal_error;

use super::devflow_pack::capability_pack_gate_command;
use super::devflow_pack::capability_pack_gate_status;
use super::devflow_processor::DevflowCapabilityPackGateOutcome;
use super::devflow_processor::DevflowCapabilityPackTarget;
use super::devflow_processor::DevflowRequestProcessor;

const BENCHMARK_RUNNER_SCHEMA_VERSION: u32 = 1;
const BENCHMARK_MAX_ASSETS: usize = 256;
const BENCHMARK_MAX_DEPTH: usize = 4;
const BENCHMARK_DEFAULT_ASSET_BUDGET_BYTES: u64 = 512 * 1024;
const BENCHMARK_LARGE_ASSET_BUDGET_BYTES: u64 = 1024 * 1024;
const BENCHMARK_TOTAL_ASSET_BUDGET_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkAssetReport {
    path: String,
    kind: String,
    bytes: u64,
    budget_bytes: u64,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkDashboardDimension {
    name: String,
    status: String,
    score: Option<u8>,
    details: String,
}

impl DevflowRequestProcessor {
    pub(super) async fn run_gstack_benchmark_capability(
        &self,
        target: &DevflowCapabilityPackTarget,
    ) -> Result<(DevflowCapabilityPackRunStatus, String, DevflowArtifact), JSONRPCErrorError> {
        let assets = benchmark_asset_reports(target);
        let total_bytes = assets.iter().map(|asset| asset.bytes).sum::<u64>();
        let oversized_assets = assets
            .iter()
            .filter(|asset| asset.status == "failed")
            .count();
        let total_budget_failed =
            !assets.is_empty() && total_bytes > BENCHMARK_TOTAL_ASSET_BUDGET_BYTES;
        let violation_count = oversized_assets + usize::from(total_budget_failed);
        let status = if assets.is_empty() {
            DevflowCapabilityPackRunStatus::Skipped
        } else if violation_count > 0 {
            DevflowCapabilityPackRunStatus::Failed
        } else {
            DevflowCapabilityPackRunStatus::Completed
        };
        let summary = match status {
            DevflowCapabilityPackRunStatus::Completed => format!(
                "gstack benchmark completed: {} static assets within local budgets",
                assets.len()
            ),
            DevflowCapabilityPackRunStatus::Failed => {
                format!("gstack benchmark failed: {violation_count} static asset budget violations")
            }
            DevflowCapabilityPackRunStatus::Skipped => {
                "gstack benchmark skipped: no static browser assets were detected".to_string()
            }
        };
        let dimensions = benchmark_dimensions(
            target,
            &assets,
            total_bytes,
            total_budget_failed,
            violation_count,
        );
        let largest_assets = assets.iter().take(10).cloned().collect::<Vec<_>>();
        let report = serde_json::json!({
            "schemaVersion": BENCHMARK_RUNNER_SCHEMA_VERSION,
            "runner": "codex-owned-pack-runner",
            "packId": "gstack-engineering",
            "capability": "benchmark",
            "benchmarkType": "static_asset_budget",
            "status": format!("{status:?}").to_ascii_lowercase(),
            "summary": summary.clone(),
            "policy": {
                "approval": "No arbitrary shell execution, browser automation, package-manager scripts, or network calls. The runner only reads local static asset metadata and writes this Devflow report artifact.",
                "commandAllowlist": [],
                "budgetBytes": {
                    "defaultAsset": BENCHMARK_DEFAULT_ASSET_BUDGET_BYTES,
                    "imageOrFontAsset": BENCHMARK_LARGE_ASSET_BUDGET_BYTES,
                    "totalStaticAssets": BENCHMARK_TOTAL_ASSET_BUDGET_BYTES,
                },
                "assetDiscovery": {
                    "roots": ["index.html", "public/index.html", "public", "dist", "build", "out"],
                    "maxAssets": BENCHMARK_MAX_ASSETS,
                    "maxDepth": BENCHMARK_MAX_DEPTH,
                    "symlinks": "not followed",
                },
                "scope": {
                    "projectRoot": target.project_root,
                    "cwd": target.cwd_path.display().to_string(),
                    "worktreeId": target.worktree_id.clone(),
                    "writes": "Only this Devflow artifact file is written by the benchmark runner.",
                },
                "artifactFormat": "application/json; schemaVersion=1; assets contain path, kind, bytes, budgetBytes, and status.",
            },
            "dimensions": dimensions,
            "totals": {
                "assetCount": assets.len(),
                "totalBytes": total_bytes,
                "budgetBytes": BENCHMARK_TOTAL_ASSET_BUDGET_BYTES,
                "violationCount": violation_count,
            },
            "largestAssets": largest_assets,
            "assets": assets,
        });
        let content = serde_json::to_string_pretty(&report).map_err(|err| {
            internal_error(format!(
                "failed to serialize gstack benchmark report: {err}"
            ))
        })?;
        let artifact = self
            .write_capability_pack_artifact(target, "benchmark", &content, summary.clone())
            .await?;
        if status == DevflowCapabilityPackRunStatus::Failed {
            self.record_watchdog_alert(
                DevflowWatchdogStatus::NoProgress,
                DevflowWatchdogAlertSeverity::Warning,
                Some(target.project_root.clone()),
                Some(target.task_id.clone()),
                Some(target.run_id.clone()),
                format!("{summary}; see artifact {}", artifact.id),
            )
            .await;
        }
        if let Some(gate_status) = capability_pack_gate_status(status) {
            self.record_capability_pack_quality_gate(
                target,
                DevflowCapabilityPackGateOutcome {
                    kind: DevflowQualityGateKind::GstackBenchmark,
                    capability: "benchmark",
                    status: gate_status,
                    command: if status == DevflowCapabilityPackRunStatus::Failed {
                        "static asset budget check".to_string()
                    } else {
                        capability_pack_gate_command("benchmark")
                    },
                    exit_code: None,
                    duration_ms: None,
                    summary: format!("{summary}; see artifact {}", artifact.id),
                },
                &artifact,
            )
            .await;
        }
        Ok((status, summary, artifact))
    }
}

fn benchmark_dimensions(
    target: &DevflowCapabilityPackTarget,
    assets: &[BenchmarkAssetReport],
    total_bytes: u64,
    total_budget_failed: bool,
    violation_count: usize,
) -> Vec<BenchmarkDashboardDimension> {
    let asset_inventory = if assets.is_empty() {
        BenchmarkDashboardDimension {
            name: "assetInventory".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "no known static web assets were detected".to_string(),
        }
    } else {
        BenchmarkDashboardDimension {
            name: "assetInventory".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!("detected {} static web assets", assets.len()),
        }
    };
    let largest_asset_budget = if violation_count == 0 {
        BenchmarkDashboardDimension {
            name: "largestAssetBudget".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: "all static assets are within per-asset budgets".to_string(),
        }
    } else {
        BenchmarkDashboardDimension {
            name: "largestAssetBudget".to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: format!("{violation_count} budget checks failed"),
        }
    };
    let total_asset_budget = if assets.is_empty() {
        BenchmarkDashboardDimension {
            name: "totalAssetBudget".to_string(),
            status: "skipped".to_string(),
            score: None,
            details: "total budget was not evaluated because no assets were detected".to_string(),
        }
    } else if total_budget_failed {
        BenchmarkDashboardDimension {
            name: "totalAssetBudget".to_string(),
            status: "failed".to_string(),
            score: Some(0),
            details: format!(
                "total static asset size {total_bytes} bytes exceeds {BENCHMARK_TOTAL_ASSET_BUDGET_BYTES} bytes"
            ),
        }
    } else {
        BenchmarkDashboardDimension {
            name: "totalAssetBudget".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: format!(
                "total static asset size {total_bytes} bytes is within {BENCHMARK_TOTAL_ASSET_BUDGET_BYTES} bytes"
            ),
        }
    };
    let scope_details = if let Some(worktree_id) = &target.worktree_id {
        format!("static benchmark running inside managed worktree {worktree_id}")
    } else {
        "static benchmark running in project root".to_string()
    };
    vec![
        asset_inventory,
        largest_asset_budget,
        total_asset_budget,
        BenchmarkDashboardDimension {
            name: "executionScope".to_string(),
            status: "completed".to_string(),
            score: Some(10),
            details: scope_details,
        },
    ]
}

fn benchmark_asset_reports(target: &DevflowCapabilityPackTarget) -> Vec<BenchmarkAssetReport> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for relative_path in ["index.html", "public/index.html"] {
        push_benchmark_asset_path(&target.cwd_path.join(relative_path), &mut paths, &mut seen);
    }
    for relative_root in ["public", "dist", "build", "out"] {
        collect_benchmark_asset_paths(
            &target.cwd_path.join(relative_root),
            &mut paths,
            &mut seen,
            0,
        );
    }
    paths.sort();
    let mut assets = paths
        .into_iter()
        .take(BENCHMARK_MAX_ASSETS)
        .filter_map(|path| benchmark_asset_report(&target.cwd_path, &path))
        .collect::<Vec<_>>();
    assets.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.path.cmp(&right.path))
    });
    assets
}

fn collect_benchmark_asset_paths(
    root: &Path,
    paths: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    depth: usize,
) {
    if depth > BENCHMARK_MAX_DEPTH || paths.len() >= BENCHMARK_MAX_ASSETS {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if paths.len() >= BENCHMARK_MAX_ASSETS {
            return;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_benchmark_asset_paths(&path, paths, seen, depth + 1);
        } else if file_type.is_file() && benchmark_is_static_asset(&path) {
            push_benchmark_asset_path(&path, paths, seen);
        }
    }
}

fn push_benchmark_asset_path(path: &Path, paths: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    if path.is_file() && seen.insert(path.to_path_buf()) {
        paths.push(path.to_path_buf());
    }
}

fn benchmark_asset_report(root: &Path, path: &Path) -> Option<BenchmarkAssetReport> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let kind = benchmark_asset_kind(path);
    let budget_bytes = benchmark_asset_budget_bytes(kind);
    let bytes = metadata.len();
    Some(BenchmarkAssetReport {
        path: path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string(),
        kind: kind.to_string(),
        bytes,
        budget_bytes,
        status: if bytes <= budget_bytes {
            "completed".to_string()
        } else {
            "failed".to_string()
        },
    })
}

fn benchmark_is_static_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "html"
                    | "css"
                    | "js"
                    | "mjs"
                    | "cjs"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "webp"
                    | "svg"
                    | "ico"
                    | "woff"
                    | "woff2"
                    | "ttf"
                    | "otf"
                    | "json"
            )
        })
        .unwrap_or(false)
}

fn benchmark_asset_kind(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html") => "html",
        Some("css") => "style",
        Some("js" | "mjs" | "cjs") => "script",
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico") => "image",
        Some("woff" | "woff2" | "ttf" | "otf") => "font",
        Some("json") => "data",
        _ => "other",
    }
}

fn benchmark_asset_budget_bytes(kind: &str) -> u64 {
    match kind {
        "image" | "font" => BENCHMARK_LARGE_ASSET_BUDGET_BYTES,
        _ => BENCHMARK_DEFAULT_ASSET_BUDGET_BYTES,
    }
}
