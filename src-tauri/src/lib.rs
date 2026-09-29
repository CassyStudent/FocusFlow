mod backend;
mod commands;
mod models;

use std::fs;

use backend::Backend;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_dir)?;
            let backend = Backend::open(app_data_dir.join("focusflow.sqlite"))
                .map_err(std::io::Error::other)?;
            app.manage(backend);
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_snapshot,
            commands::create_task,
            commands::update_task,
            commands::complete_task,
            commands::reopen_task,
            commands::delete_task,
            commands::start_focus,
            commands::pause_focus,
            commands::resume_focus,
            commands::finish_focus,
            commands::cancel_focus,
            commands::start_break,
            commands::pause_break,
            commands::resume_break,
            commands::cancel_break,
            commands::recover_active_timer,
            commands::save_settings,
            commands::validate_vault,
            commands::configure_vault,
            commands::retry_outbox,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
