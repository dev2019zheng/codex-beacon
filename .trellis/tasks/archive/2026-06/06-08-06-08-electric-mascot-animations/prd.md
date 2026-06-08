# Implement Full Electric Mascot Animations

## Goal

Turn the existing `electric-mascot` theme entry into a real original animated character shell for Codex Beacon, while preserving the current read-only `BeaconSnapshot` boundary.

## Background

The app currently exposes an `Electric Mascot` option, but the UI still renders the generic status orb. The architecture document defines the mascot direction as an original electric character: running uses soft glow, completed jumps happily, failed short-circuits, and waiting states fire strong electric alerts.

## Requirements

- Add an original, non-IP electric mascot character rendered by the existing React/CSS shell.
- Render the mascot in card mode as the primary left-side status figure.
- Render a compact mascot expression in capsule mode.
- Keep `minimal-card` and `neon-hud` behavior visually compatible.
- Drive all mascot behavior only from `BeaconSnapshot.overallStatus`, `alertLevel`, and current view mode.
- Implement distinct animations for:
  - `running`: active work posture with soft tail/core electrical motion.
  - `completed`: cheerful jump or success bounce with celebratory glow.
  - `waiting_approval`: strong electric discharge reminder.
  - `waiting_input`: strong attention/request animation distinct from approval.
  - `failed`: short-circuit, shake, dim/error sparks.
  - `idle`: calm standby breathing.
  - `unknown`: searching/scanning uncertainty.
- Keep animation CSS scoped to `.theme-electric-mascot` where possible.
- Include `prefers-reduced-motion` safety through the existing global reduced-motion rule.
- Keep text truncation and HUD dimensions stable in both card and capsule modes.
- Do not add audio, notifications, Codex control actions, or direct Codex state parsing.
- Do not use copyrighted mascot names or third-party character likenesses.

## Acceptance Criteria

- Selecting `Electric Mascot` visibly replaces the generic orb with an original animated character in card mode.
- Capsule mode uses a compact mascot face/bolt indicator rather than the generic orb.
- Every supported `CodexTaskStatus` has a visibly distinct mascot pose or motion.
- Waiting states are clearly stronger than running/idle, with electric discharge effects.
- Completed state plays a celebratory one-shot or loop-safe success bounce.
- Browser preview can cycle through statuses and exercise the mascot without Tauri.
- Typecheck and production build pass.
- Card and capsule visual previews are inspected after implementation.

## Definition of Done

- `pnpm --filter @codex-beacon/desktop typecheck`
- `pnpm --filter @codex-beacon/desktop build`
- Visual smoke check of card and capsule mascot modes through browser preview.
- Trellis task validation passes.
- Changes committed with Lore commit protocol and pushed to `master`.

## Out of Scope

- Third-party theme marketplace.
- Persistent theme preference.
- Audio or macOS notification center integration.
- Real bitmap/spritesheet asset generation.
- Tauri/Rust status-source changes.
