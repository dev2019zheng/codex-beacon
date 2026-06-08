use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration as StdDuration,
};

use beacon_core::{
    parse_hook_events_jsonl, snapshot_for_status_with_source, snapshot_from_codex_app_tasks,
    snapshot_from_hook_events, BeaconSnapshot, BeaconSnapshotSource, CodexAppTask, CodexTaskStatus,
};
use chrono::TimeZone;
use rusqlite::{Connection, OpenFlags};

const CODEX_ACTIVE_WINDOW_SECONDS: i64 = 10 * 60;
const CODEX_UPDATED_FALLBACK_SECONDS: i64 = 2 * 60;
const CODEX_THREAD_QUERY_LIMIT: i64 = 50;
const CODEX_TASK_DISPLAY_LIMIT: usize = 5;

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

    automatic_snapshot(now)
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

    automatic_snapshot(chrono::Utc::now())
}

fn automatic_snapshot(now: chrono::DateTime<chrono::Utc>) -> BeaconSnapshot {
    snapshot_from_codex_app(now).unwrap_or_else(|| snapshot_from_hook_log(now))
}

fn snapshot_from_codex_app(now: chrono::DateTime<chrono::Utc>) -> Option<BeaconSnapshot> {
    let state_path = codex_state_db_path();
    if !state_path.is_file() {
        return None;
    }

    let active_threads = recent_codex_activity(&codex_logs_db_path(), now).unwrap_or_default();
    let tasks = codex_app_tasks_from_state(&state_path, &active_threads, now).ok()?;

    Some(snapshot_from_codex_app_tasks(tasks, now))
}

fn snapshot_from_hook_log(now: chrono::DateTime<chrono::Utc>) -> BeaconSnapshot {
    let events = fs::read_to_string(hook_event_log_path())
        .map(|contents| parse_hook_events_jsonl(&contents))
        .unwrap_or_default();

    snapshot_from_hook_events(&events, now)
}

fn recent_codex_activity(
    logs_path: &Path,
    now: chrono::DateTime<chrono::Utc>,
) -> rusqlite::Result<HashMap<String, i64>> {
    if !logs_path.is_file() {
        return Ok(HashMap::new());
    }

    let conn = open_readonly_sqlite(logs_path)?;
    let cutoff_seconds = now.timestamp() - CODEX_ACTIVE_WINDOW_SECONDS;
    let mut stmt = conn.prepare(
        r#"
        SELECT thread_id, MAX(ts * 1000 + ts_nanos / 1000000) AS last_activity_ms
        FROM logs
        WHERE thread_id IS NOT NULL
          AND ts >= ?1
          AND feedback_log_body NOT LIKE '%Agent loop exited%'
        GROUP BY thread_id
        "#,
    )?;
    let rows = stmt.query_map([cutoff_seconds], |row| {
        let thread_id: String = row.get(0)?;
        let last_activity_ms: i64 = row.get(1)?;
        Ok((thread_id, last_activity_ms))
    })?;

    rows.collect()
}

fn codex_app_tasks_from_state(
    state_path: &Path,
    active_threads: &HashMap<String, i64>,
    now: chrono::DateTime<chrono::Utc>,
) -> rusqlite::Result<Vec<CodexAppTask>> {
    let conn = open_readonly_sqlite(state_path)?;
    let fallback_cutoff_ms = now.timestamp_millis() - (CODEX_UPDATED_FALLBACK_SECONDS * 1000);
    let active_cutoff_ms = now.timestamp_millis() - (CODEX_ACTIVE_WINDOW_SECONDS * 1000);
    let mut stmt = conn.prepare(
        r#"
        SELECT id, title, cwd, COALESCE(updated_at_ms, updated_at * 1000) AS updated_at_ms
        FROM threads
        WHERE archived = 0
          AND COALESCE(thread_source, 'user') = 'user'
        ORDER BY updated_at_ms DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([CODEX_THREAD_QUERY_LIMIT], |row| {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let cwd: String = row.get(2)?;
        let updated_at_ms: i64 = row.get(3)?;
        Ok((id, title, cwd, updated_at_ms))
    })?;

    let mut tasks = Vec::new();
    for row in rows {
        let (id, title, cwd, updated_at_ms) = row?;
        let is_active_from_logs = active_threads
            .get(&id)
            .is_some_and(|last_activity_ms| *last_activity_ms >= active_cutoff_ms);
        let is_recent_from_state = updated_at_ms >= fallback_cutoff_ms;

        if !is_active_from_logs && !is_recent_from_state {
            continue;
        }

        tasks.push(CodexAppTask {
            id,
            title,
            workspace: Some(cwd),
            updated_at: millis_to_datetime(updated_at_ms, now),
        });

        if tasks.len() >= CODEX_TASK_DISPLAY_LIMIT {
            break;
        }
    }

    Ok(tasks)
}

fn open_readonly_sqlite(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    conn.busy_timeout(StdDuration::from_millis(200))?;
    Ok(conn)
}

fn millis_to_datetime(
    timestamp_ms: i64,
    fallback: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .unwrap_or(fallback)
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

fn codex_state_db_path() -> PathBuf {
    codex_db_path("CODEX_BEACON_CODEX_STATE_DB", "state_", "state_5.sqlite")
}

fn codex_logs_db_path() -> PathBuf {
    codex_db_path("CODEX_BEACON_CODEX_LOGS_DB", "logs_", "logs_2.sqlite")
}

fn codex_db_path(env_var: &str, prefix: &str, fallback_name: &str) -> PathBuf {
    if let Ok(path) = env::var(env_var) {
        return PathBuf::from(path);
    }

    let codex_dir = home_dir().join(".codex");
    latest_sqlite_for_prefix(&codex_dir, prefix).unwrap_or_else(|| codex_dir.join(fallback_name))
}

fn latest_sqlite_for_prefix(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            if !file_name.starts_with(prefix) || !file_name.ends_with(".sqlite") {
                return None;
            }

            Some((sqlite_suffix_number(file_name, prefix), path))
        })
        .max_by_key(|(number, _)| *number)
        .map(|(_, path)| path)
}

fn sqlite_suffix_number(file_name: &str, prefix: &str) -> u32 {
    file_name
        .trim_start_matches(prefix)
        .trim_end_matches(".sqlite")
        .parse()
        .unwrap_or(0)
}

fn home_dir() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn desktop_sqlite_source_returns_recent_active_user_threads() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-08T08:01:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let state_path = temp_sqlite_path("state");
        let logs_path = temp_sqlite_path("logs");

        create_state_fixture(&state_path, now);
        create_logs_fixture(&logs_path, now);

        let active_threads = recent_codex_activity(&logs_path, now).unwrap();
        let tasks = codex_app_tasks_from_state(&state_path, &active_threads, now).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "thread-active");
        assert_eq!(tasks[0].title, "Active user thread");
        assert_eq!(
            tasks[0].workspace.as_deref(),
            Some("/Users/example/codex-beacon")
        );

        fs::remove_file(state_path).ok();
        fs::remove_file(logs_path).ok();
    }

    fn create_state_fixture(path: &Path, now: chrono::DateTime<chrono::Utc>) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE threads (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                cwd TEXT NOT NULL,
                archived INTEGER NOT NULL,
                thread_source TEXT,
                updated_at INTEGER NOT NULL,
                updated_at_ms INTEGER
            );
            "#,
        )
        .unwrap();

        insert_thread(
            &conn,
            "thread-active",
            "Active user thread",
            "/Users/example/codex-beacon",
            "user",
            now.timestamp_millis() - 1_000,
        );
        insert_thread(
            &conn,
            "thread-stale",
            "Stale user thread",
            "/Users/example/stale",
            "user",
            now.timestamp_millis() - 900_000,
        );
        insert_thread(
            &conn,
            "thread-subagent",
            "Subagent thread",
            "/Users/example/subagent",
            "subagent",
            now.timestamp_millis() - 1_000,
        );
    }

    fn create_logs_fixture(path: &Path, now: chrono::DateTime<chrono::Utc>) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                ts_nanos INTEGER NOT NULL,
                feedback_log_body TEXT,
                thread_id TEXT
            );
            "#,
        )
        .unwrap();

        insert_log(
            &conn,
            "thread-active",
            now.timestamp() - 5,
            "session_task.turn receiving_stream",
        );
        insert_log(
            &conn,
            "thread-stale",
            now.timestamp() - 5,
            "Agent loop exited",
        );
        insert_log(
            &conn,
            "thread-subagent",
            now.timestamp() - 5,
            "session_task.turn receiving_stream",
        );
    }

    fn insert_thread(
        conn: &Connection,
        id: &str,
        title: &str,
        cwd: &str,
        thread_source: &str,
        updated_at_ms: i64,
    ) {
        conn.execute(
            r#"
            INSERT INTO threads (
                id,
                title,
                cwd,
                archived,
                thread_source,
                updated_at,
                updated_at_ms
            ) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6)
            "#,
            params![
                id,
                title,
                cwd,
                thread_source,
                updated_at_ms / 1000,
                updated_at_ms
            ],
        )
        .unwrap();
    }

    fn insert_log(conn: &Connection, thread_id: &str, ts: i64, body: &str) {
        conn.execute(
            "INSERT INTO logs (ts, ts_nanos, feedback_log_body, thread_id) VALUES (?1, 0, ?2, ?3)",
            params![ts, body, thread_id],
        )
        .unwrap();
    }

    fn temp_sqlite_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("codex-beacon-{prefix}-{nanos}.sqlite"))
    }
}
