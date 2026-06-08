use std::{env, fs, path::PathBuf, sync::Mutex};

use beacon_core::{
    parse_hook_events_jsonl, snapshot_for_status_with_source, snapshot_from_hook_events,
    BeaconSnapshot, BeaconSnapshotSource, CodexTaskStatus,
};

#[derive(Default)]
struct BeaconState {
    manual_status: Mutex<Option<CodexTaskStatus>>,
}

#[tauri::command]
fn get_beacon_snapshot(state: tauri::State<'_, BeaconState>) -> BeaconSnapshot {
    let now = chrono::Utc::now();

    if let Some(status) = state
        .manual_status
        .lock()
        .expect("manual status lock poisoned")
        .clone()
    {
        return snapshot_for_status_with_source(status, now, BeaconSnapshotSource::Manual);
    }

    snapshot_from_hook_log(now)
}

#[tauri::command]
fn set_manual_status(
    state: tauri::State<'_, BeaconState>,
    status: CodexTaskStatus,
) -> BeaconSnapshot {
    *state
        .manual_status
        .lock()
        .expect("manual status lock poisoned") = Some(status.clone());

    snapshot_for_status_with_source(status, chrono::Utc::now(), BeaconSnapshotSource::Manual)
}

#[tauri::command]
fn clear_manual_status(state: tauri::State<'_, BeaconState>) -> BeaconSnapshot {
    *state
        .manual_status
        .lock()
        .expect("manual status lock poisoned") = None;

    snapshot_from_hook_log(chrono::Utc::now())
}

fn snapshot_from_hook_log(now: chrono::DateTime<chrono::Utc>) -> BeaconSnapshot {
    let events = fs::read_to_string(hook_event_log_path())
        .map(|contents| parse_hook_events_jsonl(&contents))
        .unwrap_or_default();

    snapshot_from_hook_events(&events, now)
}

fn hook_event_log_path() -> PathBuf {
    if let Ok(path) = env::var("CODEX_BEACON_EVENT_LOG") {
        return PathBuf::from(path);
    }

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".codex-beacon")
        .join("events.jsonl")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(BeaconState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_beacon_snapshot,
            set_manual_status,
            clear_manual_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
