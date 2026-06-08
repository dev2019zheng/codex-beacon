# Codex Beacon

Codex Beacon is a macOS desktop HUD for watching Codex task status while you work elsewhere. The product shape is a Rust core with replaceable desktop shells and theme shells.

## MVP

- Rust core status model in `core/beacon-core`
- Tauri desktop shell in `apps/desktop-tauri`
- Simulated read-only status source
- Compact card mode and capsule mode
- Built-in theme entries for `minimal-card`, `neon-hud`, and `electric-mascot`

## Run

```bash
pnpm install
pnpm desktop:dev
```

## Verify

```bash
cargo test --workspace
pnpm typecheck
pnpm build
```

Release-channel planning lives in `docs/release-pipeline.md`.
