use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexTaskStatus {
    Running,
    Completed,
    WaitingApproval,
    WaitingInput,
    Failed,
    Idle,
    Unknown,
}

impl CodexTaskStatus {
    pub fn alert_level(&self) -> AlertLevel {
        match self {
            CodexTaskStatus::Idle | CodexTaskStatus::Unknown => AlertLevel::Silent,
            CodexTaskStatus::Running => AlertLevel::Soft,
            CodexTaskStatus::Completed | CodexTaskStatus::Failed => AlertLevel::Normal,
            CodexTaskStatus::WaitingApproval | CodexTaskStatus::WaitingInput => AlertLevel::Strong,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Silent,
    Soft,
    Normal,
    Strong,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BeaconSnapshotSource {
    Hooks,
    Manual,
    Simulation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexTaskSnapshot {
    pub id: String,
    pub title: String,
    pub status: CodexTaskStatus,
    pub detail: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub supports_mascot: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeaconSnapshot {
    pub source: BeaconSnapshotSource,
    pub overall_status: CodexTaskStatus,
    pub alert_level: AlertLevel,
    pub active_count: usize,
    pub waiting_count: usize,
    pub tasks: Vec<CodexTaskSnapshot>,
    pub themes: Vec<ThemeDescriptor>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexHookEvent {
    #[serde(default)]
    pub schema_version: Option<u16>,
    pub timestamp: DateTime<Utc>,
    pub event: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
}

pub fn builtin_themes() -> Vec<ThemeDescriptor> {
    vec![
        ThemeDescriptor {
            id: "minimal-card".to_string(),
            name: "Minimal Card".to_string(),
            description: "Default compact card and capsule HUD.".to_string(),
            supports_mascot: false,
        },
        ThemeDescriptor {
            id: "neon-hud".to_string(),
            name: "Neon HUD".to_string(),
            description: "Glow-first status shell for completion and waiting events.".to_string(),
            supports_mascot: false,
        },
        ThemeDescriptor {
            id: "electric-mascot".to_string(),
            name: "Electric Mascot".to_string(),
            description: "Original electric mascot reminder personality.".to_string(),
            supports_mascot: true,
        },
    ]
}

pub fn simulated_snapshot(tick: u64) -> BeaconSnapshot {
    let now = Utc::now();
    let status = match tick % 5 {
        0 => CodexTaskStatus::Running,
        1 => CodexTaskStatus::WaitingApproval,
        2 => CodexTaskStatus::WaitingInput,
        3 => CodexTaskStatus::Completed,
        _ => CodexTaskStatus::Idle,
    };

    snapshot_for_status_with_source(status, now, BeaconSnapshotSource::Simulation)
}

pub fn snapshot_for_status(status: CodexTaskStatus, now: DateTime<Utc>) -> BeaconSnapshot {
    snapshot_for_status_with_source(status, now, BeaconSnapshotSource::Simulation)
}

pub fn snapshot_for_status_with_source(
    status: CodexTaskStatus,
    now: DateTime<Utc>,
    source: BeaconSnapshotSource,
) -> BeaconSnapshot {
    let tasks = match status {
        CodexTaskStatus::Idle => Vec::new(),
        CodexTaskStatus::Running => vec![
            task(
                "task-frontend",
                "HUD shell",
                CodexTaskStatus::Running,
                "Building the floating status view",
                now,
            ),
            task(
                "task-core",
                "Rust core",
                CodexTaskStatus::Running,
                "Normalizing simulated Codex state",
                now,
            ),
        ],
        CodexTaskStatus::WaitingApproval => vec![
            task(
                "task-release",
                "Preview release",
                CodexTaskStatus::WaitingApproval,
                "Waiting for command approval",
                now,
            ),
            task(
                "task-hud",
                "HUD polish",
                CodexTaskStatus::Running,
                "Rendering translucent overlay",
                now,
            ),
        ],
        CodexTaskStatus::WaitingInput => vec![task(
            "task-requirements",
            "Theme decision",
            CodexTaskStatus::WaitingInput,
            "Needs user input before continuing",
            now,
        )],
        CodexTaskStatus::Completed => vec![task(
            "task-mvp",
            "MVP scaffold",
            CodexTaskStatus::Completed,
            "Ready for review",
            now,
        )],
        CodexTaskStatus::Failed => vec![task(
            "task-build",
            "Tauri build",
            CodexTaskStatus::Failed,
            "Build failed in verification",
            now,
        )],
        CodexTaskStatus::Unknown => vec![task(
            "task-unknown",
            "Codex status",
            CodexTaskStatus::Unknown,
            "No recent state source available",
            now,
        )],
    };

    snapshot_from_tasks(status, tasks, now, source)
}

pub fn parse_hook_events_jsonl(input: &str) -> Vec<CodexHookEvent> {
    input
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            serde_json::from_str::<CodexHookEvent>(trimmed).ok()
        })
        .collect()
}

pub fn snapshot_from_hook_events(events: &[CodexHookEvent], now: DateTime<Utc>) -> BeaconSnapshot {
    let Some(latest) = events.iter().max_by_key(|event| event.timestamp) else {
        return snapshot_from_tasks(
            CodexTaskStatus::Idle,
            Vec::new(),
            now,
            BeaconSnapshotSource::Hooks,
        );
    };

    let status = status_for_hook_event(latest, now);
    if status == CodexTaskStatus::Idle {
        return snapshot_from_tasks(status, Vec::new(), now, BeaconSnapshotSource::Hooks);
    }

    let detail = latest
        .summary
        .clone()
        .unwrap_or_else(|| latest.event.clone());
    let task = CodexTaskSnapshot {
        id: hook_task_id(latest),
        title: hook_task_title(latest),
        status: status.clone(),
        detail,
        updated_at: latest.timestamp,
    };

    snapshot_from_tasks(status, vec![task], now, BeaconSnapshotSource::Hooks)
}

fn snapshot_from_tasks(
    fallback_status: CodexTaskStatus,
    tasks: Vec<CodexTaskSnapshot>,
    now: DateTime<Utc>,
    source: BeaconSnapshotSource,
) -> BeaconSnapshot {
    let overall_status = tasks
        .iter()
        .map(|task| task.status.clone())
        .max_by_key(status_priority)
        .unwrap_or(fallback_status);
    let alert_level = overall_status.alert_level();
    let active_count = tasks
        .iter()
        .filter(|task| matches!(task.status, CodexTaskStatus::Running))
        .count();
    let waiting_count = tasks
        .iter()
        .filter(|task| {
            matches!(
                task.status,
                CodexTaskStatus::WaitingApproval | CodexTaskStatus::WaitingInput
            )
        })
        .count();

    BeaconSnapshot {
        source,
        overall_status,
        alert_level,
        active_count,
        waiting_count,
        tasks,
        themes: builtin_themes(),
        updated_at: now,
    }
}

fn status_for_hook_event(event: &CodexHookEvent, now: DateTime<Utc>) -> CodexTaskStatus {
    if now.signed_duration_since(event.timestamp) > Duration::minutes(10) {
        return CodexTaskStatus::Idle;
    }

    let normalized = event.event.replace(['_', '-'], "").to_lowercase();

    if normalized.contains("approval") || normalized.contains("permission") {
        return CodexTaskStatus::WaitingApproval;
    }
    if normalized.contains("input") || normalized.contains("question") {
        return CodexTaskStatus::WaitingInput;
    }
    if normalized.contains("stop") || normalized.contains("complete") {
        return CodexTaskStatus::Completed;
    }
    if normalized.contains("userpromptsubmit")
        || normalized.contains("pretooluse")
        || normalized.contains("posttooluse")
        || normalized.contains("sessionstart")
    {
        return CodexTaskStatus::Running;
    }

    CodexTaskStatus::Unknown
}

fn hook_task_id(event: &CodexHookEvent) -> String {
    event
        .session_id
        .as_ref()
        .map(|session_id| format!("hook-{session_id}"))
        .unwrap_or_else(|| "hook-latest".to_string())
}

fn hook_task_title(event: &CodexHookEvent) -> String {
    if let Some(tool_name) = &event.tool_name {
        return format!("Codex {tool_name}");
    }

    if let Some(session_id) = &event.session_id {
        let short_id: String = session_id.chars().take(8).collect();
        return format!("Codex session {short_id}");
    }

    "Codex activity".to_string()
}

fn task(
    id: &str,
    title: &str,
    status: CodexTaskStatus,
    detail: &str,
    updated_at: DateTime<Utc>,
) -> CodexTaskSnapshot {
    CodexTaskSnapshot {
        id: id.to_string(),
        title: title.to_string(),
        status,
        detail: detail.to_string(),
        updated_at,
    }
}

fn status_priority(status: &CodexTaskStatus) -> u8 {
    match status {
        CodexTaskStatus::WaitingApproval | CodexTaskStatus::WaitingInput => 6,
        CodexTaskStatus::Failed => 5,
        CodexTaskStatus::Completed => 4,
        CodexTaskStatus::Running => 3,
        CodexTaskStatus::Unknown => 2,
        CodexTaskStatus::Idle => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_states_have_strong_alerts() {
        assert_eq!(
            CodexTaskStatus::WaitingApproval.alert_level(),
            AlertLevel::Strong
        );
        assert_eq!(
            CodexTaskStatus::WaitingInput.alert_level(),
            AlertLevel::Strong
        );
    }

    #[test]
    fn idle_snapshot_has_no_tasks() {
        let snapshot = snapshot_for_status(CodexTaskStatus::Idle, Utc::now());

        assert_eq!(snapshot.source, BeaconSnapshotSource::Simulation);
        assert_eq!(snapshot.overall_status, CodexTaskStatus::Idle);
        assert_eq!(snapshot.alert_level, AlertLevel::Silent);
        assert!(snapshot.tasks.is_empty());
    }

    #[test]
    fn simulated_snapshot_cycles_statuses() {
        let first = simulated_snapshot(0);
        let second = simulated_snapshot(1);

        assert_eq!(first.overall_status, CodexTaskStatus::Running);
        assert_eq!(second.overall_status, CodexTaskStatus::WaitingApproval);
    }

    #[test]
    fn parses_hook_events_from_jsonl() {
        let jsonl = r#"{"schemaVersion":1,"timestamp":"2026-06-08T08:00:00Z","event":"PreToolUse","summary":"Starting tool: Bash","sessionId":"abc","toolName":"Bash"}"#;
        let events = parse_hook_events_jsonl(jsonl);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "PreToolUse");
        assert_eq!(events[0].tool_name.as_deref(), Some("Bash"));
    }

    #[test]
    fn hook_snapshot_maps_waiting_events_to_strong_alerts() {
        let now = DateTime::parse_from_rfc3339("2026-06-08T08:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let events = parse_hook_events_jsonl(
            r#"{"schemaVersion":1,"timestamp":"2026-06-08T08:00:30Z","event":"PermissionRequest","summary":"Waiting for approval","sessionId":"abc"}"#,
        );

        let snapshot = snapshot_from_hook_events(&events, now);

        assert_eq!(snapshot.source, BeaconSnapshotSource::Hooks);
        assert_eq!(snapshot.overall_status, CodexTaskStatus::WaitingApproval);
        assert_eq!(snapshot.alert_level, AlertLevel::Strong);
        assert_eq!(snapshot.waiting_count, 1);
    }

    #[test]
    fn stale_hook_events_return_idle_snapshot() {
        let now = DateTime::parse_from_rfc3339("2026-06-08T08:20:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let events = parse_hook_events_jsonl(
            r#"{"schemaVersion":1,"timestamp":"2026-06-08T08:00:00Z","event":"PreToolUse","summary":"Starting tool"}"#,
        );

        let snapshot = snapshot_from_hook_events(&events, now);

        assert_eq!(snapshot.source, BeaconSnapshotSource::Hooks);
        assert_eq!(snapshot.overall_status, CodexTaskStatus::Idle);
        assert!(snapshot.tasks.is_empty());
    }
}
