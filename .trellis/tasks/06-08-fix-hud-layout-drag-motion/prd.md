# Fix HUD Layout, Drag, And Motion

## Problem

The current card HUD is too small for the live multi-task state. Header controls, task rows, theme controls, and timestamps crowd together, and the footer controls overlap the task list. The floating window also relies on drag-region styling but does not expose a reliable native drag path. Running state motion is too subtle to reassure the user that Codex is active.

## Goals

- Card mode remains a compact always-on-top HUD but has enough room to show four task rows without overlap.
- Capsule mode stays small and readable after collapse.
- The user can drag the floating window from non-interactive HUD surfaces.
- Running state has visible, restrained motion; waiting states keep stronger alert behavior.
- Tauri window APIs remain behind the frontend adapter.

## Non-Goals

- No new theme system redesign.
- No changes to Codex status parsing semantics.
- No new dependencies.

## Acceptance

- Card mode uses the updated bounded shell size and no UI text overlaps in the multi-task running preview.
- Footer controls no longer float over task rows.
- Native drag permission is declared and drag is initiated through `beaconApi.ts`.
- Browser preview, typecheck, frontend build, and Tauri app bundle build pass.
