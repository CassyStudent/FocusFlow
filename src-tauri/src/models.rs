use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub course: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: String,
    pub task_id: Option<String>,
    pub activity: Option<String>,
    pub task_title_snapshot: Option<String>,
    pub course_snapshot: Option<String>,
    pub planned_seconds: i64,
    pub actual_seconds: i64,
    pub status: String,
    pub completion_type: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub event_offset: String,
    pub local_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub focus_minutes: i64,
    pub break_minutes: i64,
    pub sound_enabled: bool,
    pub vault_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTimer {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub task_id: Option<String>,
    pub activity: Option<String>,
    pub task_title_snapshot: Option<String>,
    pub course_snapshot: Option<String>,
    pub planned_seconds: i64,
    pub accumulated_seconds: i64,
    pub remaining_seconds: i64,
    pub started_at: String,
    pub last_resumed_at: Option<String>,
    pub event_offset: String,
    pub local_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutboxEvent {
    pub event_id: String,
    pub category: String,
    pub local_date: String,
    pub markdown_body: String,
    pub created_at: String,
    pub status: String,
    pub delivered_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub tasks: Vec<Task>,
    pub history: Vec<FocusSession>,
    pub settings: Settings,
    pub active_timer: Option<ActiveTimer>,
    pub pending_outbox_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMeta {
    pub event_id: String,
    pub occurred_at: String,
    pub event_offset: String,
    pub local_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub id: String,
    pub title: String,
    pub course: Option<String>,
    pub event: EventMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: String,
    pub course: Option<String>,
    pub event: EventMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEventInput {
    pub id: String,
    pub event: EventMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartFocusInput {
    pub id: String,
    pub task_id: Option<String>,
    pub activity: Option<String>,
    pub planned_seconds: i64,
    pub started_at: String,
    pub event_offset: String,
    pub local_date: String,
    pub event: EventMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerTickInput {
    pub at: String,
    pub accumulated_seconds: i64,
    pub remaining_seconds: i64,
    pub event: Option<EventMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishFocusInput {
    pub ended_at: String,
    pub actual_seconds: i64,
    pub completion_type: String,
    pub event: EventMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelFocusInput {
    pub ended_at: String,
    pub actual_seconds: i64,
    pub event: EventMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartBreakInput {
    pub id: String,
    pub planned_seconds: i64,
    pub started_at: String,
    pub event_offset: String,
    pub local_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsInput {
    pub focus_minutes: i64,
    pub break_minutes: i64,
    pub sound_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureVaultInput {
    pub path: String,
    pub allow_pending_redirection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultValidation {
    pub path: String,
    pub obsidian_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetryOutboxResult {
    pub delivered: usize,
    pub already_present: usize,
    pub failed: usize,
}
