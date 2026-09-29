use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[cfg(windows)]
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

use chrono::{DateTime, FixedOffset, NaiveDate, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::models::{
    ActiveTimer, AppSnapshot, CancelFocusInput, ConfigureVaultInput, CreateTaskInput, EventMeta,
    FinishFocusInput, FocusSession, OutboxEvent, RetryOutboxResult, SaveSettingsInput, Settings,
    StartBreakInput, StartFocusInput, Task, TaskEventInput, TimerTickInput, UpdateTaskInput,
    VaultValidation,
};

const MIGRATION_0001: &str = include_str!("../migrations/0001_focusflow.sql");

pub struct Backend {
    connection: Mutex<Connection>,
}

impl Backend {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(db_error)?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, String> {
        Self::from_connection(Connection::open_in_memory().map_err(db_error)?)
    }

    fn from_connection(connection: Connection) -> Result<Self, String> {
        connection
            .execute_batch(MIGRATION_0001)
            .map_err(|error| format!("Could not initialize FocusFlow database: {error}"))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn get_app_snapshot(&self) -> Result<AppSnapshot, String> {
        let connection = self.connection.lock().map_err(lock_error)?;
        let mut task_statement = connection
            .prepare("SELECT id, title, course, status, created_at, updated_at, completed_at FROM tasks ORDER BY created_at DESC, id DESC")
            .map_err(db_error)?;
        let tasks = task_statement
            .query_map([], map_task)
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;

        let mut history_statement = connection
            .prepare("SELECT id, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, actual_seconds, status, completion_type, started_at, ended_at, event_offset, local_date FROM sessions ORDER BY ended_at DESC, id DESC")
            .map_err(db_error)?;
        let history = history_statement
            .query_map([], map_session)
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;

        let settings = connection
            .query_row(
                "SELECT focus_minutes, break_minutes, sound_enabled, vault_path FROM settings WHERE singleton = 1",
                [],
                |row| {
                    Ok(Settings {
                        focus_minutes: row.get(0)?,
                        break_minutes: row.get(1)?,
                        sound_enabled: row.get::<_, i64>(2)? != 0,
                        vault_path: row.get(3)?,
                    })
                },
            )
            .map_err(db_error)?;

        let active_timer = connection
            .query_row(
                "SELECT id, kind, state, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, accumulated_seconds, remaining_seconds, started_at, last_resumed_at, event_offset, local_date FROM active_timer WHERE singleton = 1",
                [],
                map_active_timer,
            )
            .optional()
            .map_err(db_error)?;
        let pending_outbox_count = connection
            .query_row(
                "SELECT COUNT(*) FROM outbox WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(db_error)?;

        Ok(AppSnapshot {
            tasks,
            history,
            settings,
            active_timer,
            pending_outbox_count,
        })
    }

    pub fn create_task(&self, input: CreateTaskInput) -> Result<Task, String> {
        let title = validate_title(&input.title)?;
        let course = validate_course(input.course.as_deref())?;
        validate_id("Task ID", &input.id)?;
        let event = normalize_event(input.event)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        reserve_id(&transaction, "task", &input.id, &event.occurred_at)?;
        transaction
            .execute(
                "INSERT INTO tasks(id, title, course, status, created_at, updated_at, completed_at) VALUES (?1, ?2, ?3, 'active', ?4, ?4, NULL)",
                params![input.id, title, course, event.occurred_at],
            )
            .map_err(|error| format!("Could not create task: {error}"))?;
        let task = load_task(&transaction, &input.id)?;
        enqueue_task_event(&transaction, &event, "task_created", &task)?;
        transaction.commit().map_err(db_error)?;
        Ok(task)
    }

    pub fn update_task(&self, input: UpdateTaskInput) -> Result<Task, String> {
        let title = validate_title(&input.title)?;
        let course = validate_course(input.course.as_deref())?;
        validate_id("Task ID", &input.id)?;
        let event = normalize_event(input.event)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        ensure_task_mutable(&transaction, &input.id, "edit", false)?;
        let previous = load_task(&transaction, &input.id)?;
        transaction
            .execute(
                "UPDATE tasks SET title = ?1, course = ?2, updated_at = ?3 WHERE id = ?4",
                params![title, course, event.occurred_at, input.id],
            )
            .map_err(db_error)?;
        let task = load_task(&transaction, &input.id)?;
        enqueue_task_edit_event(&transaction, &event, &previous, &task)?;
        transaction.commit().map_err(db_error)?;
        Ok(task)
    }

    pub fn complete_task(&self, input: TaskEventInput) -> Result<Task, String> {
        self.change_task_status(input, "completed", "task_completed")
    }

    pub fn reopen_task(&self, input: TaskEventInput) -> Result<Task, String> {
        validate_id("Task ID", &input.id)?;
        let event = normalize_event(input.event)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let current = load_task(&transaction, &input.id)?;
        if current.status != "completed" {
            return Err("Only a completed task can be reopened".to_string());
        }
        transaction
            .execute(
                "UPDATE tasks SET status = 'active', completed_at = NULL, updated_at = ?1 WHERE id = ?2",
                params![event.occurred_at, input.id],
            )
            .map_err(db_error)?;
        let task = load_task(&transaction, &input.id)?;
        enqueue_task_event(&transaction, &event, "task_reopened", &task)?;
        transaction.commit().map_err(db_error)?;
        Ok(task)
    }

    pub fn delete_task(&self, input: TaskEventInput) -> Result<(), String> {
        validate_id("Task ID", &input.id)?;
        let event = normalize_event(input.event)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        ensure_task_mutable(&transaction, &input.id, "delete", true)?;
        let task = load_task(&transaction, &input.id)?;
        enqueue_task_event(&transaction, &event, "task_deleted", &task)?;
        transaction
            .execute("DELETE FROM tasks WHERE id = ?1", params![input.id])
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(())
    }

    fn change_task_status(
        &self,
        input: TaskEventInput,
        status: &str,
        event_type: &str,
    ) -> Result<Task, String> {
        validate_id("Task ID", &input.id)?;
        let event = normalize_event(input.event)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        ensure_task_mutable(&transaction, &input.id, "complete", false)?;
        transaction
            .execute(
                "UPDATE tasks SET status = ?1, completed_at = ?2, updated_at = ?2 WHERE id = ?3",
                params![status, event.occurred_at, input.id],
            )
            .map_err(db_error)?;
        let task = load_task(&transaction, &input.id)?;
        enqueue_task_event(&transaction, &event, event_type, &task)?;
        transaction.commit().map_err(db_error)?;
        Ok(task)
    }

    pub fn start_focus(&self, input: StartFocusInput) -> Result<ActiveTimer, String> {
        validate_id("Session ID", &input.id)?;
        if !(1..=10_800).contains(&input.planned_seconds) {
            return Err("Focus duration must be between 1 second and 180 minutes".to_string());
        }
        let activity = input
            .activity
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if activity
            .as_deref()
            .is_some_and(|value| value.chars().count() > 120)
        {
            return Err("Activity must be 120 characters or fewer".to_string());
        }
        if input.task_id.is_some() == activity.is_some() {
            return Err("Choose exactly one task or activity for a focus session".to_string());
        }
        if let Some(task_id) = input.task_id.as_deref() {
            validate_id("Task ID", task_id)?;
        }
        let started_at =
            normalize_time_context(&input.started_at, &input.event_offset, &input.local_date)?;
        let event = normalize_event(input.event)?;
        ensure_matching_operation_event(
            &started_at,
            &input.event_offset,
            &input.local_date,
            &event,
        )?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        ensure_no_active_timer(&transaction)?;
        reserve_id(&transaction, "timer", &input.id, &started_at)?;
        let (title_snapshot, course_snapshot) = if let Some(task_id) = &input.task_id {
            let task = load_task(&transaction, task_id)?;
            if task.status == "completed" {
                return Err("reopen the completed task before focusing on it".to_string());
            }
            (Some(task.title), task.course)
        } else {
            (None, None)
        };
        transaction
            .execute(
                "INSERT INTO active_timer(singleton, id, kind, state, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, accumulated_seconds, remaining_seconds, started_at, last_resumed_at, state_changed_at, event_offset, local_date) VALUES (1, ?1, 'focus', 'running', ?2, ?3, ?4, ?5, ?6, 0, ?6, ?7, ?7, ?7, ?8, ?9)",
                params![input.id, input.task_id, activity, title_snapshot, course_snapshot, input.planned_seconds, started_at, input.event_offset, input.local_date],
            )
            .map_err(|error| format!("Could not start focus timer: {error}"))?;
        let timer = load_active_timer(&transaction)?;
        enqueue_active_focus_event(&transaction, &event, "focus_started", "running", &timer)?;
        transaction.commit().map_err(db_error)?;
        Ok(timer)
    }

    pub fn pause_focus(&self, input: TimerTickInput) -> Result<ActiveTimer, String> {
        self.transition_timer("focus", "running", "paused", input)
    }

    pub fn resume_focus(&self, input: TimerTickInput) -> Result<ActiveTimer, String> {
        self.transition_timer("focus", "paused", "running", input)
    }

    pub fn finish_focus(&self, input: FinishFocusInput) -> Result<FocusSession, String> {
        if !matches!(
            input.completion_type.as_str(),
            "timer_finished" | "completed_early"
        ) {
            return Err("Completion type must be timer_finished or completed_early".to_string());
        }
        self.end_focus(
            input.ended_at,
            input.actual_seconds,
            "completed",
            Some(input.completion_type),
            input.event,
        )
    }

    pub fn cancel_focus(&self, input: CancelFocusInput) -> Result<FocusSession, String> {
        self.end_focus(
            input.ended_at,
            input.actual_seconds,
            "cancelled",
            None,
            input.event,
        )
    }

    pub fn start_break(&self, input: StartBreakInput) -> Result<ActiveTimer, String> {
        validate_id("Break ID", &input.id)?;
        let started_at =
            normalize_time_context(&input.started_at, &input.event_offset, &input.local_date)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        ensure_no_active_timer(&transaction)?;
        reserve_id(&transaction, "timer", &input.id, &started_at)?;
        let planned_seconds = load_settings(&transaction)?.break_minutes * 60;
        transaction
            .execute(
                "INSERT INTO active_timer(singleton, id, kind, state, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, accumulated_seconds, remaining_seconds, started_at, last_resumed_at, state_changed_at, event_offset, local_date) VALUES (1, ?1, 'break', 'running', NULL, NULL, NULL, NULL, ?2, 0, ?2, ?3, ?3, ?3, ?4, ?5)",
                params![input.id, planned_seconds, started_at, input.event_offset, input.local_date],
            )
            .map_err(|error| format!("Could not start break timer: {error}"))?;
        let timer = load_active_timer(&transaction)?;
        transaction.commit().map_err(db_error)?;
        Ok(timer)
    }

    pub fn pause_break(&self, input: TimerTickInput) -> Result<ActiveTimer, String> {
        self.transition_timer("break", "running", "paused", input)
    }

    pub fn resume_break(&self, input: TimerTickInput) -> Result<ActiveTimer, String> {
        self.transition_timer("break", "paused", "running", input)
    }

    pub fn cancel_break(&self) -> Result<(), String> {
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let timer = load_active_timer(&transaction)?;
        if timer.kind != "break" {
            return Err("The active timer is not a break".to_string());
        }
        transaction
            .execute("DELETE FROM active_timer WHERE singleton = 1", [])
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(())
    }

    pub fn recover_active_timer(&self) -> Result<Option<ActiveTimer>, String> {
        let connection = self.connection.lock().map_err(lock_error)?;
        connection
            .query_row(
                "SELECT id, kind, state, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, accumulated_seconds, remaining_seconds, started_at, last_resumed_at, event_offset, local_date FROM active_timer WHERE singleton = 1",
                [],
                map_active_timer,
            )
            .optional()
            .map_err(db_error)
    }

    fn transition_timer(
        &self,
        kind: &str,
        from_state: &str,
        to_state: &str,
        input: TimerTickInput,
    ) -> Result<ActiveTimer, String> {
        let at = canonical_utc_timestamp(&input.at)?;
        let event = input.event.map(normalize_event).transpose()?;
        if let Some(event) = event.as_ref()
            && event.occurred_at != at
        {
            return Err("Timer operation timestamp must match its event timestamp".to_string());
        }

        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let timer = load_active_timer(&transaction)?;
        if timer.kind != kind || timer.state != from_state {
            return Err(format!("Expected a {from_state} {kind} timer"));
        }
        let state_changed_at: String = transaction
            .query_row(
                "SELECT state_changed_at FROM active_timer WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        ensure_not_backwards(&state_changed_at, &at)?;

        let accumulated_seconds = if from_state == "running" {
            let resumed_at = timer
                .last_resumed_at
                .as_deref()
                .ok_or_else(|| "Running timer is missing its resume timestamp".to_string())?;
            (timer.accumulated_seconds + elapsed_seconds(resumed_at, &at)?)
                .min(timer.planned_seconds)
        } else {
            timer.accumulated_seconds
        };
        if to_state == "paused" && accumulated_seconds >= timer.planned_seconds {
            return Err("Expired timers must be completed instead of paused".to_string());
        }
        let remaining_seconds = (timer.planned_seconds - accumulated_seconds).max(0);
        let last_resumed_at = if to_state == "running" {
            Some(at.as_str())
        } else {
            timer.last_resumed_at.as_deref()
        };
        transaction
            .execute(
                "UPDATE active_timer SET state = ?1, accumulated_seconds = ?2, remaining_seconds = ?3, last_resumed_at = ?4, state_changed_at = ?5 WHERE singleton = 1",
                params![to_state, accumulated_seconds, remaining_seconds, last_resumed_at, at],
            )
            .map_err(db_error)?;
        let timer = load_active_timer(&transaction)?;
        transaction.commit().map_err(db_error)?;
        Ok(timer)
    }

    fn end_focus(
        &self,
        ended_at: String,
        _actual_seconds: i64,
        status: &str,
        completion_type: Option<String>,
        event: EventMeta,
    ) -> Result<FocusSession, String> {
        let ended_at = canonical_utc_timestamp(&ended_at)?;
        let event = normalize_event(event)?;
        if event.occurred_at != ended_at {
            return Err("Focus end timestamp must match its event timestamp".to_string());
        }
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let timer = load_active_timer(&transaction)?;
        if timer.kind != "focus" {
            return Err("The active timer is not a focus session".to_string());
        }
        let state_changed_at: String = transaction
            .query_row(
                "SELECT state_changed_at FROM active_timer WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        ensure_not_backwards(&state_changed_at, &ended_at)?;
        let actual_seconds = if timer.state == "running" {
            let resumed_at = timer
                .last_resumed_at
                .as_deref()
                .ok_or_else(|| "Running timer is missing its resume timestamp".to_string())?;
            (timer.accumulated_seconds + elapsed_seconds(resumed_at, &ended_at)?)
                .min(timer.planned_seconds)
        } else {
            timer.accumulated_seconds
        };
        if completion_type.as_deref() == Some("timer_finished")
            && actual_seconds < timer.planned_seconds
        {
            return Err(
                "A timer_finished event cannot occur before the planned duration".to_string(),
            );
        }
        transaction
            .execute(
                "INSERT INTO sessions(id, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, actual_seconds, status, completion_type, started_at, ended_at, event_offset, local_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![timer.id, timer.task_id, timer.activity, timer.task_title_snapshot, timer.course_snapshot, timer.planned_seconds, actual_seconds, status, completion_type, timer.started_at, ended_at, event.event_offset, event.local_date],
            )
            .map_err(|error| format!("Could not save focus history: {error}"))?;
        let session = transaction
            .query_row(
                "SELECT id, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, actual_seconds, status, completion_type, started_at, ended_at, event_offset, local_date FROM sessions WHERE id = ?1",
                params![timer.id],
                map_session,
            )
            .map_err(db_error)?;
        enqueue_session_event(&transaction, &event, &session)?;
        transaction
            .execute("DELETE FROM active_timer WHERE singleton = 1", [])
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(session)
    }

    pub fn save_settings(&self, input: SaveSettingsInput) -> Result<Settings, String> {
        if input.focus_minutes <= 0 || input.break_minutes <= 0 {
            return Err("Focus and break minutes must be greater than zero".to_string());
        }
        if input.focus_minutes > 180 || input.break_minutes > 60 {
            return Err(
                "Focus duration must be 1 to 180 minutes and break duration 1 to 60 minutes"
                    .to_string(),
            );
        }
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        transaction
            .execute(
                "UPDATE settings SET focus_minutes = ?1, break_minutes = ?2, sound_enabled = ?3 WHERE singleton = 1",
                params![input.focus_minutes, input.break_minutes, i64::from(input.sound_enabled)],
            )
            .map_err(db_error)?;
        let settings = load_settings(&transaction)?;
        transaction.commit().map_err(db_error)?;
        Ok(settings)
    }

    pub fn validate_vault(&self, path: String) -> Result<VaultValidation, String> {
        validate_vault_path(&path)
    }

    pub fn configure_vault(&self, input: ConfigureVaultInput) -> Result<VaultValidation, String> {
        let validation = validate_vault_path(&input.path)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let active_count: i64 = transaction
            .query_row("SELECT COUNT(*) FROM active_timer", [], |row| row.get(0))
            .map_err(db_error)?;
        if active_count > 0 {
            return Err("Vault cannot be changed while a timer is running or paused".to_string());
        }
        let settings = load_settings(&transaction)?;
        let pending_count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM outbox WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        if settings.vault_path.is_some()
            && settings.vault_path.as_deref() != Some(validation.path.as_str())
            && pending_count > 0
            && !input.allow_pending_redirection
        {
            return Err("Pending events would be redirected; explicitly allow pending redirection to change the vault".to_string());
        }
        transaction
            .execute(
                "UPDATE settings SET vault_path = ?1 WHERE singleton = 1",
                params![validation.path],
            )
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(validation)
    }

    pub fn retry_outbox(&self, delivered_at: &str) -> Result<RetryOutboxResult, String> {
        let delivered_at = canonical_utc_timestamp(delivered_at)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let vault_path: Option<String> = connection
            .query_row(
                "SELECT vault_path FROM settings WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        let vault_path = vault_path.ok_or_else(|| {
            "Configure an Obsidian vault before retrying pending events".to_string()
        })?;
        let vault_root = fs::canonicalize(&vault_path)
            .map_err(|error| format!("Configured vault is unavailable: {error}"))?;
        let events = {
            let mut statement = connection
                .prepare("SELECT event_id, category, local_date, markdown_body, created_at, status, delivered_at, last_error FROM outbox WHERE status = 'pending' ORDER BY created_at ASC, event_id ASC")
                .map_err(db_error)?;
            statement
                .query_map([], map_outbox_event)
                .map_err(db_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db_error)?
        };
        let mut result = RetryOutboxResult {
            delivered: 0,
            already_present: 0,
            failed: 0,
        };
        for event in events {
            match deliver_event(&vault_root, &event) {
                Ok(already_present) => {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(db_error)?;
                    transaction
                        .execute(
                            "UPDATE outbox SET status = 'delivered', delivered_at = ?1, last_error = NULL WHERE event_id = ?2",
                            params![delivered_at, event.event_id],
                        )
                        .map_err(db_error)?;
                    transaction.commit().map_err(db_error)?;
                    if already_present {
                        result.already_present += 1;
                    } else {
                        result.delivered += 1;
                    }
                }
                Err(error) => {
                    connection
                        .execute(
                            "UPDATE outbox SET last_error = ?1 WHERE event_id = ?2",
                            params![error, event.event_id],
                        )
                        .map_err(db_error)?;
                    result.failed += 1;
                    break;
                }
            }
        }
        Ok(result)
    }
}

fn load_settings(connection: &Connection) -> Result<Settings, String> {
    connection
        .query_row(
            "SELECT focus_minutes, break_minutes, sound_enabled, vault_path FROM settings WHERE singleton = 1",
            [],
            |row| {
                Ok(Settings {
                    focus_minutes: row.get(0)?,
                    break_minutes: row.get(1)?,
                    sound_enabled: row.get::<_, i64>(2)? != 0,
                    vault_path: row.get(3)?,
                })
            },
        )
        .map_err(db_error)
}

fn validate_vault_path(path: &str) -> Result<VaultValidation, String> {
    let root = PathBuf::from(path);
    if !root.is_dir() {
        return Err("Selected vault folder does not exist or is not a directory".to_string());
    }
    let root = fs::canonicalize(&root)
        .map_err(|error| format!("Could not resolve selected vault folder: {error}"))?;
    let probe = root.join(format!(".focusflow-write-test-{}", std::process::id()));
    let write_result = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .and_then(|mut file| file.write_all(b"FocusFlow write test"));
    let cleanup_result = if probe.exists() {
        fs::remove_file(&probe)
    } else {
        Ok(())
    };
    write_result.map_err(|error| format!("Selected vault folder is not writable: {error}"))?;
    cleanup_result.map_err(|error| format!("Could not clean up vault write test: {error}"))?;

    let focusflow = root.join("FocusFlow");
    fs::create_dir_all(&focusflow)
        .map_err(|error| format!("Could not create FocusFlow output folder: {error}"))?;
    let focusflow = fs::canonicalize(&focusflow)
        .map_err(|error| format!("Could not resolve FocusFlow output folder: {error}"))?;
    if !focusflow.starts_with(&root) {
        return Err("FocusFlow output folder must remain inside the selected vault".to_string());
    }
    for name in ["Tasks", "Sessions"] {
        let directory = focusflow.join(name);
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Could not create FocusFlow/{name}: {error}"))?;
        let resolved = fs::canonicalize(&directory)
            .map_err(|error| format!("Could not resolve FocusFlow/{name}: {error}"))?;
        if !resolved.starts_with(&focusflow) {
            return Err(
                "FocusFlow output folders must remain inside the selected vault".to_string(),
            );
        }
    }
    Ok(VaultValidation {
        path: root.to_string_lossy().into_owned(),
        obsidian_warning: !root.join(".obsidian").is_dir(),
    })
}

fn deliver_event(vault_root: &Path, event: &OutboxEvent) -> Result<bool, String> {
    let category_folder = match event.category.as_str() {
        "tasks" => "Tasks",
        "sessions" => "Sessions",
        _ => return Err(format!("Unsupported outbox category: {}", event.category)),
    };
    if event.local_date.contains(['/', '\\']) || event.local_date.contains("..") {
        return Err("Outbox local date is not safe for a filename".to_string());
    }
    let focusflow = vault_root.join("FocusFlow");
    reject_link_or_reparse(&focusflow, "FocusFlow output folder")?;
    let focusflow = fs::canonicalize(&focusflow)
        .map_err(|error| format!("Could not resolve FocusFlow output folder: {error}"))?;
    if !focusflow.starts_with(vault_root) {
        return Err("FocusFlow output folder must remain inside the selected vault".to_string());
    }
    let directory = focusflow.join(category_folder);
    reject_link_or_reparse(&directory, "Outbox target folder")?;
    let directory = fs::canonicalize(&directory)
        .map_err(|error| format!("Could not resolve outbox target folder: {error}"))?;
    if !directory.starts_with(&focusflow) {
        return Err("Outbox target must remain inside the selected vault".to_string());
    }
    let target = directory.join(format!("{}.md", event.local_date));
    let existing = match fs::symlink_metadata(&target) {
        Ok(_) => {
            reject_link_or_reparse(&target, "Outbox daily file")?;
            fs::read_to_string(&target)
                .map_err(|error| format!("Could not read outbox target: {error}"))?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("Could not inspect outbox target: {error}")),
    };
    let marker = format!("Event ID: {}", event.event_id);
    if existing
        .lines()
        .any(|line| line.trim().trim_start_matches("- ") == marker)
    {
        return Ok(true);
    }
    let mut file = open_append_no_follow(&target)?;
    file.write_all(event.markdown_body.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Could not append outbox event: {error}"))?;
    Ok(false)
}

fn reject_link_or_reparse(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Could not inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} cannot be a symbolic link"));
    }
    #[cfg(windows)]
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(format!("{label} cannot be a Windows reparse point"));
    }
    Ok(())
}

fn open_append_no_follow(path: &Path) -> Result<std::fs::File, String> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(windows)]
    options.custom_flags(0x0020_0000);
    options
        .open(path)
        .map_err(|error| format!("Could not open outbox target for append: {error}"))
}

fn ensure_no_active_timer(transaction: &Transaction<'_>) -> Result<(), String> {
    let count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM active_timer", [], |row| row.get(0))
        .map_err(db_error)?;
    if count > 0 {
        return Err("Finish or cancel the current timer before starting another".to_string());
    }
    Ok(())
}

fn load_active_timer(transaction: &Transaction<'_>) -> Result<ActiveTimer, String> {
    transaction
        .query_row(
            "SELECT id, kind, state, task_id, activity, task_title_snapshot, course_snapshot, planned_seconds, accumulated_seconds, remaining_seconds, started_at, last_resumed_at, event_offset, local_date FROM active_timer WHERE singleton = 1",
            [],
            map_active_timer,
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| "There is no active timer".to_string())
}

fn enqueue_active_focus_event(
    transaction: &Transaction<'_>,
    event: &EventMeta,
    event_type: &str,
    status: &str,
    timer: &ActiveTimer,
) -> Result<(), String> {
    let subject = timer
        .task_title_snapshot
        .as_deref()
        .or(timer.activity.as_deref())
        .unwrap_or("Unknown");
    let markdown = format!(
        "\n## Focus: {}\n- Event ID: {}\n- Timestamp: {} {}\n- Event type: {}\n- Status: {}\n- Session ID: {}\n- Task ID: {}\n- Task title: {}\n- Course/Subject: {}\n- Activity: {}\n- Planned seconds: {}\n- Focused seconds: {}\n- Remaining seconds: {}\n",
        markdown_text(subject),
        event.event_id,
        event.occurred_at,
        event.event_offset,
        event_type,
        status,
        timer.id,
        timer.task_id.as_deref().unwrap_or("None"),
        timer
            .task_title_snapshot
            .as_deref()
            .map(markdown_text)
            .unwrap_or_else(|| "None".to_string()),
        timer
            .course_snapshot
            .as_deref()
            .map(markdown_text)
            .unwrap_or_else(|| "None".to_string()),
        timer
            .activity
            .as_deref()
            .map(markdown_text)
            .unwrap_or_else(|| "None".to_string()),
        timer.planned_seconds,
        timer.accumulated_seconds,
        timer.remaining_seconds,
    );
    transaction
        .execute(
            "INSERT INTO outbox(event_id, category, local_date, markdown_body, created_at, status, delivered_at, last_error) VALUES (?1, 'sessions', ?2, ?3, ?4, 'pending', NULL, NULL)",
            params![event.event_id, event.local_date, markdown, event.occurred_at],
        )
        .map_err(|error| format!("Could not record focus event atomically: {error}"))?;
    Ok(())
}

fn enqueue_session_event(
    transaction: &Transaction<'_>,
    event: &EventMeta,
    session: &FocusSession,
) -> Result<(), String> {
    let subject = session
        .task_title_snapshot
        .as_deref()
        .or(session.activity.as_deref())
        .unwrap_or("Unknown");
    let event_type = if session.status == "cancelled" {
        "focus_cancelled"
    } else {
        "focus_completed"
    };
    let markdown = format!(
        "\n## Focus: {}\n- Event ID: {}\n- Timestamp: {} {}\n- Event type: {}\n- Status: {}\n- Session ID: {}\n- Task ID: {}\n- Task title: {}\n- Course/Subject: {}\n- Activity: {}\n- Planned seconds: {}\n- Actual seconds: {}\n- Completion type: {}\n",
        markdown_text(subject),
        event.event_id,
        event.occurred_at,
        event.event_offset,
        event_type,
        session.status,
        session.id,
        session.task_id.as_deref().unwrap_or("None"),
        session
            .task_title_snapshot
            .as_deref()
            .map(markdown_text)
            .unwrap_or_else(|| "None".to_string()),
        session
            .course_snapshot
            .as_deref()
            .map(markdown_text)
            .unwrap_or_else(|| "None".to_string()),
        session
            .activity
            .as_deref()
            .map(markdown_text)
            .unwrap_or_else(|| "None".to_string()),
        session.planned_seconds,
        session.actual_seconds,
        session.completion_type.as_deref().unwrap_or("None")
    );
    transaction
        .execute(
            "INSERT INTO outbox(event_id, category, local_date, markdown_body, created_at, status, delivered_at, last_error) VALUES (?1, 'sessions', ?2, ?3, ?4, 'pending', NULL, NULL)",
            params![event.event_id, event.local_date, markdown, event.occurred_at],
        )
        .map_err(|error| format!("Could not record session event atomically: {error}"))?;
    Ok(())
}

fn validate_title(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Task title is required".to_string());
    }
    if value.chars().count() > 120 {
        return Err("Task title must be 120 characters or fewer".to_string());
    }
    Ok(value.to_string())
}

fn validate_course(value: Option<&str>) -> Result<Option<String>, String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if value.is_some_and(|course| course.chars().count() > 60) {
        return Err("Course must be 60 characters or fewer".to_string());
    }
    Ok(value.map(str::to_string))
}

fn validate_id(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
    {
        return Err(format!(
            "{label} must be 1 to 128 characters using only letters, numbers, '.', '_', '-', or ':'"
        ));
    }
    Ok(())
}

fn reserve_id(
    transaction: &Transaction<'_>,
    kind: &str,
    id: &str,
    created_at: &str,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO used_ids(kind, id, created_at) VALUES (?1, ?2, ?3)",
            params![kind, id, created_at],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE constraint failed") {
                format!("{kind} ID '{id}' has already been used")
            } else {
                db_error(error)
            }
        })?;
    Ok(())
}

fn canonical_utc_timestamp(value: &str) -> Result<String, String> {
    let timestamp = DateTime::parse_from_rfc3339(value)
        .map_err(|_| "Timestamp must be a valid UTC ISO 8601 value".to_string())?;
    if timestamp.offset().local_minus_utc() != 0 {
        return Err("Timestamp must be stored in UTC".to_string());
    }
    Ok(timestamp
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Nanos, true))
}

fn parse_offset(value: &str) -> Result<FixedOffset, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 6
        || !matches!(bytes[0], b'+' | b'-')
        || bytes[3] != b':'
        || !bytes[1..3].iter().all(u8::is_ascii_digit)
        || !bytes[4..6].iter().all(u8::is_ascii_digit)
    {
        return Err("Event offset must use +HH:MM or -HH:MM".to_string());
    }
    let hours = value[1..3].parse::<i32>().unwrap_or(24);
    let minutes = value[4..6].parse::<i32>().unwrap_or(60);
    if hours > 23 || minutes > 59 {
        return Err("Event offset must use a valid +HH:MM or -HH:MM value".to_string());
    }
    let seconds = (hours * 60 + minutes) * 60;
    let signed = if bytes[0] == b'-' { -seconds } else { seconds };
    FixedOffset::east_opt(signed)
        .ok_or_else(|| "Event offset must use a valid +HH:MM or -HH:MM value".to_string())
}

fn normalize_time_context(
    timestamp: &str,
    offset: &str,
    local_date: &str,
) -> Result<String, String> {
    let canonical = canonical_utc_timestamp(timestamp)?;
    let fixed_offset = parse_offset(offset)?;
    let supplied_date = NaiveDate::parse_from_str(local_date, "%Y-%m-%d")
        .map_err(|_| "Local date must use YYYY-MM-DD".to_string())?;
    let utc = DateTime::parse_from_rfc3339(&canonical)
        .map_err(|_| "Timestamp must be a valid UTC ISO 8601 value".to_string())?;
    let calculated_date = utc.with_timezone(&fixed_offset).date_naive();
    if supplied_date != calculated_date {
        return Err("Event local date must match its timestamp and numeric UTC offset".to_string());
    }
    Ok(canonical)
}

fn normalize_event(mut event: EventMeta) -> Result<EventMeta, String> {
    validate_id("Event ID", &event.event_id)?;
    event.occurred_at =
        normalize_time_context(&event.occurred_at, &event.event_offset, &event.local_date)?;
    Ok(event)
}

fn ensure_matching_operation_event(
    operation_at: &str,
    offset: &str,
    local_date: &str,
    event: &EventMeta,
) -> Result<(), String> {
    if event.occurred_at != operation_at
        || event.event_offset != offset
        || event.local_date != local_date
    {
        return Err("Operation timestamp context must match its event metadata".to_string());
    }
    Ok(())
}

fn ensure_not_backwards(previous: &str, next: &str) -> Result<(), String> {
    let previous = DateTime::parse_from_rfc3339(previous)
        .map_err(|_| "Persisted timer timestamp is invalid".to_string())?;
    let next =
        DateTime::parse_from_rfc3339(next).map_err(|_| "Timer timestamp is invalid".to_string())?;
    if next < previous {
        return Err("Timer operation cannot move backwards in time".to_string());
    }
    Ok(())
}

fn elapsed_seconds(start: &str, end: &str) -> Result<i64, String> {
    let start = DateTime::parse_from_rfc3339(start)
        .map_err(|_| "Persisted timer timestamp is invalid".to_string())?;
    let end =
        DateTime::parse_from_rfc3339(end).map_err(|_| "Timer timestamp is invalid".to_string())?;
    let milliseconds = end.signed_duration_since(start).num_milliseconds();
    if milliseconds < 0 {
        return Err("Timer operation cannot move backwards in time".to_string());
    }
    Ok(milliseconds / 1000)
}

fn load_task(transaction: &Transaction<'_>, id: &str) -> Result<Task, String> {
    transaction
        .query_row(
            "SELECT id, title, course, status, created_at, updated_at, completed_at FROM tasks WHERE id = ?1",
            params![id],
            map_task,
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| format!("Task '{id}' was not found"))
}

fn ensure_task_mutable(
    transaction: &Transaction<'_>,
    id: &str,
    operation: &str,
    allow_completed: bool,
) -> Result<(), String> {
    let task = load_task(transaction, id)?;
    if task.status == "completed" && !allow_completed {
        return Err(format!(
            "reopen this task before attempting to {operation} it"
        ));
    }
    let linked_timer: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM active_timer WHERE singleton = 1 AND kind = 'focus' AND task_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if linked_timer > 0 {
        return Err(format!(
            "Cannot {operation} this task while its focus timer is active"
        ));
    }
    Ok(())
}

fn markdown_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character == '\r'
            || character == '\n'
            || character == '\u{2028}'
            || character == '\u{2029}'
            || character.is_control()
        {
            escaped.push(' ');
        } else {
            if matches!(
                character,
                '\\' | '`'
                    | '*'
                    | '_'
                    | '{'
                    | '}'
                    | '['
                    | ']'
                    | '('
                    | ')'
                    | '#'
                    | '+'
                    | '-'
                    | '!'
                    | '|'
                    | '>'
                    | '<'
                    | '~'
            ) {
                escaped.push('\\');
            }
            escaped.push(character);
        }
    }
    escaped
}

fn task_snapshot_markdown(task: &Task) -> String {
    format!(
        "- Title: {}\n- Course/Subject: {}\n",
        markdown_text(&task.title),
        task.course
            .as_deref()
            .map(markdown_text)
            .unwrap_or_else(|| "None".to_string())
    )
}

fn insert_task_outbox(
    transaction: &Transaction<'_>,
    event: &EventMeta,
    markdown: &str,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO outbox(event_id, category, local_date, markdown_body, created_at, status, delivered_at, last_error) VALUES (?1, 'tasks', ?2, ?3, ?4, 'pending', NULL, NULL)",
            params![event.event_id, event.local_date, markdown, event.occurred_at],
        )
        .map_err(|error| format!("Could not record task event atomically: {error}"))?;
    Ok(())
}

fn enqueue_task_event(
    transaction: &Transaction<'_>,
    event: &EventMeta,
    event_type: &str,
    task: &Task,
) -> Result<(), String> {
    let markdown = format!(
        "\n## Task: {}\n- Event ID: {}\n- Timestamp: {} {}\n- Event type: {}\n- Status: {}\n- Task ID: {}\n{}",
        markdown_text(&task.title),
        event.event_id,
        event.occurred_at,
        event.event_offset,
        event_type,
        task.status,
        task.id,
        task_snapshot_markdown(task),
    );
    insert_task_outbox(transaction, event, &markdown)
}

fn enqueue_task_edit_event(
    transaction: &Transaction<'_>,
    event: &EventMeta,
    previous: &Task,
    task: &Task,
) -> Result<(), String> {
    let mut changes = String::new();
    if previous.title != task.title {
        changes.push_str(&format!(
            "- Previous Title: {}\n- New Title: {}\n",
            markdown_text(&previous.title),
            markdown_text(&task.title)
        ));
    }
    if previous.course != task.course {
        changes.push_str(&format!(
            "- Previous Course/Subject: {}\n- New Course/Subject: {}\n",
            previous
                .course
                .as_deref()
                .map(markdown_text)
                .unwrap_or_else(|| "None".to_string()),
            task.course
                .as_deref()
                .map(markdown_text)
                .unwrap_or_else(|| "None".to_string())
        ));
    }
    if changes.is_empty() {
        changes.push_str("- Changed fields: None\n");
    }
    let markdown = format!(
        "\n## Task edited: {}\n- Event ID: {}\n- Timestamp: {} {}\n- Event type: task_edited\n- Status: {}\n- Task ID: {}\n{}{}",
        markdown_text(&task.title),
        event.event_id,
        event.occurred_at,
        event.event_offset,
        task.status,
        task.id,
        changes,
        task_snapshot_markdown(task),
    );
    insert_task_outbox(transaction, event, &markdown)
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        course: row.get(2)?,
        status: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        completed_at: row.get(6)?,
    })
}

fn map_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusSession> {
    Ok(FocusSession {
        id: row.get(0)?,
        task_id: row.get(1)?,
        activity: row.get(2)?,
        task_title_snapshot: row.get(3)?,
        course_snapshot: row.get(4)?,
        planned_seconds: row.get(5)?,
        actual_seconds: row.get(6)?,
        status: row.get(7)?,
        completion_type: row.get(8)?,
        started_at: row.get(9)?,
        ended_at: row.get(10)?,
        event_offset: row.get(11)?,
        local_date: row.get(12)?,
    })
}

fn map_active_timer(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActiveTimer> {
    Ok(ActiveTimer {
        id: row.get(0)?,
        kind: row.get(1)?,
        state: row.get(2)?,
        task_id: row.get(3)?,
        activity: row.get(4)?,
        task_title_snapshot: row.get(5)?,
        course_snapshot: row.get(6)?,
        planned_seconds: row.get(7)?,
        accumulated_seconds: row.get(8)?,
        remaining_seconds: row.get(9)?,
        started_at: row.get(10)?,
        last_resumed_at: row.get(11)?,
        event_offset: row.get(12)?,
        local_date: row.get(13)?,
    })
}

fn map_outbox_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<OutboxEvent> {
    Ok(OutboxEvent {
        event_id: row.get(0)?,
        category: row.get(1)?,
        local_date: row.get(2)?,
        markdown_body: row.get(3)?,
        created_at: row.get(4)?,
        status: row.get(5)?,
        delivered_at: row.get(6)?,
        last_error: row.get(7)?,
    })
}

fn db_error(error: rusqlite::Error) -> String {
    format!("FocusFlow database error: {error}")
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> String {
    "FocusFlow database is busy because a previous operation failed".to_string()
}

#[cfg(test)]
mod tests {
    use super::Backend;
    use crate::models::{
        CancelFocusInput, ConfigureVaultInput, CreateTaskInput, EventMeta, FinishFocusInput,
        SaveSettingsInput, StartBreakInput, StartFocusInput, TaskEventInput, TimerTickInput,
        UpdateTaskInput,
    };

    fn event(id: &str, at: &str) -> EventMeta {
        EventMeta {
            event_id: id.to_string(),
            occurred_at: at.to_string(),
            event_offset: "+02:00".to_string(),
            local_date: "2026-09-28".to_string(),
        }
    }

    fn create_task(backend: &Backend) {
        backend
            .create_task(CreateTaskInput {
                id: "task-1".to_string(),
                title: "  Read chapter 4  ".to_string(),
                course: Some("  Biology  ".to_string()),
                event: event("event-created", "2026-09-28T08:00:00Z"),
            })
            .unwrap();
    }

    #[test]
    fn new_database_has_default_settings() {
        let backend = Backend::open_in_memory().unwrap();
        let snapshot = backend.get_app_snapshot().unwrap();

        assert_eq!(snapshot.settings.focus_minutes, 25);
        assert_eq!(snapshot.settings.break_minutes, 5);
        assert!(snapshot.settings.sound_enabled);
        assert!(snapshot.tasks.is_empty());
        assert!(snapshot.history.is_empty());
        assert!(snapshot.active_timer.is_none());
    }

    #[test]
    fn task_lifecycle_is_validated_and_creates_transactional_outbox_events() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);

        let created = backend.get_app_snapshot().unwrap();
        assert_eq!(created.tasks[0].title, "Read chapter 4");
        assert_eq!(created.tasks[0].course.as_deref(), Some("Biology"));
        assert_eq!(created.pending_outbox_count, 1);

        backend
            .update_task(UpdateTaskInput {
                id: "task-1".to_string(),
                title: "Review chapter 4".to_string(),
                course: None,
                event: event("event-edited", "2026-09-28T09:00:00Z"),
            })
            .unwrap();
        let edited_markdown: String = backend
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT markdown_body FROM outbox WHERE event_id = 'event-edited'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(edited_markdown.contains("Event type: task_edited"));
        assert!(edited_markdown.contains("Previous Title: Read chapter 4"));
        assert!(edited_markdown.contains("New Title: Review chapter 4"));
        assert!(edited_markdown.contains("Previous Course/Subject: Biology"));
        assert!(edited_markdown.contains("New Course/Subject: None"));
        backend
            .complete_task(TaskEventInput {
                id: "task-1".to_string(),
                event: event("event-completed", "2026-09-28T10:00:00Z"),
            })
            .unwrap();

        let completed = backend.get_app_snapshot().unwrap();
        assert_eq!(completed.tasks[0].status, "completed");
        assert!(
            backend
                .update_task(UpdateTaskInput {
                    id: "task-1".to_string(),
                    title: "No".to_string(),
                    course: None,
                    event: event("event-invalid", "2026-09-28T10:30:00Z"),
                })
                .unwrap_err()
                .contains("reopen")
        );

        backend
            .reopen_task(TaskEventInput {
                id: "task-1".to_string(),
                event: event("event-reopened", "2026-09-28T11:00:00Z"),
            })
            .unwrap();
        backend
            .delete_task(TaskEventInput {
                id: "task-1".to_string(),
                event: event("event-deleted", "2026-09-28T12:00:00Z"),
            })
            .unwrap();

        let deleted = backend.get_app_snapshot().unwrap();
        assert!(deleted.tasks.is_empty());
        assert_eq!(deleted.pending_outbox_count, 5);
    }

    #[test]
    fn invalid_task_input_rolls_back_without_outbox_event() {
        let backend = Backend::open_in_memory().unwrap();
        let error = backend
            .create_task(CreateTaskInput {
                id: "task-invalid".to_string(),
                title: "   ".to_string(),
                course: None,
                event: event("event-invalid", "2026-09-28T08:00:00Z"),
            })
            .unwrap_err();

        assert!(error.contains("title"));
        let snapshot = backend.get_app_snapshot().unwrap();
        assert!(snapshot.tasks.is_empty());
        assert_eq!(snapshot.pending_outbox_count, 0);
    }

    #[test]
    fn focus_timer_snapshots_task_blocks_mutation_and_finishes_into_history() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);
        backend
            .start_focus(StartFocusInput {
                id: "session-1".to_string(),
                task_id: Some("task-1".to_string()),
                activity: None,
                planned_seconds: 1500,
                started_at: "2026-09-28T13:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
                event: event("session-started", "2026-09-28T13:00:00Z"),
            })
            .unwrap();
        assert!(
            backend
                .delete_task(TaskEventInput {
                    id: "task-1".to_string(),
                    event: event("blocked", "2026-09-28T13:01:00Z"),
                })
                .unwrap_err()
                .contains("timer")
        );
        backend
            .pause_focus(TimerTickInput {
                at: "2026-09-28T13:10:00Z".to_string(),
                accumulated_seconds: 600,
                remaining_seconds: 900,
                event: Some(event("session-paused", "2026-09-28T13:10:00Z")),
            })
            .unwrap();
        backend
            .resume_focus(TimerTickInput {
                at: "2026-09-28T13:15:00Z".to_string(),
                accumulated_seconds: 600,
                remaining_seconds: 900,
                event: Some(event("session-resumed", "2026-09-28T13:15:00Z")),
            })
            .unwrap();
        backend
            .finish_focus(FinishFocusInput {
                ended_at: "2026-09-28T13:30:00Z".to_string(),
                actual_seconds: 1500,
                completion_type: "timer_finished".to_string(),
                event: event("session-finished", "2026-09-28T13:30:00Z"),
            })
            .unwrap();

        let snapshot = backend.get_app_snapshot().unwrap();
        assert!(snapshot.active_timer.is_none());
        assert_eq!(snapshot.history.len(), 1);
        assert_eq!(
            snapshot.history[0].task_title_snapshot.as_deref(),
            Some("Read chapter 4")
        );
        assert_eq!(snapshot.history[0].actual_seconds, 1500);
        assert_eq!(snapshot.history[0].status, "completed");
        assert_eq!(snapshot.pending_outbox_count, 3);
        let connection = backend.connection.lock().unwrap();
        let event_bodies: Vec<String> = connection
            .prepare(
                "SELECT markdown_body FROM outbox WHERE category = 'sessions' ORDER BY created_at",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(
            event_bodies
                .iter()
                .any(|body| body.contains("Event type: focus_started"))
        );
        assert!(
            !event_bodies
                .iter()
                .any(|body| body.contains("Event type: focus_paused"))
        );
        assert!(
            !event_bodies
                .iter()
                .any(|body| body.contains("Event type: focus_resumed"))
        );
        let completed_body = event_bodies
            .iter()
            .find(|body| body.contains("Event ID: session-finished"))
            .unwrap();
        assert!(completed_body.contains("Event type: focus_completed"));
        assert!(completed_body.contains("Task title: Read chapter 4"));
        assert!(completed_body.contains("Course/Subject: Biology"));
        assert!(completed_body.contains("Actual seconds: 1500"));
    }

    #[test]
    fn activity_focus_can_be_cancelled_and_preserves_actual_time() {
        let backend = Backend::open_in_memory().unwrap();
        backend
            .start_focus(StartFocusInput {
                id: "session-activity".to_string(),
                task_id: None,
                activity: Some("  Plan study week  ".to_string()),
                planned_seconds: 900,
                started_at: "2026-09-28T14:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
                event: event("activity-started", "2026-09-28T14:00:00Z"),
            })
            .unwrap();
        backend
            .cancel_focus(CancelFocusInput {
                ended_at: "2026-09-28T14:04:00Z".to_string(),
                actual_seconds: 240,
                event: event("session-cancelled", "2026-09-28T14:04:00Z"),
            })
            .unwrap();

        let snapshot = backend.get_app_snapshot().unwrap();
        assert_eq!(
            snapshot.history[0].activity.as_deref(),
            Some("Plan study week")
        );
        assert_eq!(snapshot.history[0].actual_seconds, 240);
        assert_eq!(snapshot.history[0].status, "cancelled");
        assert_eq!(snapshot.pending_outbox_count, 2);
    }

    #[test]
    fn break_timer_has_no_history_or_outbox() {
        let backend = Backend::open_in_memory().unwrap();
        backend
            .start_break(StartBreakInput {
                id: "break-1".to_string(),
                planned_seconds: 300,
                started_at: "2026-09-28T15:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
            })
            .unwrap();
        backend
            .pause_break(TimerTickInput {
                at: "2026-09-28T15:01:00Z".to_string(),
                accumulated_seconds: 60,
                remaining_seconds: 240,
                event: None,
            })
            .unwrap();
        backend
            .resume_break(TimerTickInput {
                at: "2026-09-28T15:02:00Z".to_string(),
                accumulated_seconds: 60,
                remaining_seconds: 240,
                event: None,
            })
            .unwrap();
        backend.cancel_break().unwrap();

        let snapshot = backend.get_app_snapshot().unwrap();
        assert!(snapshot.active_timer.is_none());
        assert!(snapshot.history.is_empty());
        assert_eq!(snapshot.pending_outbox_count, 0);
    }

    #[test]
    fn settings_are_validated_and_persisted() {
        let backend = Backend::open_in_memory().unwrap();
        let saved = backend
            .save_settings(SaveSettingsInput {
                focus_minutes: 40,
                break_minutes: 10,
                sound_enabled: false,
            })
            .unwrap();
        assert_eq!(saved.focus_minutes, 40);
        assert_eq!(saved.break_minutes, 10);
        assert!(!saved.sound_enabled);
        assert!(
            backend
                .save_settings(SaveSettingsInput {
                    focus_minutes: 0,
                    break_minutes: 5,
                    sound_enabled: true,
                })
                .unwrap_err()
                .contains("greater than zero")
        );
        assert!(
            backend
                .save_settings(SaveSettingsInput {
                    focus_minutes: 181,
                    break_minutes: 5,
                    sound_enabled: true,
                })
                .unwrap_err()
                .contains("1 to 180")
        );
    }

    #[test]
    fn vault_configuration_creates_safe_output_folders_and_warns_without_obsidian() {
        let backend = Backend::open_in_memory().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let validation = backend
            .configure_vault(ConfigureVaultInput {
                path: vault.path().to_string_lossy().into_owned(),
                allow_pending_redirection: false,
            })
            .unwrap();

        assert!(validation.obsidian_warning);
        assert!(vault.path().join("FocusFlow/Tasks").is_dir());
        assert!(vault.path().join("FocusFlow/Sessions").is_dir());
        assert_eq!(
            backend
                .get_app_snapshot()
                .unwrap()
                .settings
                .vault_path
                .as_deref(),
            Some(validation.path.as_str())
        );
    }

    #[test]
    fn vault_change_is_blocked_by_timer_and_requires_explicit_pending_redirect() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        backend
            .configure_vault(ConfigureVaultInput {
                path: first.path().to_string_lossy().into_owned(),
                allow_pending_redirection: false,
            })
            .unwrap();
        assert!(
            backend
                .configure_vault(ConfigureVaultInput {
                    path: second.path().to_string_lossy().into_owned(),
                    allow_pending_redirection: false,
                })
                .unwrap_err()
                .contains("pending")
        );
        backend
            .configure_vault(ConfigureVaultInput {
                path: second.path().to_string_lossy().into_owned(),
                allow_pending_redirection: true,
            })
            .unwrap();
        backend
            .start_break(StartBreakInput {
                id: "break-lock".to_string(),
                planned_seconds: 300,
                started_at: "2026-09-28T16:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
            })
            .unwrap();
        assert!(
            backend
                .configure_vault(ConfigureVaultInput {
                    path: first.path().to_string_lossy().into_owned(),
                    allow_pending_redirection: true,
                })
                .unwrap_err()
                .contains("timer")
        );
    }

    #[test]
    fn retry_outbox_is_idempotent_and_marks_existing_event_ids_delivered() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);
        let vault = tempfile::tempdir().unwrap();
        backend
            .configure_vault(ConfigureVaultInput {
                path: vault.path().to_string_lossy().into_owned(),
                allow_pending_redirection: true,
            })
            .unwrap();
        let daily_file = vault.path().join("FocusFlow/Tasks/2026-09-28.md");
        std::fs::write(&daily_file, "# Existing\n- Event ID: event-created\n").unwrap();

        let result = backend.retry_outbox("2026-09-28T17:00:00Z").unwrap();
        assert_eq!(result.already_present, 1);
        assert_eq!(result.delivered, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(backend.get_app_snapshot().unwrap().pending_outbox_count, 0);
        let content = std::fs::read_to_string(daily_file).unwrap();
        assert_eq!(content.matches("Event ID: event-created").count(), 1);
    }

    #[test]
    fn task_can_be_deleted_after_its_focus_history_is_snapshotted() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);
        backend
            .start_focus(StartFocusInput {
                id: "historical-session".to_string(),
                task_id: Some("task-1".to_string()),
                activity: None,
                planned_seconds: 60,
                started_at: "2026-09-28T18:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
                event: event("historical-session-started", "2026-09-28T18:00:00Z"),
            })
            .unwrap();
        backend
            .finish_focus(FinishFocusInput {
                ended_at: "2026-09-28T18:01:00Z".to_string(),
                actual_seconds: 60,
                completion_type: "timer_finished".to_string(),
                event: event("historical-session-finished", "2026-09-28T18:01:00Z"),
            })
            .unwrap();
        backend
            .delete_task(TaskEventInput {
                id: "task-1".to_string(),
                event: event("historical-task-deleted", "2026-09-28T18:02:00Z"),
            })
            .unwrap();

        let snapshot = backend.get_app_snapshot().unwrap();
        assert!(snapshot.tasks.is_empty());
        assert_eq!(snapshot.history[0].task_id.as_deref(), Some("task-1"));
        assert_eq!(
            snapshot.history[0].task_title_snapshot.as_deref(),
            Some("Read chapter 4")
        );
    }

    #[test]
    fn completed_task_can_be_deleted_without_reopening() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);
        backend
            .complete_task(TaskEventInput {
                id: "task-1".to_string(),
                event: event("complete-before-delete", "2026-09-28T19:00:00Z"),
            })
            .unwrap();
        backend
            .delete_task(TaskEventInput {
                id: "task-1".to_string(),
                event: event("delete-completed", "2026-09-28T19:01:00Z"),
            })
            .unwrap();

        assert!(backend.get_app_snapshot().unwrap().tasks.is_empty());
    }

    #[test]
    fn event_timestamps_must_be_utc_with_valid_offset_and_local_date() {
        let backend = Backend::open_in_memory().unwrap();
        let mut invalid = event("invalid-time", "2026-09-28T08:00:00+02:00");
        invalid.event_offset = "Europe/Paris".to_string();
        invalid.local_date = "28-09-2026".to_string();
        let error = backend
            .create_task(CreateTaskInput {
                id: "invalid-time-task".to_string(),
                title: "Invalid time".to_string(),
                course: None,
                event: invalid,
            })
            .unwrap_err();

        assert!(error.contains("UTC"));
        assert!(backend.get_app_snapshot().unwrap().tasks.is_empty());
    }

    #[test]
    fn identifiers_are_conservative_and_cannot_be_reused() {
        let backend = Backend::open_in_memory().unwrap();
        let invalid_id = backend
            .create_task(CreateTaskInput {
                id: "task\n- injected".to_string(),
                title: "Invalid ID".to_string(),
                course: None,
                event: event("event-invalid-id", "2026-09-28T08:00:00Z"),
            })
            .unwrap_err();
        assert!(invalid_id.contains("ID"));

        create_task(&backend);
        backend
            .delete_task(TaskEventInput {
                id: "task-1".to_string(),
                event: event("delete-before-reuse", "2026-09-28T08:01:00Z"),
            })
            .unwrap();
        let reused = backend
            .create_task(CreateTaskInput {
                id: "task-1".to_string(),
                title: "Reused".to_string(),
                course: None,
                event: event("create-after-delete", "2026-09-28T08:02:00Z"),
            })
            .unwrap_err();
        assert!(reused.contains("already been used"));

        let invalid_event = backend
            .create_task(CreateTaskInput {
                id: "task-2".to_string(),
                title: "Invalid event".to_string(),
                course: None,
                event: event("event|markdown", "2026-09-28T08:03:00Z"),
            })
            .unwrap_err();
        assert!(invalid_event.contains("Event ID"));
    }

    #[test]
    fn human_fields_are_escaped_in_markdown() {
        let backend = Backend::open_in_memory().unwrap();
        backend
            .create_task(CreateTaskInput {
                id: "task-markdown".to_string(),
                title: "# Heading\n- injected".to_string(),
                course: Some("[Course](https://example.invalid)".to_string()),
                event: event("event-markdown", "2026-09-28T08:00:00Z"),
            })
            .unwrap();

        let body: String = backend
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT markdown_body FROM outbox WHERE event_id = 'event-markdown'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(body.contains("\\# Heading \\- injected"));
        assert!(body.contains("\\[Course\\]\\(https://example.invalid\\)"));
        assert!(!body.contains("\n- injected"));
    }

    #[test]
    fn timer_durations_are_derived_from_persisted_timestamps() {
        let backend = Backend::open_in_memory().unwrap();
        backend
            .start_focus(StartFocusInput {
                id: "derived-session".to_string(),
                task_id: None,
                activity: Some("Study".to_string()),
                planned_seconds: 600,
                started_at: "2026-09-28T13:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
                event: event("derived-start", "2026-09-28T13:00:00Z"),
            })
            .unwrap();
        let paused = backend
            .pause_focus(TimerTickInput {
                at: "2026-09-28T13:02:00Z".to_string(),
                accumulated_seconds: 9_999,
                remaining_seconds: 1,
                event: Some(event("derived-pause", "2026-09-28T13:02:00Z")),
            })
            .unwrap();
        assert_eq!(paused.accumulated_seconds, 120);
        assert_eq!(paused.remaining_seconds, 480);
        backend
            .resume_focus(TimerTickInput {
                at: "2026-09-28T13:03:00Z".to_string(),
                accumulated_seconds: 0,
                remaining_seconds: 600,
                event: Some(event("derived-resume", "2026-09-28T13:03:00Z")),
            })
            .unwrap();
        let session = backend
            .finish_focus(FinishFocusInput {
                ended_at: "2026-09-28T13:05:00Z".to_string(),
                actual_seconds: 9_999,
                completion_type: "completed_early".to_string(),
                event: event("derived-finish", "2026-09-28T13:05:00Z"),
            })
            .unwrap();
        assert_eq!(session.actual_seconds, 240);
    }

    #[test]
    fn timers_reject_backwards_mismatched_and_premature_events() {
        let backend = Backend::open_in_memory().unwrap();
        let mismatched_start = backend
            .start_focus(StartFocusInput {
                id: "mismatched-session".to_string(),
                task_id: None,
                activity: Some("Study".to_string()),
                planned_seconds: 600,
                started_at: "2026-09-28T13:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
                event: event("mismatched-start", "2026-09-28T13:00:01Z"),
            })
            .unwrap_err();
        assert!(mismatched_start.contains("match"));

        backend
            .start_focus(StartFocusInput {
                id: "guarded-session".to_string(),
                task_id: None,
                activity: Some("Study".to_string()),
                planned_seconds: 600,
                started_at: "2026-09-28T13:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
                event: event("guarded-start", "2026-09-28T13:00:00Z"),
            })
            .unwrap();
        let backwards = backend
            .pause_focus(TimerTickInput {
                at: "2026-09-28T12:59:59Z".to_string(),
                accumulated_seconds: 0,
                remaining_seconds: 600,
                event: Some(event("guarded-pause", "2026-09-28T12:59:59Z")),
            })
            .unwrap_err();
        assert!(backwards.contains("backwards"));

        let premature = backend
            .finish_focus(FinishFocusInput {
                ended_at: "2026-09-28T13:05:00Z".to_string(),
                actual_seconds: 600,
                completion_type: "timer_finished".to_string(),
                event: event("guarded-finish", "2026-09-28T13:05:00Z"),
            })
            .unwrap_err();
        assert!(premature.contains("before the planned duration"));
    }

    #[test]
    fn break_duration_comes_from_persisted_settings() {
        let backend = Backend::open_in_memory().unwrap();
        backend
            .save_settings(SaveSettingsInput {
                focus_minutes: 25,
                break_minutes: 7,
                sound_enabled: true,
            })
            .unwrap();
        let timer = backend
            .start_break(StartBreakInput {
                id: "settings-break".to_string(),
                planned_seconds: 1,
                started_at: "2026-09-28T15:00:00Z".to_string(),
                event_offset: "+02:00".to_string(),
                local_date: "2026-09-28".to_string(),
            })
            .unwrap();
        assert_eq!(timer.planned_seconds, 420);
        assert_eq!(timer.remaining_seconds, 420);
    }

    #[test]
    fn timestamps_are_canonicalized_and_local_dates_must_match_offsets() {
        let backend = Backend::open_in_memory().unwrap();
        let mut next_day = event("canonical-event", "2026-09-28T23:30:00+00:00");
        next_day.local_date = "2026-09-30".to_string();
        let error = backend
            .create_task(CreateTaskInput {
                id: "canonical-task".to_string(),
                title: "Canonical".to_string(),
                course: None,
                event: next_day,
            })
            .unwrap_err();
        assert!(error.contains("local date"));

        let mut valid = event("canonical-event", "2026-09-28T06:00:00+00:00");
        valid.event_offset = "+02:00".to_string();
        backend
            .create_task(CreateTaskInput {
                id: "canonical-task".to_string(),
                title: "Canonical".to_string(),
                course: None,
                event: valid,
            })
            .unwrap();
        assert_eq!(
            backend.get_app_snapshot().unwrap().tasks[0].created_at,
            "2026-09-28T06:00:00.000000000Z"
        );
    }

    #[test]
    fn retry_outbox_stops_after_the_first_failure() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);
        backend
            .update_task(UpdateTaskInput {
                id: "task-1".to_string(),
                title: "Second event".to_string(),
                course: None,
                event: event("event-second", "2026-09-28T08:01:00Z"),
            })
            .unwrap();
        let vault = tempfile::tempdir().unwrap();
        backend
            .configure_vault(ConfigureVaultInput {
                path: vault.path().to_string_lossy().into_owned(),
                allow_pending_redirection: true,
            })
            .unwrap();
        backend
            .connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE outbox SET local_date = '../escape' WHERE event_id = 'event-created'",
                [],
            )
            .unwrap();

        let result = backend.retry_outbox("2026-09-28T17:00:00Z").unwrap();
        assert_eq!(result.failed, 1);
        assert_eq!(result.delivered, 0);
        let second_error: Option<String> = backend
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT last_error FROM outbox WHERE event_id = 'event-second'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(second_error.is_none());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn delivery_rejects_a_symlinked_daily_file() {
        let backend = Backend::open_in_memory().unwrap();
        create_task(&backend);
        let vault = tempfile::tempdir().unwrap();
        backend
            .configure_vault(ConfigureVaultInput {
                path: vault.path().to_string_lossy().into_owned(),
                allow_pending_redirection: true,
            })
            .unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), "outside").unwrap();
        let daily_file = vault.path().join("FocusFlow/Tasks/2026-09-28.md");
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &daily_file).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(outside.path(), &daily_file).is_err() {
            return;
        }

        let result = backend.retry_outbox("2026-09-28T17:00:00Z").unwrap();
        assert_eq!(result.failed, 1);
        assert_eq!(std::fs::read_to_string(outside.path()).unwrap(), "outside");
    }
}
