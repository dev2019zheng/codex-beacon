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
