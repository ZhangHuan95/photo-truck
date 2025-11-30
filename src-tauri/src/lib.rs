// 公开模块以支持测试
pub mod classify;
mod commands;
pub mod exif;
pub mod hash;
pub mod transfer;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            check_environment,
            get_templates,
            set_classify_config,
            get_classify_config,
            scan_source_folder,
            start_transfer,
            preview_classification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
