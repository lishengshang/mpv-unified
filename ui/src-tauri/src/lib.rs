/// Example command: placeholder for the commands that will proxy
/// core/cli/pkg logic in later tasks (T18+).
#[tauri::command]
fn greet(name: &str) -> String {
    format!("你好, {name}!来自 Rust 的问候。")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
