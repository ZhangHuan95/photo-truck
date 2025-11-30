use crate::classify::{get_preset_templates, ClassifyConfig, SUPPORTED_EXTENSIONS};
use crate::exif::check_exiftool;
use crate::history::{TransferHistory, TransferRecord};
use crate::rename::{get_rename_templates as get_rename_presets, RenameConfig};
use crate::thumbnail::{extract_thumbnails, ThumbnailInfo};
use crate::transfer::{scan_photos, ScanResult, TransferResult};
use crate::transfer_v2::{transfer_photos_v2, TransferContext};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

/// 应用状态
pub struct AppState {
    pub scan_result: Mutex<Option<ScanResult>>,
    pub config: Mutex<ClassifyConfig>,
    pub rename_config: Mutex<RenameConfig>,
    pub cancel_flag: Arc<AtomicBool>,
    pub source_dir: Mutex<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scan_result: Mutex::new(None),
            config: Mutex::new(ClassifyConfig::default()),
            rename_config: Mutex::new(RenameConfig::default()),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            source_dir: Mutex::new(String::new()),
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
    
    // 保存扫描结果和源目录
    let mut scan_result = state.scan_result.lock().map_err(|e| e.to_string())?;
    *scan_result = Some(result.clone());
    
    let mut src = state.source_dir.lock().map_err(|e| e.to_string())?;
    *src = source_dir;
    
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
    // 重置取消标志
    state.cancel_flag.store(false, Ordering::Relaxed);
    
    let scan_result = state.scan_result.lock().map_err(|e| e.to_string())?;
    let photos = scan_result
        .as_ref()
        .ok_or("请先扫描源文件夹")?
        .photos
        .clone();
    drop(scan_result);
    
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let template = config.template.clone();
    drop(config);
    
    let rename_config = state.rename_config.lock().map_err(|e| e.to_string())?;
    let rename = rename_config.clone();
    drop(rename_config);
    
    let source_dir = state.source_dir.lock().map_err(|e| e.to_string())?;
    let src = source_dir.clone();
    drop(source_dir);
    
    let mut ctx = TransferContext::new(
        app_handle,
        state.cancel_flag.clone(),
        &src,
        &target_dir,
        &template,
    );
    ctx.rename_config = rename;
    
    transfer_photos_v2(&ctx, &photos, &target_dir, skip_duplicates)
}

/// 取消传输
#[tauri::command]
pub fn cancel_transfer(state: State<AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    Ok(())
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

// ==================== 重命名相关命令 ====================

/// 获取重命名模板列表
#[tauri::command]
pub fn get_rename_templates() -> Vec<RenameTemplateInfo> {
    get_rename_presets()
        .into_iter()
        .map(|(name, template)| RenameTemplateInfo {
            name: name.to_string(),
            template: template.to_string(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct RenameTemplateInfo {
    pub name: String,
    pub template: String,
}

/// 设置重命名配置
#[tauri::command]
pub fn set_rename_config(
    state: State<AppState>,
    enabled: bool,
    template: String,
    counter_start: u32,
    counter_digits: u32,
) -> Result<(), String> {
    let mut config = state.rename_config.lock().map_err(|e| e.to_string())?;
    config.enabled = enabled;
    config.template = template;
    config.counter_start = counter_start;
    config.counter_digits = counter_digits;
    Ok(())
}

// ==================== 历史记录相关命令 ====================

/// 获取传输历史记录
#[tauri::command]
pub fn get_transfer_history() -> Result<Vec<TransferRecord>, String> {
    let history = TransferHistory::load();
    Ok(history.records)
}

/// 清空历史记录
#[tauri::command]
pub fn clear_transfer_history() -> Result<(), String> {
    let mut history = TransferHistory::load();
    history.clear();
    history.save()
}

/// 删除单条历史记录
#[tauri::command]
pub fn delete_history_record(id: String) -> Result<(), String> {
    let mut history = TransferHistory::load();
    history.delete_record(&id);
    history.save()
}

// ==================== 缩略图相关命令 ====================

/// 获取照片缩略图
#[tauri::command]
pub fn get_thumbnails(state: State<AppState>, max_count: usize) -> Result<Vec<ThumbnailInfo>, String> {
    let scan_result = state.scan_result.lock().map_err(|e| e.to_string())?;
    
    let photos = scan_result
        .as_ref()
        .ok_or("请先扫描源文件夹")?;
    
    let paths: Vec<String> = photos.photos.iter().map(|p| p.path.clone()).collect();
    Ok(extract_thumbnails(&paths, max_count))
}

// ==================== 模板验证命令 ====================

/// 验证自定义模板
#[tauri::command]
pub fn validate_custom_template(template: String) -> Result<TemplateValidation, String> {
    let valid_vars = vec!["{year}", "{month}", "{day}", "{camera}", "{make}"];
    let mut warnings = Vec::new();
    let mut example = template.clone();
    
    // 检查是否包含有效变量
    let has_valid_var = valid_vars.iter().any(|v| template.contains(v));
    if !has_valid_var {
        warnings.push("模板中没有包含任何有效变量".to_string());
    }
    
    // 生成示例
    example = example.replace("{year}", "2024");
    example = example.replace("{month}", "03");
    example = example.replace("{day}", "15");
    example = example.replace("{camera}", "Canon EOS R5");
    example = example.replace("{make}", "Canon");
    
    // 检查未知变量
    let re = regex::Regex::new(r"\{[^}]+\}").unwrap();
    for cap in re.find_iter(&example) {
        warnings.push(format!("未知变量: {}", cap.as_str()));
    }
    
    Ok(TemplateValidation {
        valid: warnings.is_empty(),
        example,
        warnings,
        supported_vars: valid_vars.iter().map(|s| s.to_string()).collect(),
    })
}

#[derive(Debug, Serialize)]
pub struct TemplateValidation {
    pub valid: bool,
    pub example: String,
    pub warnings: Vec<String>,
    pub supported_vars: Vec<String>,
}
