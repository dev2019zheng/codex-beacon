use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    time::Duration as StdDuration,
};

use beacon_core::{
    parse_hook_events_jsonl, snapshot_from_codex_app_tasks, snapshot_from_hook_events,
    BeaconSnapshot, CodexAppTask,
};
use chrono::TimeZone;
use rusqlite::{Connection, OpenFlags};

const CODEX_ACTIVE_WINDOW_SECONDS: i64 = 10 * 60;
const CODEX_MARKER_WINDOW_SECONDS: i64 = 24 * 60 * 60;
const CODEX_UPDATED_FALLBACK_SECONDS: i64 = 2 * 60;
const CODEX_THREAD_QUERY_LIMIT: i64 = 50;
const CODEX_TASK_DISPLAY_LIMIT: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CodexThreadActivity {
    last_activity_ms: i64,
    last_exit_ms: Option<i64>,
}

#[tauri::command]
fn get_beacon_snapshot(refresh_nonce: Option<u64>) -> BeaconSnapshot {
    let _ = refresh_nonce;
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

    let logs_path = codex_logs_db_path();
    let allow_recent_state_fallback = !logs_path.is_file();
    let active_threads = recent_codex_activity(&logs_path, now).unwrap_or_default();
    let tasks = codex_app_tasks_from_state(
        &state_path,
        &active_threads,
        allow_recent_state_fallback,
        now,
    )
    .ok()?;

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
) -> rusqlite::Result<HashMap<String, CodexThreadActivity>> {
    if !logs_path.is_file() {
        return Ok(HashMap::new());
    }

    let conn = open_readonly_sqlite(logs_path)?;
    let cutoff_seconds = now.timestamp() - CODEX_ACTIVE_WINDOW_SECONDS;
    let marker_cutoff_seconds = now.timestamp() - CODEX_MARKER_WINDOW_SECONDS;
    let mut stmt = conn.prepare(
        r#"
        WITH recent_activity AS (
            SELECT
                thread_id,
                MAX(ts * 1000 + ts_nanos / 1000000) AS last_activity_ms
            FROM logs
            WHERE thread_id IS NOT NULL
              AND ts >= ?1
              AND (
                feedback_log_body LIKE '%run_sampling_request%'
                OR feedback_log_body LIKE '%session_task.turn%'
              )
            GROUP BY thread_id
        ),
        thread_markers AS (
            SELECT
                recent_activity.thread_id,
                recent_activity.last_activity_ms,
                (
                    SELECT MAX(exit_logs.ts * 1000 + exit_logs.ts_nanos / 1000000)
                    FROM logs AS exit_logs
                    WHERE exit_logs.thread_id = recent_activity.thread_id
                      AND exit_logs.ts >= ?2
                      AND exit_logs.target = 'codex_core::session::handlers'
                      AND exit_logs.feedback_log_body LIKE '%}: Agent loop exited'
                )
                AS last_exit_ms
            FROM recent_activity
        )
        SELECT thread_id, last_activity_ms, last_exit_ms
        FROM thread_markers
        "#,
    )?;
    let rows = stmt.query_map((cutoff_seconds, marker_cutoff_seconds), |row| {
        let thread_id: String = row.get(0)?;
        let last_activity_ms: i64 = row.get(1)?;
        let last_exit_ms: Option<i64> = row.get(2)?;
        Ok((
            thread_id,
            CodexThreadActivity {
                last_activity_ms,
                last_exit_ms,
            },
        ))
    })?;

    rows.collect()
}

fn codex_app_tasks_from_state(
    state_path: &Path,
    active_threads: &HashMap<String, CodexThreadActivity>,
    allow_recent_state_fallback: bool,
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
        let activity = active_threads.get(&id).copied();
        let is_active_from_logs = activity
            .is_some_and(|activity| is_active_activity(activity, updated_at_ms, active_cutoff_ms));
        let is_recent_from_state =
            allow_recent_state_fallback && updated_at_ms >= fallback_cutoff_ms;

        if !is_active_from_logs && !is_recent_from_state {
            continue;
        }

        let display_updated_at_ms = activity
            .map(|activity| activity.last_activity_ms.max(updated_at_ms))
            .unwrap_or(updated_at_ms);

        tasks.push(CodexAppTask {
            id,
            title,
            workspace: Some(cwd),
            updated_at: millis_to_datetime(display_updated_at_ms, now),
        });

        if tasks.len() >= CODEX_TASK_DISPLAY_LIMIT {
            break;
        }
    }

    Ok(tasks)
}

fn is_active_activity(
    activity: CodexThreadActivity,
    state_updated_at_ms: i64,
    active_cutoff_ms: i64,
) -> bool {
    if activity.last_activity_ms < active_cutoff_ms {
        return false;
    }

    let Some(last_exit_ms) = activity.last_exit_ms else {
        return true;
    };

    activity.last_activity_ms > last_exit_ms && state_updated_at_ms > last_exit_ms
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_beacon_snapshot])
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
        let tasks = codex_app_tasks_from_state(&state_path, &active_threads, false, now).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "thread-active");
        assert_eq!(tasks[0].title, "Active user thread");
        assert_eq!(
            tasks[0].workspace.as_deref(),
            Some("/Users/example/codex-beacon")
        );
        assert_eq!(
            tasks[0].updated_at.timestamp_millis(),
            now.timestamp_millis()
        );

        fs::remove_file(state_path).ok();
        fs::remove_file(logs_path).ok();
    }

    #[test]
    fn desktop_sqlite_source_excludes_threads_whose_latest_log_exited() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-08T08:01:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let state_path = temp_sqlite_path("state-exited");
        let logs_path = temp_sqlite_path("logs-exited");

        let state_conn = Connection::open(&state_path).unwrap();
        state_conn
            .execute_batch(
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
            &state_conn,
            "thread-finished",
            "Finished user thread",
            "/Users/example/codex-beacon",
            "user",
            now.timestamp_millis() - 1_000,
        );

        let logs_conn = Connection::open(&logs_path).unwrap();
        create_logs_schema(&logs_conn);
        insert_log(
            &logs_conn,
            "thread-finished",
            now.timestamp() - 4,
            "session_task.turn receiving_stream",
        );
        insert_log_with_target(
            &logs_conn,
            "thread-finished",
            now.timestamp() - 1,
            "codex_core::session::handlers",
            "session_loop{thread_id=thread-finished}: Agent loop exited",
        );

        let active_threads = recent_codex_activity(&logs_path, now).unwrap();
        let tasks = codex_app_tasks_from_state(&state_path, &active_threads, false, now).unwrap();

        assert!(active_threads.contains_key("thread-finished"));
        assert!(tasks.is_empty());

        fs::remove_file(state_path).ok();
        fs::remove_file(logs_path).ok();
    }

    #[test]
    fn desktop_sqlite_source_includes_codex_desktop_chat_workspaces() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-08T08:01:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let state_path = temp_sqlite_path("state-chat");
        let logs_path = temp_sqlite_path("logs-chat");

        let state_conn = Connection::open(&state_path).unwrap();
        state_conn
            .execute_batch(
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
            &state_conn,
            "thread-chat",
            "Learning chat",
            "/Users/example/Documents/Codex/2026-06-08/agent",
            "user",
            now.timestamp_millis() - 1_000,
        );
        insert_thread(
            &state_conn,
            "thread-project",
            "Project coding task",
            "/Users/example/codex-beacon",
            "user",
            now.timestamp_millis() - 1_000,
        );

        let logs_conn = Connection::open(&logs_path).unwrap();
        create_logs_schema(&logs_conn);
        insert_log(
            &logs_conn,
            "thread-chat",
            now.timestamp() - 5,
            &active_turn_log_body("thread-chat"),
        );
        insert_log(
            &logs_conn,
            "thread-project",
            now.timestamp() - 5,
            &active_turn_log_body("thread-project"),
        );

        let active_threads = recent_codex_activity(&logs_path, now).unwrap();
        let tasks = codex_app_tasks_from_state(&state_path, &active_threads, false, now).unwrap();

        assert!(active_threads.contains_key("thread-chat"));
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "thread-chat");
        assert_eq!(tasks[1].id, "thread-project");

        fs::remove_file(state_path).ok();
        fs::remove_file(logs_path).ok();
    }

    #[test]
    fn desktop_sqlite_source_treats_true_exit_as_terminal_despite_tail_logs() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-08T08:01:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let state_path = temp_sqlite_path("state-tail");
        let logs_path = temp_sqlite_path("logs-tail");

        let state_conn = Connection::open(&state_path).unwrap();
        state_conn
            .execute_batch(
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
            &state_conn,
            "thread-tail",
            "Stopped task with tail logs",
            "/Users/example/codex-beacon",
            "user",
            now.timestamp_millis() - 5_000,
        );

        let logs_conn = Connection::open(&logs_path).unwrap();
        create_logs_schema(&logs_conn);
        insert_log(
            &logs_conn,
            "thread-tail",
            now.timestamp() - 10,
            &active_turn_log_body("thread-tail"),
        );
        insert_log_with_target(
            &logs_conn,
            "thread-tail",
            now.timestamp() - 5,
            "codex_core::session::handlers",
            "session_loop{thread_id=thread-tail}: Agent loop exited",
        );
        insert_log(
            &logs_conn,
            "thread-tail",
            now.timestamp() - 1,
            "session_task.turn run_sampling_request tail log",
        );

        let active_threads = recent_codex_activity(&logs_path, now).unwrap();
        let tasks = codex_app_tasks_from_state(&state_path, &active_threads, false, now).unwrap();

        assert!(active_threads.contains_key("thread-tail"));
        assert!(tasks.is_empty());

        fs::remove_file(state_path).ok();
        fs::remove_file(logs_path).ok();
    }

    #[test]
    fn desktop_sqlite_source_reactivates_after_exit_settle_window() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-08T08:01:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let state_path = temp_sqlite_path("state-reactivated");
        let logs_path = temp_sqlite_path("logs-reactivated");

        let state_conn = Connection::open(&state_path).unwrap();
        state_conn
            .execute_batch(
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
            &state_conn,
            "thread-reactivated",
            "Reactivated task",
            "/Users/example/codex-beacon",
            "user",
            now.timestamp_millis() - 1_000,
        );

        let logs_conn = Connection::open(&logs_path).unwrap();
        create_logs_schema(&logs_conn);
        insert_log_with_target(
            &logs_conn,
            "thread-reactivated",
            now.timestamp() - 30,
            "codex_core::session::handlers",
            "session_loop{thread_id=thread-reactivated}: Agent loop exited",
        );
        insert_log(
            &logs_conn,
            "thread-reactivated",
            now.timestamp() - 1,
            &active_turn_log_body("thread-reactivated"),
        );

        let active_threads = recent_codex_activity(&logs_path, now).unwrap();
        let tasks = codex_app_tasks_from_state(&state_path, &active_threads, false, now).unwrap();

        assert!(active_threads.contains_key("thread-reactivated"));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "thread-reactivated");

        fs::remove_file(state_path).ok();
        fs::remove_file(logs_path).ok();
    }

    #[test]
    fn desktop_sqlite_source_uses_recent_state_fallback_without_logs() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-08T08:01:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let state_path = temp_sqlite_path("state-fallback");

        create_state_fixture(&state_path, now);

        let tasks = codex_app_tasks_from_state(&state_path, &HashMap::new(), true, now).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "thread-active");

        fs::remove_file(state_path).ok();
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
        create_logs_schema(&conn);

        insert_log(
            &conn,
            "thread-active",
            now.timestamp(),
            &active_turn_log_body("thread-active"),
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
            &active_turn_log_body("thread-subagent"),
        );
    }

    fn active_turn_log_body(thread_id: &str) -> String {
        format!(
            "session_loop{{thread_id={thread_id}}}:submission_dispatch{{otel.name=\"op.dispatch.user_input\" codex.op=\"user_input\"}}:turn{{otel.name=\"session_task.turn\" thread.id={thread_id}}}:run_turn:run_sampling_request"
        )
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

    fn create_logs_schema(conn: &Connection) {
        conn.execute_batch(
            r#"
            CREATE TABLE logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                ts_nanos INTEGER NOT NULL,
                level TEXT,
                target TEXT,
                feedback_log_body TEXT,
                thread_id TEXT
            );
            "#,
        )
        .unwrap();
    }

    fn insert_log(conn: &Connection, thread_id: &str, ts: i64, body: &str) {
        insert_log_with_target(conn, thread_id, ts, "codex_otel.log_only", body);
    }

    fn insert_log_with_target(
        conn: &Connection,
        thread_id: &str,
        ts: i64,
        target: &str,
        body: &str,
    ) {
        conn.execute(
            r#"
            INSERT INTO logs (
                ts,
                ts_nanos,
                level,
                target,
                feedback_log_body,
                thread_id
            ) VALUES (?1, 0, 'DEBUG', ?2, ?3, ?4)
            "#,
            params![ts, target, body, thread_id],
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
