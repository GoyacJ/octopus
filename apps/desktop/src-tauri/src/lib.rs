pub mod backend;
pub mod bootstrap;
pub mod commands;
pub mod error;
pub mod services;
pub mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .setup(|app| {
      app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .build(),
      )?;

      let shell_state =
        state::build_shell_state(app.handle()).map_err(Box::<dyn std::error::Error>::from)?;
      let preferences_path = shell_state.preferences_service.path().to_path_buf();
      if let Err(error) = tauri::async_runtime::block_on(async {
        shell_state
          .backend_supervisor
          .start(app.handle(), &shell_state.host_state, &preferences_path)
          .await
      }) {
        log::warn!("desktop backend unavailable at startup: {error}");
      }
      app.manage(shell_state);

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::bootstrap_shell,
      commands::get_host_state,
      commands::load_preferences,
      commands::save_preferences,
      commands::list_connections_stub,
      commands::healthcheck,
      commands::restart_desktop_backend,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
