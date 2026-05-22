use std::path::Path;
use std::path::PathBuf;

use chrono::Utc;
use codex_app_server_protocol::DevflowApproval;
use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowQualityGate;
use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowWatchdogAlert;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;
use uuid::Uuid;

use super::devflow_quality_gate::GateCommand;

const DEVFLOW_STORE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DevflowStoreSnapshot {
    pub(crate) schema_version: u32,
    pub(crate) saved_at: i64,
    pub(crate) tasks: Vec<DevflowTask>,
    pub(crate) runs: Vec<PersistedDevflowRunRecord>,
    pub(crate) quality_gates: Vec<PersistedDevflowQualityGateRecord>,
    #[serde(default)]
    pub(crate) approvals: Vec<DevflowApproval>,
    pub(crate) artifacts: Vec<DevflowArtifact>,
    pub(crate) watchdog_alerts: Vec<DevflowWatchdogAlert>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersistedDevflowRunRecord {
    pub(crate) run: codex_app_server_protocol::DevflowRun,
    pub(crate) project_root: String,
    pub(crate) diff_artifact_id: Option<String>,
    pub(crate) summary_artifact_id: Option<String>,
    pub(crate) output_archive_artifact_id: Option<String>,
    pub(crate) review_artifact_id: Option<String>,
    pub(crate) quality_gate_id: Option<String>,
    pub(crate) review_requested: bool,
    pub(crate) review_completed: bool,
    pub(crate) auto_repair_attempt: u32,
    #[serde(default)]
    pub(crate) auto_integrator_merge: bool,
    pub(crate) requested_stop: Option<PersistedDevflowRequestedStop>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersistedDevflowRequestedStop {
    Pause,
    Cancel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersistedDevflowQualityGateRecord {
    pub(crate) gate: DevflowQualityGate,
    pub(crate) command: GateCommand,
}

impl DevflowStoreSnapshot {
    pub(crate) fn new(
        tasks: Vec<DevflowTask>,
        runs: Vec<PersistedDevflowRunRecord>,
        quality_gates: Vec<PersistedDevflowQualityGateRecord>,
        approvals: Vec<DevflowApproval>,
        artifacts: Vec<DevflowArtifact>,
        watchdog_alerts: Vec<DevflowWatchdogAlert>,
    ) -> Self {
        Self {
            schema_version: DEVFLOW_STORE_SCHEMA_VERSION,
            saved_at: Utc::now().timestamp(),
            tasks,
            runs,
            quality_gates,
            approvals,
            artifacts,
            watchdog_alerts,
        }
    }
}

pub(crate) fn devflow_store_snapshot_path(codex_home: &Path) -> PathBuf {
    codex_home.join("devflow").join("store").join("state.json")
}

pub(crate) fn load_devflow_store_snapshot(
    codex_home: &Path,
) -> Result<Option<DevflowStoreSnapshot>, String> {
    let path = devflow_store_snapshot_path(codex_home);
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "failed to read Devflow store snapshot {}: {err}",
                path.display()
            ));
        }
    };
    let snapshot: DevflowStoreSnapshot = serde_json::from_str(&contents).map_err(|err| {
        format!(
            "failed to parse Devflow store snapshot {}: {err}",
            path.display()
        )
    })?;
    if snapshot.schema_version != DEVFLOW_STORE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported Devflow store snapshot schema version {} in {}",
            snapshot.schema_version,
            path.display()
        ));
    }
    Ok(Some(snapshot))
}

pub(crate) async fn save_devflow_store_snapshot(
    codex_home: &Path,
    snapshot: &DevflowStoreSnapshot,
) -> Result<(), String> {
    let path = devflow_store_snapshot_path(codex_home);
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Devflow store snapshot path has no parent: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent)
        .await
        .map_err(|err| format!("failed to create Devflow store directory: {err}"))?;
    let temp_path = parent.join(format!("state-{}.json.tmp", Uuid::new_v4()));
    let contents = serde_json::to_string_pretty(snapshot)
        .map_err(|err| format!("failed to serialize Devflow store snapshot: {err}"))?;
    fs::write(&temp_path, contents)
        .await
        .map_err(|err| format!("failed to write Devflow store snapshot temp file: {err}"))?;
    fs::rename(&temp_path, &path)
        .await
        .map_err(|err| format!("failed to replace Devflow store snapshot: {err}"))?;
    Ok(())
}
