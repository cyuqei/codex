#!/usr/bin/env bash
set -euo pipefail

TASK_ID="${1:?usage: verify-task.sh TASK_ID [command ...]}"
shift || true
ROOT="${CODEX_ROOT:-/Users/yuqei/codex}"
REPORT_DIR="$ROOT/automation/reports/test-runs"
mkdir -p "$REPORT_DIR"
STAMP="$(date +%Y%m%d-%H%M%S)"
REPORT="$REPORT_DIR/$STAMP-$TASK_ID.log"

{
  echo "task: $TASK_ID"
  echo "timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "root: $ROOT"
  echo
  if [ "$#" -eq 0 ]; then
    echo "NO_COMMAND_PROVIDED"
    exit 2
  fi
  echo "command: $*"
  echo "--- output ---"
  "$@"
} >"$REPORT" 2>&1

status=$?
echo "report: $REPORT"
echo "status: $status"
exit "$status"
