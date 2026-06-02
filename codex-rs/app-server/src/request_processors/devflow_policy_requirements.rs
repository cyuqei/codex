use codex_app_server_protocol::DevflowQualityGateKind;
use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowTaskKind;
use codex_app_server_protocol::DevflowTaskRiskLevel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DevflowRequiredQualityGate {
    pub(crate) kind: DevflowQualityGateKind,
    pub(crate) artifact_name: &'static str,
    pub(crate) display_name: &'static str,
}

pub(crate) fn policy_pack_required_artifacts(
    task: Option<&DevflowTask>,
    risk_level: Option<DevflowTaskRiskLevel>,
    task_requires_root_cause: impl FnOnce(&DevflowTask) -> bool,
) -> Vec<String> {
    let mut artifacts = Vec::new();
    let mut push_artifact = |artifact: &str| {
        let artifact = artifact.to_string();
        if !artifacts.contains(&artifact) {
            artifacts.push(artifact);
        }
    };

    if matches!(
        risk_level,
        Some(DevflowTaskRiskLevel::Medium | DevflowTaskRiskLevel::High)
    ) {
        push_artifact("plan");
    }

    match task.map(|task| task.kind) {
        Some(DevflowTaskKind::Implementation) => {
            push_artifact("worktree");
            push_artifact("diff");
            push_artifact("verification");
            if matches!(risk_level, Some(DevflowTaskRiskLevel::High)) {
                push_artifact("integrationTest");
            }
            if task.is_some_and(task_is_snapshot_sensitive) {
                push_artifact("snapshot");
            }
            push_artifact("review");
        }
        Some(DevflowTaskKind::Review) => {
            push_artifact("review");
        }
        Some(DevflowTaskKind::Report | DevflowTaskKind::Automation) => {
            push_artifact("report");
        }
        Some(DevflowTaskKind::Diagnostic) => {
            push_artifact("rootCause");
        }
        None => {
            push_artifact("verification");
            push_artifact("review");
        }
    }

    if task.is_some_and(task_requires_root_cause) {
        push_artifact("rootCause");
    }

    artifacts
}

pub(crate) fn required_quality_gates(task: &DevflowTask) -> Vec<DevflowRequiredQualityGate> {
    if task.kind != DevflowTaskKind::Implementation {
        return Vec::new();
    }

    let mut gates = Vec::new();
    if task.risk_level == DevflowTaskRiskLevel::High {
        gates.push(DevflowRequiredQualityGate {
            kind: DevflowQualityGateKind::IntegrationTest,
            artifact_name: "integrationTest",
            display_name: "integration test",
        });
    }
    if task_is_snapshot_sensitive(task) {
        gates.push(DevflowRequiredQualityGate {
            kind: DevflowQualityGateKind::Snapshot,
            artifact_name: "snapshot",
            display_name: "snapshot",
        });
    }
    gates
}

fn task_is_snapshot_sensitive(task: &DevflowTask) -> bool {
    let text = format!(
        "{} {} {}",
        task.title,
        task.objective,
        task.trigger_source.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();
    [
        "snapshot",
        "screenshot",
        "visual",
        "ui",
        "tui",
        "frontend",
        "front-end",
        "react",
        "component",
        "ratatui",
        "warp",
        "screen",
        "dialog",
        "popup",
        "form",
        "settings",
    ]
    .iter()
    .any(|signal| {
        if signal.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            text.split(|ch: char| !ch.is_ascii_alphanumeric())
                .any(|token| token == *signal)
        } else {
            text.contains(signal)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::DevflowTaskStatus;
    use pretty_assertions::assert_eq;

    #[test]
    fn high_risk_implementation_requires_integration_test_artifact() {
        let task = task("Change release pipeline", "Update backend release gating.");

        assert_eq!(
            policy_pack_required_artifacts(Some(&task), Some(DevflowTaskRiskLevel::High), |_| {
                false
            }),
            vec![
                "plan".to_string(),
                "worktree".to_string(),
                "diff".to_string(),
                "verification".to_string(),
                "integrationTest".to_string(),
                "review".to_string(),
            ]
        );
        assert_eq!(
            required_quality_gates(&task),
            vec![DevflowRequiredQualityGate {
                kind: DevflowQualityGateKind::IntegrationTest,
                artifact_name: "integrationTest",
                display_name: "integration test",
            }]
        );
    }

    #[test]
    fn snapshot_sensitive_implementation_requires_snapshot_artifact() {
        let mut task = task(
            "Polish provider form UI",
            "Update the React settings dialog.",
        );
        task.risk_level = DevflowTaskRiskLevel::Medium;

        assert!(
            policy_pack_required_artifacts(Some(&task), Some(DevflowTaskRiskLevel::Medium), |_| {
                false
            })
            .contains(&"snapshot".to_string())
        );
        assert_eq!(
            required_quality_gates(&task),
            vec![DevflowRequiredQualityGate {
                kind: DevflowQualityGateKind::Snapshot,
                artifact_name: "snapshot",
                display_name: "snapshot",
            }]
        );
    }

    #[test]
    fn signal_matching_does_not_treat_format_as_form() {
        let mut task = task("Run format cleanup", "Fix cargo fmt output only.");
        task.risk_level = DevflowTaskRiskLevel::Medium;

        assert!(
            !policy_pack_required_artifacts(
                Some(&task),
                Some(DevflowTaskRiskLevel::Medium),
                |_| false
            )
            .contains(&"snapshot".to_string())
        );
        assert!(required_quality_gates(&task).is_empty());
    }

    fn task(title: &str, objective: &str) -> DevflowTask {
        DevflowTask {
            id: "task-1".to_string(),
            project_id: "/tmp/project".to_string(),
            title: title.to_string(),
            objective: objective.to_string(),
            trigger_source: None,
            status: DevflowTaskStatus::Planned,
            kind: DevflowTaskKind::Implementation,
            risk_level: DevflowTaskRiskLevel::High,
            dependencies: Vec::new(),
            assigned_agent_id: Some("codex-main".to_string()),
            worktree_id: None,
            context_pack_id: None,
            run_ids: Vec::new(),
            artifact_ids: Vec::new(),
            created_at: 1_700_000_000,
            updated_at: 1_700_000_000,
        }
    }
}
