# Codex Beacon

Codex Beacon is a macOS desktop HUD for watching Codex task status while you work elsewhere. The product shape is a Rust core with replaceable desktop shells and theme shells.

## MVP

- Rust core status model in `core/beacon-core`
- Tauri desktop shell in `apps/desktop-tauri`
- Project-level Codex hook event source
- Browser-preview simulated status source
- Compact card mode and capsule mode
- Built-in theme entries for `minimal-card`, `neon-hud`, and `electric-mascot`

## Status Source

Codex Beacon reads sanitized project hook events from:

```text
~/.codex-beacon/events.jsonl
```

Set `CODEX_BEACON_EVENT_LOG=/custom/path/events.jsonl` to override the path for development or tests.

The repo-level `.codex/hooks.json` records `SessionStart`, `UserPromptSubmit`, tool activity, and `Stop` events through `.codex/hooks/beacon-record-event.py`. The recorder stores only safe metadata such as event name, timestamp, session id, cwd, tool name, and a short summary. It does not persist prompts, tool arguments, command output, or Codex responses.

If the hook log is missing or empty, the desktop shell shows an idle hooks snapshot. The browser preview keeps a simulated source so UI work remains possible without a Tauri runtime.

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
