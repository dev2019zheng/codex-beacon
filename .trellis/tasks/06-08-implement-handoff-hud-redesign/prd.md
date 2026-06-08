# Implement handoff HUD redesign

## Goal

Implement the generated Codex Beacon handoff package as the default HUD shell, replacing the current rough card with a compact Quiet Neon Glass macOS floating utility UI while preserving the existing BeaconSnapshot data boundary.

## What I already know

- The handoff package is at `/Users/zhengyh/Documents/NeatDownload/Compressed/Codex_Beacon_Agent_Ready_Handoff_v3.zip`.
- The package defines P0 card mode, capsule mode, status visual system, CSS token contract, and snapshot-driven rendering.
- Current frontend is Tauri + React + vanilla CSS in `apps/desktop-tauri/src`.
- Current app already has `BeaconSnapshot`, manual/demo controls, hook/manual/simulation source, and card/capsule state.
- Tauri config is already transparent, undecorated, always-on-top, skip-taskbar, and non-resizable.

## Assumptions

- The package is the source of truth for this task, so no extra design question is blocking implementation.
- Use vanilla CSS and existing React dependencies; no new UI library.
- Debug/manual status controls may remain but must be visually weak and secondary.
- We should not commit handoff images/assets into the app; use them as implementation references only.

## Requirements

- Rework the HUD into the handoff component shape: window shell, animation layer, card mode, capsule mode, status orb, status pill, metrics strip, task list, and footer controls.
- Use CSS custom properties for all status colors and glow values.
- Bind visual state through `data-status` and `data-alert`; do not hardcode colors in React logic.
- Card mode targets approximately `360x176` and capsule mode approximately `240x48`.
- On collapse/expand in Tauri, resize the native window to match the visual mode.
- Preserve browser preview behavior.
- Preserve snapshot polling, manual status controls, theme selection, and error display.
- Show at most 3 visible tasks, sorted by status priority.
- Text must truncate instead of resizing or overflowing the window.
- Waiting states need restrained but obvious halo/electric edge animation.
- Include reduced-motion handling.

## Acceptance Criteria

- [x] Card mode visually follows the handoff reference: glass shell, compact header, radar/orb area, status copy, metrics strip, recent tasks, weak footer controls.
- [x] Capsule mode visually follows the handoff reference: 240x48 pill, orb, status label, summary, time, refresh.
- [x] All seven statuses render distinct state colors and labels.
- [x] Manual demo controls still let developers preview each state.
- [x] Tauri window resizes between card and capsule modes when available.
- [x] Browser preview renders without Tauri APIs.
- [x] Typecheck, build, and Tauri bundle checks pass.
- [x] Visual smoke check is captured with browser screenshots for card and capsule.

## Definition of Done

- `pnpm --filter @codex-beacon/desktop typecheck`
- `pnpm --filter @codex-beacon/desktop build`
- `pnpm --filter @codex-beacon/desktop tauri:build --bundles app`
- Visual smoke check of card and capsule modes.
- Trellis task validation passes.
- Changes committed with Lore commit protocol and pushed to `master`.

## Out of Scope

- Fully implementing the mascot theme.
- Importing or embedding the handoff PNG assets.
- Global theme marketplace or right-click menu.
- Persistent window-position storage.
- Native macOS notification center.

## Technical Notes

- Handoff sections read: README, implementation checklist, master spec, visual language, card mode, capsule mode, capsule state gallery, status visual system, card state gallery, tokens CSS, motion CSS, React/Tauri implementation, mascot preserved.
- Current files likely impacted: `apps/desktop-tauri/src/App.tsx`, `apps/desktop-tauri/src/App.css`, `apps/desktop-tauri/src-tauri/tauri.conf.json`.
- Keep `beaconApi.ts` as the sole frontend IPC/data adapter.
