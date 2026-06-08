# Fix Live Beacon Refresh

## Problem

Codex Beacon shows the Codex Desktop task state captured when the app opens, but manual refresh and the one-minute polling loop do not reliably reflect current Codex Desktop task changes.

## Goals

- Manual refresh must re-read current Codex Desktop local state every time.
- The one-minute polling loop must use the same fresh read path.
- The Rust source must read active SQLite/WAL data from Codex Desktop instead of acting like a startup snapshot.
- The HUD remains read-only and keeps browser fallback behavior.

## Non-Goals

- Do not add manual/demo status controls.
- Do not control or mutate Codex tasks.
- Do not redesign the HUD UI in this task.

## Acceptance Criteria

- Repeated calls to the Tauri snapshot command produce fresh `updatedAt` values.
- Codex Desktop state changes written to SQLite/WAL can be observed by refresh without restarting the app.
- Existing Codex active/exit semantics remain covered by tests.
- Frontend typecheck/build and Rust workspace tests pass.
