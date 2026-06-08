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
