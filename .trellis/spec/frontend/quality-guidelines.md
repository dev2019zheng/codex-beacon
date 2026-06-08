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
