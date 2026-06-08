# Frontend Quality Guidelines

## Overview

The HUD is a compact utility surface. It should stay dense, readable, and stable across status changes.

## Required Patterns

- Run `pnpm --filter @codex-beacon/desktop typecheck`.
- Run `pnpm --filter @codex-beacon/desktop build`.
- Keep fixed or bounded dimensions for HUD controls so status text changes do not resize the window unpredictably.
- Use `title` and `aria-label` for compact icon/symbol controls.
- Keep browser preview working through the adapter fallback so visual checks can run without a Tauri runtime.

## Forbidden Patterns

- Do not use landing-page or marketing composition for the desktop HUD.
- Do not let button labels wrap inside compact controls; use symbols with accessible labels when space is tight.
- Do not commit generated `dist/`, `*.tsbuildinfo`, or transpiled Vite config output.

## Testing Requirements

- For visual changes, inspect card and capsule modes.
- For shell changes, run the Tauri app bundle check: `pnpm --filter @codex-beacon/desktop tauri:build --bundles app`.

## Code Review Checklist

- Does the HUD remain readable in a small window?
- Are strong waiting alerts visually distinct from running/completed states?
- Does the browser preview still render without Tauri IPC?

## Scenario: Beacon HUD Shell Contract

### 1. Scope / Trigger

- Trigger: visual or interaction work on the desktop HUD, including new shells/themes, card/capsule mode changes, or native window sizing.
- Goal: keep the UI replaceable as a shell over the status core rather than coupling a theme to Codex/Tauri internals.

### 2. Signatures

- Snapshot input: `BeaconSnapshot` from `apps/desktop-tauri/src/beaconApi.ts`.
- Task rows: `CodexTaskSnapshot[]` with `id`, `title`, `status`, `detail`, and `updatedAt`.
- View mode adapter: `setBeaconWindowMode(mode: "card" | "capsule")`.
- Shell state marker: `.beacon-window[data-status="<CodexTaskStatus>"]`.

### 3. Contracts

- Card mode target size is `480x272`; capsule mode target size is `280x52`.
- Tauri window APIs stay behind `beaconApi.ts`; React components should call the adapter, not import Tauri window modules directly.
- Tauri capsule resizing requires `core:window:allow-set-size` in `src-tauri/capabilities/default.json`.
- Explicit native dragging requires `core:window:allow-start-dragging` in `src-tauri/capabilities/default.json` and should be exposed through the adapter.
- Transparent macOS HUD windows require both `app.macOSPrivateApi = true` in `tauri.conf.json` and the Rust `tauri/macos-private-api` feature.
- Themes/shells read status from `BeaconSnapshot` props and CSS data attributes. They must not parse Codex logs, hook files, or process state directly.
- Status colors and alert effects belong in CSS variables such as `--state-color`, `--state-glow`, and `--state-soft`.

### 4. Validation & Error Matrix

- Missing Tauri runtime -> browser fallback returns a simulated snapshot and `setBeaconWindowMode` is a no-op, including browser errors such as missing `__TAURI_INTERNALS__`, `__TAURI_IPC__`, or `invoke`.
- Window resize failure -> keep the current React mode visible and surface the error through the shell error slot.
- Missing/empty hook event log in Tauri -> render a hooks-sourced `unknown` snapshot, not `idle`, so setup failures are visible.
- Long title/detail text -> truncate inside its row; the HUD window must not grow or wrap unpredictably.
- Empty task list -> render an idle/empty affordance instead of blank space.

### 5. Good/Base/Bad Cases

- Good: a mascot theme receives the same `BeaconSnapshot` and replaces only visual presentation plus optional motion.
- Base: card and capsule modes render the same overall status, counts, and updated time in different densities.
- Bad: a shell imports `@tauri-apps/api/window` directly or reads `.codex` state files from React.

### 6. Tests Required

- Run `pnpm --filter @codex-beacon/desktop typecheck`.
- Run `pnpm --filter @codex-beacon/desktop build`.
- Run `pnpm --filter @codex-beacon/desktop tauri:build --bundles app` when native window sizing or Tauri config changes.
- Run `cargo test --workspace` when changing hook snapshot semantics.
- Capture/inspect card and capsule previews after visual changes.

### 7. Wrong vs Correct

#### Wrong

```typescript
import { getCurrentWindow } from "@tauri-apps/api/window";
await getCurrentWindow().setSize(...);
```

#### Correct

```typescript
await setBeaconWindowMode("capsule");
```
