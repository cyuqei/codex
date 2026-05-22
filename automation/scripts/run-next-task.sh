#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-/Users/yuqei/codex}"
QUEUE_DIR="$ROOT/automation/queue"

if [ ! -d "$QUEUE_DIR" ]; then
  echo "NO_QUEUE"
  exit 0
fi

python3 - "$QUEUE_DIR" <<'PY'
import pathlib
import re
import sys

queue_dir = pathlib.Path(sys.argv[1])
for path in sorted(queue_dir.glob("*.yaml")):
    text = path.read_text()
    blocks = re.split(r"\n(?=- id: )", text)
    for block in blocks:
        if re.search(r"^\s*status:\s*pending\s*$", block, re.M):
            task_id = re.search(r"^\s*-?\s*id:\s*(.+?)\s*$", block, re.M)
            title = re.search(r"^\s*title:\s*(.+?)\s*$", block, re.M)
            phase = re.search(r"^\s*phase:\s*(.+?)\s*$", block, re.M)
            print(f"file: {path}")
            print(f"id: {task_id.group(1) if task_id else 'unknown'}")
            print(f"title: {title.group(1) if title else 'unknown'}")
            print(f"phase: {phase.group(1) if phase else 'unknown'}")
            print("---")
            print(block.strip())
            raise SystemExit(0)
print("NO_PENDING_TASKS")
PY
