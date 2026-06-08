# Frontend Directory Structure

## Overview

The desktop frontend lives in the Tauri app package. Built-in theme manifests live at the repository root so later shells can reuse them.

## Directory Layout

```text
apps/desktop-tauri/
  src/
    App.tsx        # HUD composition
    App.css        # HUD layout and visual states
    beaconApi.ts   # Tauri IPC adapter with browser-preview fallback
themes/
  minimal-card/
  neon-hud/
  electric-mascot/
```

## Module Organization

- Keep Tauri IPC calls in `beaconApi.ts`, not scattered through components.
- Keep `App.tsx` focused on HUD state, view mode, and rendering.
- Keep theme manifests as declarative metadata. Theme code must consume the Beacon contract, not Codex internals.

## Naming Conventions

- Frontend package name is `@codex-beacon/desktop`.
- Theme IDs use kebab-case and match manifest directory names.

## Examples

- `apps/desktop-tauri/src/beaconApi.ts` defines the TypeScript mirror of `BeaconSnapshot`.
- `themes/electric-mascot/theme.json` declares the built-in original mascot theme entry.
