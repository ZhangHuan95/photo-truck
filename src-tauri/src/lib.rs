// 公开模块以支持测试
pub mod classify;
pub mod cli;
mod commands;
pub mod exif;
pub mod hash;
pub mod history;
pub mod rename;
pub mod thumbnail;
pub mod transfer;
pub mod transfer_v2;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 检查是否有命令行参数
    if let Some(args) = cli::parse_args() {
        std::process::exit(cli::run_cli(args));
    }

    // 无参数时启动 GUI 模式
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
            // 新增命令
            cancel_transfer,
            get_rename_templates,
            set_rename_config,
            get_transfer_history,
            clear_transfer_history,
            delete_history_record,
            get_thumbnails,
            validate_custom_template,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
