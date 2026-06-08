# Codex Beacon

Codex Beacon is a macOS desktop HUD for watching Codex task status while you work elsewhere. The product shape is a Rust core with replaceable desktop shells and theme shells.

## MVP

- Rust core status model in `core/beacon-core`
- Tauri desktop shell in `apps/desktop-tauri`
- Codex Desktop local status source with project hook fallback
- Browser-preview simulated status source
- Compact card mode and capsule mode
- Built-in theme entries for `minimal-card`, `neon-hud`, and `electric-mascot`

## Status Source

Codex Beacon first reads Codex Desktop local state in read-only mode:

```text
~/.codex/state_*.sqlite
~/.codex/logs_*.sqlite
```

Set `CODEX_BEACON_CODEX_STATE_DB=/custom/state.sqlite` or `CODEX_BEACON_CODEX_LOGS_DB=/custom/logs.sqlite` to override these paths for development or tests.

The Desktop source joins thread metadata from `state_*.sqlite` with recent activity from `logs_*.sqlite`, so multiple active Codex Desktop threads can appear in one HUD snapshot.

If the Desktop DBs are missing or unreadable, Codex Beacon falls back to sanitized project hook events from:

```text
~/.codex-beacon/events.jsonl
```

Set `CODEX_BEACON_EVENT_LOG=/custom/path/events.jsonl` to override the path for development or tests.

The repo-level `.codex/hooks.json` records `SessionStart`, `UserPromptSubmit`, tool activity, and `Stop` events through `.codex/hooks/beacon-record-event.py`. The recorder stores only safe metadata such as event name, timestamp, session id, cwd, tool name, and a short summary. It does not persist prompts, tool arguments, command output, or Codex responses.

If the hook log is missing or empty after Desktop fallback fails, the desktop shell shows an unknown hooks snapshot so it is clear that real Codex status is not connected yet. The browser preview keeps a simulated source so UI work remains possible without a Tauri runtime.

## Run

```bash
pnpm install
pnpm desktop:dev
```

## Verify

```bash
cargo test --workspace
python3 .codex/hooks/beacon-record-event.py --self-test
pnpm typecheck
pnpm build
```

Release-channel planning lives in `docs/release-pipeline.md`.
