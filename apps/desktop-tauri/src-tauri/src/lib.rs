use std::sync::Mutex;

use beacon_core::{simulated_snapshot, snapshot_for_status, BeaconSnapshot, CodexTaskStatus};

#[derive(Default)]
struct BeaconState {
    tick: Mutex<u64>,
    manual_status: Mutex<Option<CodexTaskStatus>>,
}

#[tauri::command]
fn get_beacon_snapshot(state: tauri::State<'_, BeaconState>) -> BeaconSnapshot {
    if let Some(status) = state
        .manual_status
        .lock()
        .expect("manual status lock poisoned")
        .clone()
    {
        return snapshot_for_status(status, chrono::Utc::now());
    }

    let mut tick = state.tick.lock().expect("tick lock poisoned");
    let snapshot = simulated_snapshot(*tick);
    *tick = tick.saturating_add(1);
    snapshot
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

    snapshot_for_status(status, chrono::Utc::now())
}

#[tauri::command]
fn clear_manual_status(state: tauri::State<'_, BeaconState>) -> BeaconSnapshot {
    *state
        .manual_status
        .lock()
        .expect("manual status lock poisoned") = None;

    let mut tick = state.tick.lock().expect("tick lock poisoned");
    *tick = 0;
    simulated_snapshot(0)
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
