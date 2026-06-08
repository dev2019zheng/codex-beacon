# Backend Directory Structure

## Overview

Backend-like code for Codex Beacon lives in Rust workspace crates. The core crate owns status contracts and pure state behavior. Desktop shell crates expose Tauri commands and adapt the core to app windows.

## Directory Layout

```text
core/
  beacon-core/          # Pure status model, snapshot construction, tests
apps/
  desktop-tauri/
    src-tauri/          # Tauri shell, commands, window config
```

## Module Organization

- Put reusable status models and state-machine behavior in `core/beacon-core`.
- Put Tauri-specific command handlers and application startup in `apps/desktop-tauri/src-tauri`.
- Do not make themes or frontend components read Codex files directly; expose status through shell commands or a future bridge API.

## Naming Conventions

- Rust crates use kebab-case package names.
- Serialized status payloads use `camelCase` fields and `snake_case` enum values.
- Command names are action-oriented, for example `get_beacon_snapshot` and `set_manual_status`.

## Examples

- `core/beacon-core/src/lib.rs` defines `BeaconSnapshot`, `CodexTaskSnapshot`, `CodexTaskStatus`, and alert mapping.
- `apps/desktop-tauri/src-tauri/src/lib.rs` adapts Tauri commands to the core crate.
