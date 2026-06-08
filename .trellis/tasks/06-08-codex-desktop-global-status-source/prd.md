# Codex Desktop Global Status Source

## Problem

Codex Beacon currently reads project hook events only. That misses Codex Desktop sidebar activity, where multiple user threads can be running at the same time across different workspaces.

## Requirements

- Prefer a read-only Codex Desktop source before falling back to project hooks.
- Read thread metadata from `~/.codex/state_5.sqlite` and recent activity from `~/.codex/logs_2.sqlite`.
- Show multiple active user threads in one `BeaconSnapshot`.
- Keep the core crate free of Tauri and SQLite dependencies.
- Keep React components consuming `BeaconSnapshot` only.
- Allow local test overrides for Codex DB paths.

## Acceptance

- A running Codex Desktop thread appears as a `running` task with title, workspace detail, and updated time.
- Multiple active Codex Desktop user threads increment `activeCount`.
- No active Desktop threads returns an idle `codex_app` snapshot instead of a misleading hook-only unknown state.
- Missing or unreadable Desktop DBs gracefully fall back to the existing hook source.
- Core tests cover the new `codex_app` snapshot behavior.
