mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::get_profile_state,
            commands::set_profile_state,
            commands::regenerate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
