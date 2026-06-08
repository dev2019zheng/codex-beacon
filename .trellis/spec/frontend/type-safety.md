# Frontend Type Safety

## Overview

The frontend is TypeScript strict mode. Types that mirror Rust IPC payloads must stay explicit and local to the shell adapter until generated bindings are introduced.

## Type Organization

- Define Tauri IPC payload types in `apps/desktop-tauri/src/beaconApi.ts`.
- Keep `CodexTaskStatus`, `AlertLevel`, `CodexTaskSnapshot`, `ThemeDescriptor`, and `BeaconSnapshot` aligned with `beacon-core`.
- Keep `BeaconSnapshot.source` aligned with the Rust `BeaconSnapshotSource` enum: `codex_app`, `hooks`, or `simulation`.
- Use `camelCase` field names in TypeScript because Rust payload structs serialize with `#[serde(rename_all = "camelCase")]`.

## Validation

MVP trusts Tauri command payloads. When real Codex state sources are added, validate input at the Rust core/source boundary before exposing a `BeaconSnapshot`.

## Common Patterns

```typescript
const hasTauriRuntime = "__TAURI_INTERNALS__" in window;
```

Use a browser-preview fallback only in the shell adapter. Components should call adapter functions such as `getBeaconSnapshot()` rather than importing `invoke` directly.

## Forbidden Patterns

- Do not use `any` for Beacon payloads.
- Do not call Tauri commands directly from multiple components.
- Do not make `tsc -b` emit `vite.config.js` or `.d.ts` files. Use `tsc --noEmit -p ...` for typecheck scripts.
