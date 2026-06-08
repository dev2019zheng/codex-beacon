# Remove Fake Status Controls And Fix Refresh Activity

## Problem

The HUD exposed debug status buttons in the production shell. Clicking them switched the app to synthetic manual snapshots, which made the status view look fake and undermined trust in the Codex Desktop source. Refresh also kept recently completed threads visible as running because the activity query filtered out exit logs before selecting each thread's latest log.

## Goals

- Remove all user-visible manual/demo status controls from the real HUD.
- Remove the Tauri manual status commands so the shell cannot enter a fake product state.
- Keep browser-only simulation as an adapter fallback for local visual preview.
- Treat a thread as active only when its latest recent Codex log is not `Agent loop exited`.
- Avoid state fallback from re-adding recently completed threads when the logs DB is available.

## Non-Goals

- No new Codex Desktop private API integration.
- No new UI theme work.
- No full waiting/completed detection beyond the currently available Codex logs and hook fallback.

## Acceptance

- The red-box debug buttons are gone from card mode.
- Refresh reads only real Codex Desktop or hook state in Tauri.
- A thread whose latest log is `Agent loop exited` is excluded from running tasks.
- Frontend typecheck/build, Rust tests, and Tauri app bundle build pass.
