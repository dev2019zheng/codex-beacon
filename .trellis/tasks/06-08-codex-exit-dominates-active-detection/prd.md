# Codex Exit Dominates Active Detection

## Problem

Codex Beacon can keep showing a task as running after Codex Desktop has stopped it. The current SQLite activity query treats the latest recent log row as authoritative. Codex may write tail logs after the real `Agent loop exited` event, so the HUD can be held in a false running state until the active window expires.

The previous project-workspace filter was also too strict: temporary Codex Desktop Chat workspaces are valid tasks and should be included when they are actually running.

## Goals

- Include temporary Codex Desktop Chat workspaces in the default HUD.
- Treat the true Codex session exit log as a terminal marker.
- Ignore short tail logs that arrive immediately after a true exit marker.
- Allow a later new turn to re-activate the same thread.
- Keep the Codex Desktop source read-only.

## Non-Goals

- No frontend redesign.
- No manual/demo state controls.
- No writes to Codex Desktop state.
- No full completed/waiting status mapping in this fix.

## Acceptance

- A temporary Chat workspace can appear as running when it has active logs.
- A thread with a real `Agent loop exited` marker followed by tail logs does not appear as running.
- A thread with new activity after the exit settle window appears as running again.
- Rust formatting, Rust workspace tests, and Tauri app bundle build pass.
