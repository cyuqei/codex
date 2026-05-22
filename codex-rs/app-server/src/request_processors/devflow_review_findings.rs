use codex_app_server_protocol::DevflowArtifact;
use codex_app_server_protocol::DevflowArtifactKind;
use codex_app_server_protocol::ReviewFinding as ProtocolReviewFinding;
use codex_app_server_protocol::ReviewOutput as ProtocolReviewOutput;
use serde::Serialize;

const REVIEW_FINDING_STATE_SUMMARY_PREFIX: &str = "Review finding state:";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReviewFindingStatus {
    Open,
    Resolved,
    Waived,
    FollowUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReviewFindingSeverity {
    P0,
    P1,
    P2,
    P3,
    High,
    Medium,
    Low,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewFinding {
    id: String,
    status: ReviewFindingStatus,
    severity: ReviewFindingSeverity,
    title: String,
    file_path: Option<String>,
    line: Option<u32>,
    resolution: Option<String>,
    follow_up: Option<String>,
    evidence: String,
}

struct ParsedReviewFinding {
    status: ReviewFindingStatus,
    severity: ReviewFindingSeverity,
    title: String,
    file_path: Option<String>,
    line: Option<u32>,
    resolution: Option<String>,
    follow_up: Option<String>,
    evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReviewFindingState {
    schema_version: u8,
    status: String,
    open_count: usize,
    resolved_count: usize,
    waived_count: usize,
    follow_up_count: usize,
    findings: Vec<ReviewFinding>,
}

pub(super) fn build_review_finding_state(
    review: &str,
    review_output: Option<&ProtocolReviewOutput>,
) -> ReviewFindingState {
    let parsed_findings = review_output
        .map(parsed_review_output_findings)
        .filter(|findings| !findings.is_empty())
        .unwrap_or_else(|| {
            review
                .lines()
                .filter_map(parse_review_finding_line)
                .collect()
        });
    let findings = parsed_findings
        .into_iter()
        .enumerate()
        .map(|(index, parsed)| ReviewFinding {
            id: format!("review-finding-{}", index + 1),
            status: parsed.status,
            severity: parsed.severity,
            title: parsed.title,
            file_path: parsed.file_path,
            line: parsed.line,
            resolution: parsed.resolution,
            follow_up: parsed.follow_up,
            evidence: parsed.evidence,
        })
        .collect::<Vec<_>>();
    let open_count = findings
        .iter()
        .filter(|finding| finding.status == ReviewFindingStatus::Open)
        .count();
    let resolved_count = findings
        .iter()
        .filter(|finding| finding.status == ReviewFindingStatus::Resolved)
        .count();
    let waived_count = findings
        .iter()
        .filter(|finding| finding.status == ReviewFindingStatus::Waived)
        .count();
    let follow_up_count = findings
        .iter()
        .filter(|finding| finding.status == ReviewFindingStatus::FollowUp)
        .count();
    let status = if open_count > 0 {
        "open"
    } else if findings.is_empty() {
        "clear"
    } else {
        "all_addressed"
    }
    .to_string();

    ReviewFindingState {
        schema_version: 2,
        status,
        open_count,
        resolved_count,
        waived_count,
        follow_up_count,
        findings,
    }
}

fn parsed_review_output_findings(output: &ProtocolReviewOutput) -> Vec<ParsedReviewFinding> {
    output
        .findings
        .iter()
        .map(parse_protocol_review_finding)
        .collect()
}

fn parse_protocol_review_finding(finding: &ProtocolReviewFinding) -> ParsedReviewFinding {
    let title = normalize_finding_title(&finding.title);
    let evidence = if finding.body.trim().is_empty() {
        title.clone()
    } else {
        format!("{title}: {}", finding.body.trim())
    };
    ParsedReviewFinding {
        status: ReviewFindingStatus::Open,
        severity: priority_number_to_severity(finding.priority),
        title,
        file_path: Some(finding.code_location.absolute_file_path.clone()),
        line: Some(finding.code_location.line_range.start),
        resolution: None,
        follow_up: None,
        evidence,
    }
}

pub(super) fn render_review_artifact(review: &str, state: &ReviewFindingState) -> String {
    let state_json = serde_json::to_string_pretty(state)
        .unwrap_or_else(|err| format!("{{\"error\":\"failed to serialize state: {err}\"}}"));
    let mut output = String::new();
    output.push_str("# Review Finding State\n\n");
    output.push_str(&format!("- Status: {}\n", state.status));
    output.push_str(&format!("- Open findings: {}\n", state.open_count));
    output.push_str(&format!("- Resolved findings: {}\n", state.resolved_count));
    output.push_str(&format!("- Waived findings: {}\n", state.waived_count));
    output.push_str(&format!(
        "- Follow-up findings: {}\n\n",
        state.follow_up_count
    ));
    if state.findings.is_empty() {
        output.push_str("No structured findings were reported.\n\n");
    } else {
        output.push_str("## Structured Findings\n\n");
        output.push_str("| ID | Severity | Status | Location | Title | Resolution | Follow-up |\n");
        output.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
        for finding in &state.findings {
            output.push_str(&format!(
                "| {} | {:?} | {:?} | {} | {} | {} | {} |\n",
                escape_table_cell(&finding.id),
                finding.severity,
                finding.status,
                escape_table_cell(&finding_location(finding)),
                escape_table_cell(&finding.title),
                escape_table_cell(finding.resolution.as_deref().unwrap_or("")),
                escape_table_cell(finding.follow_up.as_deref().unwrap_or(""))
            ));
        }
        output.push('\n');
    }
    output.push_str("```json\n");
    output.push_str(&state_json);
    output.push_str("\n```\n\n");
    output.push_str("## Raw Review\n\n");
    output.push_str(review);
    if !review.ends_with('\n') {
        output.push('\n');
    }
    output
}

pub(super) fn review_artifact_summary(state: &ReviewFindingState) -> String {
    format!(
        "{REVIEW_FINDING_STATE_SUMMARY_PREFIX} status={}; open={}; resolved={}; waived={}; followUp={}",
        state.status,
        state.open_count,
        state.resolved_count,
        state.waived_count,
        state.follow_up_count
    )
}

pub(super) fn review_artifact_all_findings_addressed(artifact: &DevflowArtifact) -> bool {
    if artifact.kind != DevflowArtifactKind::ReviewReport {
        return false;
    }
    let Some(status) = review_artifact_status(artifact) else {
        return false;
    };
    matches!(status, "clear" | "all_addressed")
}

pub(super) fn review_artifact_has_finding_state(artifact: &DevflowArtifact) -> bool {
    artifact.kind == DevflowArtifactKind::ReviewReport && review_artifact_status(artifact).is_some()
}

fn review_artifact_status(artifact: &DevflowArtifact) -> Option<&str> {
    artifact
        .summary
        .strip_prefix(REVIEW_FINDING_STATE_SUMMARY_PREFIX)?
        .trim()
        .strip_prefix("status=")?
        .split(';')
        .next()
}

fn parse_review_finding_line(line: &str) -> Option<ParsedReviewFinding> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("::code-comment") {
        return parse_code_comment_finding(trimmed);
    }

    let lower = trimmed.to_ascii_lowercase();
    let looks_like_list_item = trimmed.starts_with('-')
        || trimmed.starts_with('*')
        || trimmed.starts_with("::code-comment")
        || starts_with_numbered_item(trimmed);
    let looks_like_finding =
        lower.starts_with("finding:") || lower.starts_with("issue:") || lower.starts_with("bug:");
    if !looks_like_list_item && !looks_like_finding {
        return None;
    }

    let status = parse_finding_status(&lower)?;
    let severity = parse_finding_severity(trimmed).unwrap_or(ReviewFindingSeverity::Unknown);
    let (location_file, location_line) = parse_inline_location(trimmed);
    let file_path = metadata_value(trimmed, &["file", "path"]).or(location_file);
    let line = metadata_number(trimmed, &["line", "start"]).or(location_line);
    let resolution = metadata_value(trimmed, &["resolution", "resolved_by", "resolvedBy"]);
    let follow_up = metadata_value(trimmed, &["follow_up", "followUp", "follow-up"]);
    let title = normalize_finding_title(trimmed);
    Some(ParsedReviewFinding {
        status,
        severity,
        title,
        file_path,
        line,
        resolution,
        follow_up,
        evidence: trimmed.to_string(),
    })
}

fn parse_code_comment_finding(line: &str) -> Option<ParsedReviewFinding> {
    let title = directive_attr(line, "title").unwrap_or_else(|| "Code review finding".to_string());
    let body = directive_attr(line, "body");
    let file_path = directive_attr(line, "file");
    let line_number = directive_attr(line, "start").and_then(|value| value.parse::<u32>().ok());
    let status =
        parse_finding_status(&line.to_ascii_lowercase()).unwrap_or(ReviewFindingStatus::Open);
    let severity = directive_attr(line, "priority")
        .and_then(|value| priority_to_severity(&value))
        .or_else(|| parse_finding_severity(&title))
        .unwrap_or(ReviewFindingSeverity::Unknown);
    let evidence = match body.as_deref() {
        Some(body) if !body.is_empty() => format!("{title}: {body}"),
        _ => line.to_string(),
    };
    Some(ParsedReviewFinding {
        status,
        severity,
        title: normalize_finding_title(&title),
        file_path,
        line: line_number,
        resolution: directive_attr(line, "resolution"),
        follow_up: directive_attr(line, "followUp").or_else(|| directive_attr(line, "follow_up")),
        evidence,
    })
}

fn parse_finding_status(lower: &str) -> Option<ReviewFindingStatus> {
    if lower.contains("[resolved]")
        || lower.contains("status: resolved")
        || lower.contains("status=resolved")
        || lower.contains("state: resolved")
    {
        Some(ReviewFindingStatus::Resolved)
    } else if lower.contains("[waived]")
        || lower.contains("status: waived")
        || lower.contains("status=waived")
        || lower.contains("state: waived")
    {
        Some(ReviewFindingStatus::Waived)
    } else if lower.contains("[follow-up]")
        || lower.contains("[follow_up]")
        || lower.contains("[followup]")
        || lower.contains("status: follow")
        || lower.contains("status=follow")
        || lower.contains("state: follow")
    {
        Some(ReviewFindingStatus::FollowUp)
    } else if lower.contains("[open]")
        || lower.contains("[unresolved]")
        || lower.contains("[p0]")
        || lower.contains("[p1]")
        || lower.contains("[p2]")
        || lower.contains("[p3]")
        || lower.contains("finding")
        || lower.contains("issue")
        || lower.contains("bug")
        || lower.contains("blocking")
    {
        Some(ReviewFindingStatus::Open)
    } else {
        None
    }
}

fn starts_with_numbered_item(value: &str) -> bool {
    let Some((prefix, _)) = value.split_once('.') else {
        return false;
    };
    !prefix.is_empty() && prefix.chars().all(|ch| ch.is_ascii_digit())
}

fn normalize_finding_title(value: &str) -> String {
    let value = value.split('|').next().unwrap_or(value);
    let value = value
        .trim_start_matches(|ch: char| {
            ch == '-' || ch == '*' || ch == ':' || ch.is_ascii_digit() || ch == '.'
        })
        .trim();
    strip_leading_review_tags(value)
        .trim_start_matches([':', '-'])
        .trim_start_matches("finding:")
        .trim_start_matches("Finding:")
        .trim_start_matches("issue:")
        .trim_start_matches("Issue:")
        .trim_start_matches("bug:")
        .trim_start_matches("Bug:")
        .trim()
        .to_string()
}

fn strip_leading_review_tags(mut value: &str) -> &str {
    loop {
        let Some(rest) = value.strip_prefix('[') else {
            return value;
        };
        let Some((tag, after_tag)) = rest.split_once(']') else {
            return value;
        };
        let tag = tag.to_ascii_lowercase();
        if is_review_tag(&tag) {
            value = after_tag.trim_start();
        } else {
            return value;
        }
    }
}

fn is_review_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p0" | "p1"
            | "p2"
            | "p3"
            | "high"
            | "medium"
            | "low"
            | "open"
            | "unresolved"
            | "resolved"
            | "waived"
            | "follow-up"
            | "follow_up"
            | "followup"
    )
}

fn parse_finding_severity(value: &str) -> Option<ReviewFindingSeverity> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("[p0]")
        || lower.contains("severity: p0")
        || lower.contains("severity=p0")
        || lower.contains("priority: p0")
        || lower.contains("priority=p0")
    {
        Some(ReviewFindingSeverity::P0)
    } else if lower.contains("[p1]")
        || lower.contains("severity: p1")
        || lower.contains("severity=p1")
        || lower.contains("priority: p1")
        || lower.contains("priority=p1")
    {
        Some(ReviewFindingSeverity::P1)
    } else if lower.contains("[p2]")
        || lower.contains("severity: p2")
        || lower.contains("severity=p2")
        || lower.contains("priority: p2")
        || lower.contains("priority=p2")
    {
        Some(ReviewFindingSeverity::P2)
    } else if lower.contains("[p3]")
        || lower.contains("severity: p3")
        || lower.contains("severity=p3")
        || lower.contains("priority: p3")
        || lower.contains("priority=p3")
    {
        Some(ReviewFindingSeverity::P3)
    } else if lower.contains("[high]")
        || lower.contains("severity: high")
        || lower.contains("severity=high")
    {
        Some(ReviewFindingSeverity::High)
    } else if lower.contains("[medium]")
        || lower.contains("severity: medium")
        || lower.contains("severity=medium")
    {
        Some(ReviewFindingSeverity::Medium)
    } else if lower.contains("[low]")
        || lower.contains("severity: low")
        || lower.contains("severity=low")
    {
        Some(ReviewFindingSeverity::Low)
    } else {
        None
    }
}

fn priority_to_severity(value: &str) -> Option<ReviewFindingSeverity> {
    match value.trim() {
        "0" => Some(ReviewFindingSeverity::P0),
        "1" => Some(ReviewFindingSeverity::P1),
        "2" => Some(ReviewFindingSeverity::P2),
        "3" => Some(ReviewFindingSeverity::P3),
        _ => parse_finding_severity(value),
    }
}

fn priority_number_to_severity(value: i32) -> ReviewFindingSeverity {
    match value {
        0 => ReviewFindingSeverity::P0,
        1 => ReviewFindingSeverity::P1,
        2 => ReviewFindingSeverity::P2,
        3 => ReviewFindingSeverity::P3,
        _ => ReviewFindingSeverity::Unknown,
    }
}

fn metadata_value(value: &str, keys: &[&str]) -> Option<String> {
    for segment in value.split('|').skip(1) {
        let segment = segment.trim();
        for key in keys {
            if let Some(value) = segment
                .strip_prefix(&format!("{key}="))
                .or_else(|| segment.strip_prefix(&format!("{key}:")))
            {
                let value = value.trim().trim_matches('"').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn metadata_number(value: &str, keys: &[&str]) -> Option<u32> {
    metadata_value(value, keys).and_then(|value| {
        value
            .trim_start_matches("L")
            .trim_start_matches("line ")
            .parse::<u32>()
            .ok()
    })
}

fn parse_inline_location(value: &str) -> (Option<String>, Option<u32>) {
    for token in value.split_whitespace() {
        let token = token
            .trim_matches(|ch: char| ch == ',' || ch == ';' || ch == ')' || ch == '(' || ch == '`');
        let Some((path, line)) = token.rsplit_once(':') else {
            continue;
        };
        if !path.contains('/') && !path.contains('.') {
            continue;
        }
        let Ok(line) = line.parse::<u32>() else {
            continue;
        };
        return (Some(path.to_string()), Some(line));
    }
    (None, None)
}

fn directive_attr(value: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=");
    let start = value.find(&needle)? + needle.len();
    let rest = &value[start..];
    if let Some(rest) = rest.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '}')
        .unwrap_or(rest.len());
    Some(rest[..end].trim_matches('"').to_string())
}

fn finding_location(finding: &ReviewFinding) -> String {
    match (finding.file_path.as_deref(), finding.line) {
        (Some(file), Some(line)) => format!("{file}:{line}"),
        (Some(file), None) => file.to_string(),
        (None, Some(line)) => format!("line {line}"),
        (None, None) => String::new(),
    }
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_state_is_clear_when_review_has_no_finding_lines() {
        let state = build_review_finding_state("Review: no blocking issues.", None);

        assert_eq!(state.status, "clear");
        assert_eq!(state.open_count, 0);
        assert!(state.findings.is_empty());
    }

    #[test]
    fn review_state_tracks_open_and_addressed_findings() {
        let state = build_review_finding_state(
            "- [P1] Fix the config leak | file=app-server/src/config.rs | line=42 | status=open | resolution=rotate leaked token | followUp=add regression coverage\n- [resolved] Added focused coverage | file=app-server/tests/config.rs | line=12 | resolution=covered by test\n- [follow-up] Track non-blocking UX polish | followUp=create design ticket\n- [waived] Accepted legacy behavior | resolution=documented migration risk\n",
            None,
        );

        assert_eq!(state.status, "open");
        assert_eq!(state.schema_version, 2);
        assert_eq!(state.open_count, 1);
        assert_eq!(state.resolved_count, 1);
        assert_eq!(state.follow_up_count, 1);
        assert_eq!(state.waived_count, 1);
        assert_eq!(state.findings[0].severity, ReviewFindingSeverity::P1);
        assert_eq!(
            state.findings[0].file_path.as_deref(),
            Some("app-server/src/config.rs")
        );
        assert_eq!(state.findings[0].line, Some(42));
        assert_eq!(
            state.findings[0].resolution.as_deref(),
            Some("rotate leaked token")
        );
        assert_eq!(
            state.findings[0].follow_up.as_deref(),
            Some("add regression coverage")
        );
    }

    #[test]
    fn review_state_extracts_code_comment_location_and_priority() {
        let state = build_review_finding_state(
            r#"::code-comment{title="[P2] Off-by-one" body="Loop iterates past the end when length is 0." file="/tmp/foo.ts" start=10 end=11 priority=2}"#,
            None,
        );

        assert_eq!(state.status, "open");
        assert_eq!(state.open_count, 1);
        assert_eq!(state.findings[0].severity, ReviewFindingSeverity::P2);
        assert_eq!(state.findings[0].title, "Off-by-one");
        assert_eq!(state.findings[0].file_path.as_deref(), Some("/tmp/foo.ts"));
        assert_eq!(state.findings[0].line, Some(10));
    }

    #[test]
    fn review_state_prefers_protocol_review_output_findings() {
        let review_output = ProtocolReviewOutput {
            findings: vec![ProtocolReviewFinding {
                title: "[P3] Markdown fallback should be ignored".to_string(),
                body: "The reviewer supplied a native structured location.".to_string(),
                confidence_score: 0.91,
                priority: 1,
                code_location: codex_app_server_protocol::ReviewCodeLocation {
                    absolute_file_path: "/tmp/src/app.rs".to_string(),
                    line_range: codex_app_server_protocol::ReviewLineRange { start: 7, end: 9 },
                },
            }],
            overall_correctness: "patch is incorrect".to_string(),
            overall_explanation: "native review output was available".to_string(),
            overall_confidence_score: 0.8,
        };

        let state = build_review_finding_state(
            "- [P0] Text parser fallback | file=/tmp/fallback.rs | line=99 | status=open",
            Some(&review_output),
        );

        assert_eq!(state.status, "open");
        assert_eq!(state.open_count, 1);
        assert_eq!(state.findings[0].severity, ReviewFindingSeverity::P1);
        assert_eq!(
            state.findings[0].file_path.as_deref(),
            Some("/tmp/src/app.rs")
        );
        assert_eq!(state.findings[0].line, Some(7));
        assert_eq!(
            state.findings[0].title,
            "Markdown fallback should be ignored"
        );
        assert!(
            state.findings[0]
                .evidence
                .contains("native structured location")
        );
    }
}
