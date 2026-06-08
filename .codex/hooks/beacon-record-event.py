#!/usr/bin/env python3
"""Record sanitized Codex hook events for Codex Beacon.

The recorder is intentionally best-effort: normal hook execution must never
block Codex if Beacon cannot write its local event queue.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, Iterable, Optional


SCHEMA_VERSION = 1
MAX_EVENTS = 500


def default_log_path() -> Path:
    override = os.environ.get("CODEX_BEACON_EVENT_LOG")
    if override:
        return Path(override).expanduser()

    home = Path(os.environ.get("HOME", "~")).expanduser()
    return home / ".codex-beacon" / "events.jsonl"


def read_hook_payload() -> Dict[str, Any]:
    raw = sys.stdin.read()
    if not raw.strip():
        return {}

    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        return {}

    return payload if isinstance(payload, dict) else {}


def first_string(payload: Dict[str, Any], keys: Iterable[str]) -> Optional[str]:
    for key in keys:
        value = payload.get(key)
        if isinstance(value, str) and value.strip():
            return sanitize(value, max_len=240)
    return None


def tool_name_from(payload: Dict[str, Any]) -> Optional[str]:
    direct = first_string(payload, ("toolName", "tool_name", "name"))
    if direct:
        return direct

    tool = payload.get("tool")
    if isinstance(tool, dict):
        return first_string(tool, ("name", "toolName", "tool_name"))

    return None


def event_name_from(payload: Dict[str, Any], explicit_event: Optional[str]) -> str:
    if explicit_event and explicit_event.strip():
        return sanitize(explicit_event, max_len=80)

    event = first_string(
        payload,
        ("hookEventName", "hook_event_name", "eventName", "event_name", "event"),
    )
    return event or "Unknown"


def sanitize(value: str, max_len: int) -> str:
    clean = "".join(ch if ch.isprintable() else " " for ch in value)
    clean = " ".join(clean.split())
    if len(clean) <= max_len:
        return clean
    return f"{clean[: max_len - 3]}..."


def summary_for(event: str, tool_name: Optional[str]) -> str:
    normalized = event.replace("_", "").replace("-", "").lower()

    if "approval" in normalized or "permission" in normalized:
        return "Waiting for approval"
    if "userpromptsubmit" in normalized:
        return "Prompt submitted"
    if "sessionstart" in normalized:
        return "Session started"
    if "pretooluse" in normalized:
        return f"Starting tool: {tool_name}" if tool_name else "Starting tool"
    if "posttooluse" in normalized:
        return f"Finished tool: {tool_name}" if tool_name else "Finished tool"
    if "stop" in normalized:
        return "Assistant turn finished"
    if "compact" in normalized:
        return "Context compacted"

    return event


def build_record(
    payload: Dict[str, Any],
    event: str,
    now: Optional[dt.datetime] = None,
) -> Dict[str, Any]:
    timestamp = now or dt.datetime.now(dt.timezone.utc)
    tool_name = tool_name_from(payload)

    record: Dict[str, Any] = {
        "schemaVersion": SCHEMA_VERSION,
        "timestamp": timestamp.isoformat().replace("+00:00", "Z"),
        "event": event,
        "summary": summary_for(event, tool_name),
    }

    session_id = first_string(
        payload,
        ("sessionId", "session_id", "conversationId", "conversation_id", "threadId", "thread_id"),
    )
    cwd = first_string(payload, ("cwd", "projectDir", "project_dir", "workspace", "workspaceRoot"))

    if session_id:
        record["sessionId"] = session_id
    if cwd:
        record["cwd"] = cwd
    if tool_name:
        record["toolName"] = tool_name

    return record


def append_record(path: Path, record: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")))
        handle.write("\n")

    prune_log(path)


def prune_log(path: Path) -> None:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return

    if len(lines) <= MAX_EVENTS:
        return

    path.write_text("\n".join(lines[-MAX_EVENTS:]) + "\n", encoding="utf-8")


def record(payload: Dict[str, Any], explicit_event: Optional[str], path: Path) -> Dict[str, Any]:
    event = event_name_from(payload, explicit_event)
    hook_record = build_record(payload, event)
    append_record(path, hook_record)
    return hook_record


def self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp_dir:
        log_path = Path(tmp_dir) / "events.jsonl"
        payload = {
            "session_id": "session-123",
            "cwd": "/tmp/codex-beacon",
            "tool_name": "Bash",
            "prompt": "do not persist this prompt",
        }
        record(payload, "PreToolUse", log_path)

        lines = log_path.read_text(encoding="utf-8").splitlines()
        assert len(lines) == 1
        row = json.loads(lines[0])
        assert row["event"] == "PreToolUse"
        assert row["toolName"] == "Bash"
        assert row["summary"] == "Starting tool: Bash"
        assert "prompt" not in row

    print("Codex Beacon hook recorder self-test passed")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--event", help="Hook event name supplied by hooks.json")
    parser.add_argument("--log-path", help="Override event log path")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        return self_test()

    log_path = Path(args.log_path).expanduser() if args.log_path else default_log_path()

    try:
        record(read_hook_payload(), args.event, log_path)
    except Exception as exc:  # pragma: no cover - hook safety net
        print(f"Codex Beacon hook recorder skipped: {exc.__class__.__name__}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
