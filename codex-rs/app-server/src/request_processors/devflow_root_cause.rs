use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowArtifactKind;
use codex_app_server_protocol::DevflowTask;
use codex_app_server_protocol::DevflowTaskKind;
use serde::Serialize;

const ROOT_CAUSE_STATE_SUMMARY_PREFIX: &str = "Root cause state:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RootCauseState {
    schema_version: u8,
    status: String,
    root_cause: Option<String>,
}

pub(super) fn task_requires_root_cause(task: &DevflowTask) -> bool {
    if task.kind == DevflowTaskKind::Diagnostic {
        return true;
    }
    if task.kind != DevflowTaskKind::Implementation {
        return false;
    }

    let task_text = format!(
        "{}\n{}\n{}",
        task.title,
        task.objective,
        task.trigger_source.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();
    [
        "bug",
        "regression",
        "crash",
        "panic",
        "fault",
        "failure",
        "failing",
        "broken",
        "error",
        "报错",
        "错误",
        "故障",
        "缺陷",
        "崩溃",
        "失败",
        "根因",
        "诊断",
    ]
    .iter()
    .any(|keyword| task_text.contains(keyword))
}

pub(super) fn build_root_cause_state(report: &str) -> RootCauseState {
    let root_cause = find_root_cause(report)
        .map(|value| value.trim().to_string())
        .filter(|value| !is_missing_root_cause(value));
    let status = if root_cause.is_some() {
        "identified"
    } else {
        "missing"
    }
    .to_string();

    RootCauseState {
        schema_version: 1,
        status,
        root_cause,
    }
}

pub(super) fn render_root_cause_artifact(report: &str, state: &RootCauseState) -> String {
    let state_json = serde_json::to_string_pretty(state)
        .unwrap_or_else(|err| format!("{{\"error\":\"failed to serialize state: {err}\"}}"));
    let mut output = String::new();
    output.push_str("# Root Cause State\n\n");
    output.push_str(&format!("- Status: {}\n", state.status));
    output.push_str(&format!(
        "- Root cause: {}\n\n",
        state.root_cause.as_deref().unwrap_or("missing")
    ));
    output.push_str("```json\n");
    output.push_str(&state_json);
    output.push_str("\n```\n\n");
    output.push_str("## Raw Diagnostic Report\n\n");
    output.push_str(report);
    if !report.ends_with('\n') {
        output.push('\n');
    }
    output
}

pub(super) fn root_cause_artifact_summary(state: &RootCauseState) -> String {
    let root_cause = state
        .root_cause
        .as_deref()
        .map(truncate_root_cause_summary)
        .unwrap_or_else(|| "missing".to_string());
    format!(
        "{ROOT_CAUSE_STATE_SUMMARY_PREFIX} status={}; rootCause={root_cause}",
        state.status
    )
}

pub(super) fn root_cause_artifact_is_accepted(artifact: &DevflowArtifact) -> bool {
    if !is_root_cause_artifact_kind(artifact.kind) {
        return false;
    }
    matches!(root_cause_artifact_status(artifact), Some("identified"))
}

pub(super) fn root_cause_artifact_has_state(artifact: &DevflowArtifact) -> bool {
    is_root_cause_artifact_kind(artifact.kind) && root_cause_artifact_status(artifact).is_some()
}

fn root_cause_artifact_status(artifact: &DevflowArtifact) -> Option<&str> {
    artifact
        .summary
        .strip_prefix(ROOT_CAUSE_STATE_SUMMARY_PREFIX)?
        .trim()
        .strip_prefix("status=")?
        .split(';')
        .next()
}

fn is_root_cause_artifact_kind(kind: DevflowArtifactKind) -> bool {
    matches!(
        kind,
        DevflowArtifactKind::Report | DevflowArtifactKind::RunSummary
    )
}

fn find_root_cause(report: &str) -> Option<&str> {
    let lines = report.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        if let Some(value) = root_cause_from_label(line) {
            return Some(value);
        }
        if is_root_cause_heading(line) {
            for next in lines.iter().skip(index + 1) {
                let trimmed = next.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with('#') {
                    break;
                }
                return Some(trimmed);
            }
        }
    }
    None
}

fn root_cause_from_label(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_start_matches(['-', '*']).trim();
    let (label, value) = trimmed.split_once(':')?;
    let normalized = label
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "root cause"
            | "root-cause"
            | "root_cause"
            | "rootcause"
            | "probable root cause"
            | "根因"
            | "根本原因"
            | "原因"
            | "故障原因"
    ) {
        let value = value.trim();
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}

fn is_root_cause_heading(line: &str) -> bool {
    let normalized = line
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "root cause"
            | "root-cause"
            | "root_cause"
            | "rootcause"
            | "probable root cause"
            | "根因"
            | "根本原因"
            | "原因"
            | "故障原因"
    )
}

fn is_missing_root_cause(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    [
        "unknown",
        "unidentified",
        "not identified",
        "not found",
        "none",
        "n/a",
        "na",
        "tbd",
        "todo",
        "undetermined",
        "unclear",
        "未知",
        "不明",
        "待定",
        "未定位",
        "未确定",
        "无法确定",
    ]
    .iter()
    .any(|missing| normalized == *missing || normalized.starts_with(&format!("{missing} ")))
}

fn truncate_root_cause_summary(value: &str) -> String {
    const LIMIT: usize = 160;
    let mut output = value.chars().take(LIMIT).collect::<String>();
    if value.chars().count() > LIMIT {
        output.push_str("...");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::DevflowTaskRiskLevel;
    use codex_app_server_protocol::DevflowTaskStatus;
    use pretty_assertions::assert_eq;

    #[test]
    fn root_cause_state_identifies_labeled_root_cause() {
        let state = build_root_cause_state(
            "Evidence: requests hit /v1/v1/messages.\nRoot cause: base_url already included /v1.\n",
        );

        assert_eq!(
            state,
            RootCauseState {
                schema_version: 1,
                status: "identified".to_string(),
                root_cause: Some("base_url already included /v1.".to_string()),
            }
        );
    }

    #[test]
    fn root_cause_state_treats_unknown_as_missing() {
        let state = build_root_cause_state("Root cause: unknown - logs were incomplete.\n");

        assert_eq!(
            state,
            RootCauseState {
                schema_version: 1,
                status: "missing".to_string(),
                root_cause: None,
            }
        );
    }

    #[test]
    fn implementation_bug_tasks_require_root_cause() {
        assert!(task_requires_root_cause(&task(
            DevflowTaskKind::Implementation,
            "Fix login regression",
            "Repair the broken provider flow."
        )));
        assert!(!task_requires_root_cause(&task(
            DevflowTaskKind::Implementation,
            "Add provider picker",
            "Implement the planned UI polish."
        )));
    }

    fn task(kind: DevflowTaskKind, title: &str, objective: &str) -> DevflowTask {
        DevflowTask {
            id: "task-1".to_string(),
            project_id: "/tmp/project".to_string(),
            title: title.to_string(),
            objective: objective.to_string(),
            trigger_source: None,
            status: DevflowTaskStatus::Planned,
            kind,
            risk_level: DevflowTaskRiskLevel::Low,
            dependencies: Vec::new(),
            assigned_agent_id: None,
            worktree_id: None,
            context_pack_id: None,
            run_ids: Vec::new(),
            artifact_ids: Vec::new(),
            created_at: 0,
            updated_at: 0,
        }
    }
}
