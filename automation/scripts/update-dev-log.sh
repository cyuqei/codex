#!/usr/bin/env bash
set -euo pipefail

ROOT="${CODEX_ROOT:-/Users/yuqei/codex}"
LOG="$ROOT/docs/yuqei-codex-dev-log.md"
TASK_ID="${1:?usage: update-dev-log.sh TASK_ID RESULT [NEXT]}"
RESULT="${2:-}"
NEXT="${3:-}"

{
  echo
  echo "## $(date +%Y-%m-%d) $TASK_ID"
  echo
  echo "- Task: $TASK_ID"
  echo "- Files changed: See git diff for this task."
  echo "- Tests run: See automation/reports/test-runs if present."
  echo "- Result: $RESULT"
  echo "- Decisions: None."
  echo "- Blockers: None recorded."
  echo "- Next: ${NEXT:-Continue with next pending queue task.}"
} >> "$LOG"
