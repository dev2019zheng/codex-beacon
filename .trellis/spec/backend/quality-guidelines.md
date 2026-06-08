# Backend Quality Guidelines

## Overview

Rust backend/core changes must preserve the core-shell boundary and pass workspace checks before commit.

## Required Patterns

- Run `cargo fmt --all -- --check` before reporting completion.
- Run `cargo test --workspace` for core or Tauri command changes.
- Keep `beacon-core` free of Tauri dependencies.
- Derive `Serialize` and `Deserialize` on structs and enums that cross Tauri IPC.
- Keep alert-level mapping in the core crate so all shells behave consistently.

## Forbidden Patterns

- Do not let frontend themes read `~/.codex`, SQLite, JSONL, or hook output directly.
- Do not put simulated-state-only assumptions into names that will later represent real Codex state.
- Do not expose write/control commands for Codex tasks in the read-only HUD path.

## Testing Requirements

- New status mapping behavior requires unit tests in `beacon-core`.
- Tauri shell build must be checked with `pnpm --filter @codex-beacon/desktop tauri:build --bundles app` when command signatures or config change.

## Code Review Checklist

- Does the payload still serialize to the frontend contract?
- Is the core reusable by non-Tauri shells?
- Are generated artifacts such as `target/`, `dist/`, and TypeScript build info ignored?

## Scenario: Codex Hook Event Source

### 1. Scope / Trigger

- Trigger: any change to Codex hook recording, event-log storage, or hook-derived status mapping.
- Scope: `.codex/hooks/*.py`, `.codex/hooks.json`, `core/beacon-core`, and Tauri source adapters.

### 2. Signatures

- Hook command: `python3 -X utf8 .codex/hooks/beacon-record-event.py --event <HookName>`.
- Optional hook override: `--log-path <path>`.
- Environment override: `CODEX_BEACON_EVENT_LOG=/path/to/events.jsonl`.
- Core parser: `parse_hook_events_jsonl(input: &str) -> Vec<CodexHookEvent>`.
- Core mapper: `snapshot_from_hook_events(events: &[CodexHookEvent], now: DateTime<Utc>) -> BeaconSnapshot`.

### 3. Contracts

- Default event log path is `~/.codex-beacon/events.jsonl`.
- Hook JSONL rows use camelCase fields: `schemaVersion`, `timestamp`, `event`, `summary`, optional `sessionId`, optional `cwd`, optional `toolName`.
- Hook rows must never include prompt text, tool arguments, command output, or model responses.
- `BeaconSnapshot.source` must be one of `codex_app`, `hooks`, `manual`, or `simulation`.
- Tauri desktop defaults to Codex Desktop local state when available, falls back to hook-derived snapshots, and browser preview remains simulation-only unless manual demo controls are used.

### 4. Validation & Error Matrix

- Missing or unreadable Codex Desktop DB -> fall back to hook state.
- No active Desktop threads -> return a `codex_app`/idle snapshot.
- Missing or unreadable hook log after Desktop fallback fails -> return a hooks/unknown snapshot.
- Empty hook log after Desktop fallback fails -> return a hooks/unknown snapshot.
- Malformed JSONL row -> skip that row and parse the rest.
- Hook recorder write failure -> exit 0 after stderr note so Codex is not blocked.
- Latest event older than ten minutes -> return hooks/idle.
- Approval or permission event -> map to `waiting_approval` with strong alert.

### 5. Good/Base/Bad Cases

- Good: `PermissionRequest` within ten minutes maps to one waiting task and `AlertLevel::Strong`.
- Base: no event log maps to an idle hook snapshot.
- Bad: storing full user prompts or shell output in the event log.

### 6. Tests Required

- Hook recorder self-test asserts sanitized output and no prompt persistence.
- Core unit tests assert JSONL parsing, waiting mapping, and stale-event idle fallback.
- Tauri build is required when command behavior or IPC payload fields change.

### 7. Wrong vs Correct

#### Wrong

```text
React component reads ~/.codex-beacon/events.jsonl directly and derives status.
```

#### Correct

```text
Hook recorder writes sanitized JSONL -> Rust core derives BeaconSnapshot -> Tauri command exposes the snapshot.
```

## Scenario: Codex Desktop Live Refresh Source

### 1. Scope / Trigger

- Trigger: any change to Codex Desktop local-state polling, Tauri snapshot commands, or refresh semantics.
- Goal: manual refresh and polling must observe current Codex Desktop SQLite/WAL state without restarting the HUD.

### 2. Signatures

- Tauri command: `get_beacon_snapshot(refreshNonce?: number) -> BeaconSnapshot`.
- State DB path: `CODEX_BEACON_CODEX_STATE_DB` override or latest `~/.codex/state_*.sqlite`.
- Logs DB path: `CODEX_BEACON_CODEX_LOGS_DB` override or latest `~/.codex/logs_*.sqlite`.
- Source helper: `recent_codex_activity(logs_path, now) -> HashMap<thread_id, CodexThreadActivity>`.
- Merge helper: `codex_app_tasks_from_state(state_path, active_threads, allow_recent_state_fallback, now) -> Vec<CodexAppTask>`.

### 3. Contracts

- Open SQLite read-only on every snapshot call; do not keep a long-lived connection for the live source.
- The frontend adapter must pass a changing `refreshNonce` so each manual refresh has a distinct IPC payload.
- Active activity is derived from recent semantic logs containing `run_sampling_request` or `session_task.turn`.
- Exit markers are `target = 'codex_core::session::handlers'` plus `feedback_log_body LIKE '%}: Agent loop exited'`.
- A thread is active only when recent log activity is inside the active window and the thread state update is newer than any later exit marker.
- Task `updated_at` must prefer the latest log activity time over `threads.updated_at_ms`, so the HUD's relative time changes as Codex continues working.
- If the logs DB is missing, fall back to recent `threads.updated_at_ms`; if logs exist but a thread has no active activity, do not treat state recency alone as active.

### 4. Validation & Error Matrix

- Missing state DB -> fall back to hook snapshot.
- Missing logs DB -> allow short state-table fallback.
- Existing logs DB plus no recent activity -> return a Codex Desktop idle snapshot.
- Agent exit after the latest state update -> do not show the thread as running.
- Tail logs after exit -> do not revive a stopped task unless thread state is updated after the exit.
- New turn after exit -> show running again and update the task timestamp.

### 5. Good/Base/Bad Cases

- Good: clicking refresh after Codex writes new logs changes `BeaconSnapshot.updatedAt` and task row relative times.
- Base: an active thread with recent `session_task.turn` logs appears as one running Codex task.
- Bad: a HUD refresh reads only `threads.updated_at_ms` or uses a cached SQLite connection and keeps showing the startup snapshot.

### 6. Tests Required

- Rust unit test where latest log activity makes a stale state-table timestamp display as fresh.
- Rust unit test where `Agent loop exited` suppresses a thread even when older turn logs exist.
- Rust unit test where a new turn after an exit reactivates the thread.
- Frontend typecheck/build after changing IPC args or adapter signatures.
- Tauri app bundle build after changing command signatures.

### 7. Wrong vs Correct

#### Wrong

```rust
let updated_at = millis_to_datetime(updated_at_ms, now);
```

#### Correct

```rust
let display_updated_at_ms = activity
    .map(|activity| activity.last_activity_ms.max(updated_at_ms))
    .unwrap_or(updated_at_ms);
let updated_at = millis_to_datetime(display_updated_at_ms, now);
```
