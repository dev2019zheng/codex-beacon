# Align Active Codex Task Detection

## Problem

The HUD currently reports three running tasks while the user expects two project coding tasks. The extra row is a Codex Desktop chat session whose `cwd` is under `~/Documents/Codex/<date>/...`; it is active in Codex logs, but it belongs to the Chats area rather than a real project coding workspace.

## Goals

- Keep Codex Beacon read-only and driven by real Codex Desktop state.
- Count only active project/programming tasks in the default HUD.
- Exclude Codex Desktop temporary chat workspaces such as `~/Documents/Codex/2026-06-08/agent`.
- Preserve hook fallback and browser simulation behavior.
- Add regression coverage for the chat-workspace exclusion.

## Non-Goals

- No new manual status controls.
- No UI redesign in this fix.
- No destructive changes to Codex Desktop state.
- No preference UI for including Chat sessions yet.

## Acceptance

- Active project tasks from real workspace paths still appear as running.
- Active Codex Desktop chat sessions under `~/Documents/Codex/<date>/...` do not appear in the default running list.
- The user's current data should resolve to two project tasks instead of three.
- Rust formatting and workspace tests pass.
