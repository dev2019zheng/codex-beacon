# Codex hook status source

## Goal

Wire Codex Beacon to a first real local status source: Codex project hooks record sanitized lifecycle events into a local event queue, and the Rust core turns those events into the HUD snapshot consumed by the Tauri shell.

## What I already know

- The app architecture is Tauri shell plus Rust core, with the core owning status normalization and theme contracts.
- The current desktop app only cycles simulated snapshots or manual demo statuses.
- The project already has a repo-level `.codex/hooks.json` for Trellis `UserPromptSubmit`.
- The desired product behavior is a read-only desktop HUD that can show running, completed, waiting for confirmation/input, and attention-worthy state changes.
- Hooks must not break Codex sessions if Beacon recording fails.

## Assumptions

- MVP can use repo-level Codex hooks first instead of installing a global watcher.
- The local event queue can live under `~/.codex-beacon/events.jsonl`, with `CODEX_BEACON_EVENT_LOG` as an override for tests and custom setups.
- Hook events should store only sanitized metadata, not full prompts, tool arguments, secrets, or command output.
- Exact Codex hook payload shapes may vary, so the recorder and parser should be tolerant.

## Requirements

- Add a hook recorder script that accepts Codex hook JSON on stdin and appends sanitized event records to JSONL.
- Extend `.codex/hooks.json` so project hooks record `UserPromptSubmit`, tool activity, and turn completion events while preserving the existing Trellis hook.
- Add Rust core types/functions for parsing hook event logs and deriving a `BeaconSnapshot`.
- Update Tauri command handling to prefer hook-derived snapshots when no manual demo status is selected.
- Keep browser preview/demo mode working without Tauri.
- Surface snapshot source lightly in the UI so the user can distinguish hooks, manual demo, and browser simulation.
- Document how the real status source works and how to override the event log path.

## Acceptance Criteria

- [ ] Running the hook recorder self-test writes a sanitized JSONL event.
- [ ] Rust unit tests cover hook event parsing/status synthesis.
- [ ] Desktop app builds and typechecks after adding the real source field.
- [ ] If the hook log is absent or empty, the Tauri app returns a stable idle/unknown snapshot instead of failing.
- [ ] Manual status controls still work for theme and alert testing.
- [ ] The existing Trellis `UserPromptSubmit` hook remains configured.

## Definition of Done

- `python3 .codex/hooks/beacon-record-event.py --self-test`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `pnpm --filter @codex-beacon/desktop typecheck`
- `pnpm --filter @codex-beacon/desktop build`
- `pnpm --filter @codex-beacon/desktop tauri:build --bundles app`
- Trellis task validation passes.
- Changes are committed with the Lore commit protocol and pushed to `master`.

## Out of Scope

- Global installation into `~/.codex` or cross-repository hook management.
- Native macOS notification center integration.
- Multi-session dashboards or per-thread selection UI.
- A full plugin/theme marketplace.
- Perfect detection of every future Codex internal hook event shape.

## Technical Notes

- `core/beacon-core/src/lib.rs` currently owns snapshot structs, simulated state, and alert priority.
- `apps/desktop-tauri/src-tauri/src/lib.rs` currently owns Tauri commands and simulated tick state.
- `apps/desktop-tauri/src/beaconApi.ts` mirrors the Rust snapshot contract for the React shell and browser preview.
- `.codex/hooks.json` currently contains only the Trellis `UserPromptSubmit` command.
