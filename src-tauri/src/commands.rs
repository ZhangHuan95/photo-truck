use crate::classify::{get_preset_templates, ClassifyConfig, SUPPORTED_EXTENSIONS};
use crate::exif::check_exiftool;
use crate::transfer::{scan_photos, transfer_photos, ScanResult, TransferResult};
use serde::Serialize;
use tauri::{AppHandle, State};
use std::sync::Mutex;

/// 应用状态
pub struct AppState {
    pub scan_result: Mutex<Option<ScanResult>>,
    pub config: Mutex<ClassifyConfig>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scan_result: Mutex::new(None),
            config: Mutex::new(ClassifyConfig::default()),
        }
    }
}

/// 检查系统环境
#[tauri::command]
pub fn check_environment() -> Result<EnvironmentInfo, String> {
    let exiftool_version = check_exiftool().ok();
    
    Ok(EnvironmentInfo {
        exiftool_installed: exiftool_version.is_some(),
        exiftool_version,
        supported_formats: SUPPORTED_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
    })
}

#[derive(Debug, Serialize)]
pub struct EnvironmentInfo {
    pub exiftool_installed: bool,
    pub exiftool_version: Option<String>,
    pub supported_formats: Vec<String>,
}

/// 获取预设分类模板
#[tauri::command]
pub fn get_templates() -> Vec<TemplateInfo> {
    get_preset_templates()
        .into_iter()
        .map(|(name, template)| TemplateInfo {
            name: name.to_string(),
            template: template.to_string(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub name: String,
    pub template: String,
}

/// 设置分类配置
#[tauri::command]
pub fn set_classify_config(
    state: State<AppState>,
    template: String,
    fallback_folder: String,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.template = template;
    config.fallback_folder = fallback_folder;
    Ok(())
}

/// 获取当前分类配置
#[tauri::command]
pub fn get_classify_config(state: State<AppState>) -> Result<ClassifyConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// 扫描源文件夹
#[tauri::command]
pub fn scan_source_folder(
    state: State<AppState>,
    source_dir: String,
) -> Result<ScanResult, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let result = scan_photos(&source_dir, &config)?;
    
    // 保存扫描结果
    let mut scan_result = state.scan_result.lock().map_err(|e| e.to_string())?;
    *scan_result = Some(result.clone());
    
    Ok(result)
}

/// 开始传输
#[tauri::command]
pub async fn start_transfer(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    target_dir: String,
    skip_duplicates: bool,
) -> Result<TransferResult, String> {
    let scan_result = state.scan_result.lock().map_err(|e| e.to_string())?;
    
    let photos = scan_result
        .as_ref()
        .ok_or("请先扫描源文件夹")?
        .photos
        .clone();
    
    drop(scan_result); // 释放锁
    
    transfer_photos(&app_handle, &photos, &target_dir, skip_duplicates)
}

/// 预览分类结果（不实际传输）
#[tauri::command]
pub fn preview_classification(state: State<AppState>) -> Result<Vec<ClassificationPreview>, String> {
    let scan_result = state.scan_result.lock().map_err(|e| e.to_string())?;
    
    let photos = scan_result
        .as_ref()
        .ok_or("请先扫描源文件夹")?;
    
    // 按目标文件夹分组
    let mut groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    
    for photo in &photos.photos {
        groups
            .entry(photo.target_folder.clone())
            .or_insert_with(Vec::new)
            .push(photo.file_name.clone());
    }
    
    let mut previews: Vec<ClassificationPreview> = groups
        .into_iter()
        .map(|(folder, files)| ClassificationPreview {
            folder,
            file_count: files.len(),
            files,
        })
        .collect();
    
    previews.sort_by(|a, b| a.folder.cmp(&b.folder));
    
    Ok(previews)
}

#[derive(Debug, Serialize)]
pub struct ClassificationPreview {
    pub folder: String,
    pub file_count: usize,
    pub files: Vec<String>,
}
