use tauri::State;

use crate::backend::Backend;
use crate::models::{
    ActiveTimer, AppSnapshot, CancelFocusInput, ConfigureVaultInput, CreateTaskInput,
    FinishFocusInput, FocusSession, RetryOutboxResult, SaveSettingsInput, Settings,
    StartBreakInput, StartFocusInput, Task, TaskEventInput, TimerTickInput, UpdateTaskInput,
    VaultValidation,
};

#[tauri::command]
pub fn get_app_snapshot(backend: State<'_, Backend>) -> Result<AppSnapshot, String> {
    backend.get_app_snapshot()
}

#[tauri::command]
pub fn create_task(backend: State<'_, Backend>, input: CreateTaskInput) -> Result<Task, String> {
    backend.create_task(input)
}

#[tauri::command]
pub fn update_task(backend: State<'_, Backend>, input: UpdateTaskInput) -> Result<Task, String> {
    backend.update_task(input)
}

#[tauri::command]
pub fn complete_task(backend: State<'_, Backend>, input: TaskEventInput) -> Result<Task, String> {
    backend.complete_task(input)
}

#[tauri::command]
pub fn reopen_task(backend: State<'_, Backend>, input: TaskEventInput) -> Result<Task, String> {
    backend.reopen_task(input)
}

#[tauri::command]
pub fn delete_task(backend: State<'_, Backend>, input: TaskEventInput) -> Result<(), String> {
    backend.delete_task(input)
}

#[tauri::command]
pub fn start_focus(
    backend: State<'_, Backend>,
    input: StartFocusInput,
) -> Result<ActiveTimer, String> {
    backend.start_focus(input)
}

#[tauri::command]
pub fn pause_focus(
    backend: State<'_, Backend>,
    input: TimerTickInput,
) -> Result<ActiveTimer, String> {
    backend.pause_focus(input)
}

#[tauri::command]
pub fn resume_focus(
    backend: State<'_, Backend>,
    input: TimerTickInput,
) -> Result<ActiveTimer, String> {
    backend.resume_focus(input)
}

#[tauri::command]
pub fn finish_focus(
    backend: State<'_, Backend>,
    input: FinishFocusInput,
) -> Result<FocusSession, String> {
    backend.finish_focus(input)
}

#[tauri::command]
pub fn cancel_focus(
    backend: State<'_, Backend>,
    input: CancelFocusInput,
) -> Result<FocusSession, String> {
    backend.cancel_focus(input)
}

#[tauri::command]
pub fn start_break(
    backend: State<'_, Backend>,
    input: StartBreakInput,
) -> Result<ActiveTimer, String> {
    backend.start_break(input)
}

#[tauri::command]
pub fn pause_break(
    backend: State<'_, Backend>,
    input: TimerTickInput,
) -> Result<ActiveTimer, String> {
    backend.pause_break(input)
}

#[tauri::command]
pub fn resume_break(
    backend: State<'_, Backend>,
    input: TimerTickInput,
) -> Result<ActiveTimer, String> {
    backend.resume_break(input)
}

#[tauri::command]
pub fn cancel_break(backend: State<'_, Backend>) -> Result<(), String> {
    backend.cancel_break()
}

#[tauri::command]
pub fn recover_active_timer(backend: State<'_, Backend>) -> Result<Option<ActiveTimer>, String> {
    backend.recover_active_timer()
}

#[tauri::command]
pub fn save_settings(
    backend: State<'_, Backend>,
    input: SaveSettingsInput,
) -> Result<Settings, String> {
    backend.save_settings(input)
}

#[tauri::command]
pub fn validate_vault(
    backend: State<'_, Backend>,
    path: String,
) -> Result<VaultValidation, String> {
    backend.validate_vault(path)
}

#[tauri::command]
pub fn configure_vault(
    backend: State<'_, Backend>,
    input: ConfigureVaultInput,
) -> Result<VaultValidation, String> {
    backend.configure_vault(input)
}

#[tauri::command]
pub fn retry_outbox(
    backend: State<'_, Backend>,
    delivered_at: String,
) -> Result<RetryOutboxResult, String> {
    backend.retry_outbox(&delivered_at)
}
