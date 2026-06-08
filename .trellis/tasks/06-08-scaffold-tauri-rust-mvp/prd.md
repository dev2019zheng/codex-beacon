# Scaffold Tauri Rust MVP

## Background

Codex Beacon has architecture and release-channel docs. The next step is a runnable MVP with a Tauri shell, Rust core, built-in theme entries, and simulated Codex task state.

## Goals

- Create a Rust workspace with `core/beacon-core`.
- Create a Tauri v2 desktop app under `apps/desktop-tauri`.
- Implement a read-only Beacon status model with `running`, `completed`, `waiting_approval`, `waiting_input`, `failed`, `idle`, and `unknown`.
- Provide a transparent, always-on-top, frameless small HUD window that can switch between card and capsule modes.
- Use a simulated status source first: poll every 60 seconds and provide development controls to move between states manually.
- Include theme entries for `minimal-card`, `neon-hud`, and `electric-mascot`; the MVP defaults to `minimal-card`.
- Add basic developer scripts and README instructions for running the desktop app.

## Non-Goals

- Do not integrate real Codex hooks, SQLite, JSONL, or app-server in this task.
- Do not generate final mascot artwork in this task.
- Do not create GitHub Actions workflows yet; wait until Tauri build is stable.
- Do not implement signing, notarization, or DMG publishing in this task.

## Acceptance Criteria

- `cargo test --workspace` passes.
- `pnpm --filter @codex-beacon/desktop typecheck` passes.
- `pnpm --filter @codex-beacon/desktop build` passes.
- `pnpm --filter @codex-beacon/desktop tauri:build --bundles app` is attempted; if platform dependencies block it, record the reason.
- The frontend has both card and capsule modes, with stable dimensions so status text changes do not resize the HUD unpredictably.
- Tauri config sets a transparent, always-on-top, undecorated small HUD window.
