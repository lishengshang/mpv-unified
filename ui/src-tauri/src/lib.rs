mod commands;
mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::get_profile_state,
            commands::set_profile_state,
            commands::regenerate,
            commands::list_options,
            commands::get_gui_values,
            commands::save_gui_values,
            commands::reset_gui_value,
            store::list_packages,
            store::update_index,
            store::install_package,
            store::uninstall_package,
            store::update_package,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
