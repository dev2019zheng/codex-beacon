use chrono::{DateTime, Utc};
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
    pub overall_status: CodexTaskStatus,
    pub alert_level: AlertLevel,
    pub active_count: usize,
    pub waiting_count: usize,
    pub tasks: Vec<CodexTaskSnapshot>,
    pub themes: Vec<ThemeDescriptor>,
    pub updated_at: DateTime<Utc>,
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

    snapshot_for_status(status, now)
}

pub fn snapshot_for_status(status: CodexTaskStatus, now: DateTime<Utc>) -> BeaconSnapshot {
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

    let overall_status = tasks
        .iter()
        .map(|task| task.status.clone())
        .max_by_key(status_priority)
        .unwrap_or(status);
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
        overall_status,
        alert_level,
        active_count,
        waiting_count,
        tasks,
        themes: builtin_themes(),
        updated_at: now,
    }
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
}
