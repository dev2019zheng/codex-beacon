# Codex Beacon Desktop

Tauri shell for the Codex Beacon read-only desktop HUD.

## Development

```bash
pnpm install
pnpm --filter @codex-beacon/desktop tauri:dev
```

The MVP uses simulated Codex task state. The shell polls Rust core once per minute and includes temporary development controls for status switching.

## Checks

```bash
pnpm --filter @codex-beacon/desktop typecheck
pnpm --filter @codex-beacon/desktop build
cargo test --workspace
```
