use crate::exif::read_exif;
use crate::hash::Deduplicator;
use crate::rename::RenameConfig;
use crate::transfer::{PhotoInfo, TransferProgress, TransferResult};
use crate::history::{TransferHistory, TransferredFile, TransferFileStatus};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

/// 带取消支持的传输上下文
pub struct TransferContext {
    pub app_handle: AppHandle,
    pub cancel_flag: Arc<AtomicBool>,
    pub rename_config: RenameConfig,
    pub source_dir: String,
    pub target_dir: String,
    pub template: String,
}

impl TransferContext {
    pub fn new(
        app_handle: AppHandle,
        cancel_flag: Arc<AtomicBool>,
        source_dir: &str,
        target_dir: &str,
        template: &str,
    ) -> Self {
        Self {
            app_handle,
            cancel_flag,
            rename_config: RenameConfig::default(),
            source_dir: source_dir.to_string(),
            target_dir: target_dir.to_string(),
            template: template.to_string(),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }
}

/// 执行照片传输（支持取消、重命名和历史记录）
pub fn transfer_photos_v2(
    ctx: &TransferContext,
    photos: &[PhotoInfo],
    target_base_dir: &str,
    skip_duplicates: bool,
) -> Result<TransferResult, String> {
    let start_time = Instant::now();
    let mut success_count = 0;
    let mut skip_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();
    let mut transferred_files = Vec::new();
    let mut deduplicator = Deduplicator::new();
    let total = photos.len();
    let total_bytes: u64 = photos.iter().map(|p| p.file_size).sum();
    let mut bytes_transferred = 0u64;
    let mut counter = ctx.rename_config.counter_start;

    // 如果启用去重，先扫描目标目录中已有的文件
    if skip_duplicates {
        let _ = ctx.app_handle.emit("transfer-progress", TransferProgress {
            current: 0,
            total,
            current_file: "正在扫描目标目录已有文件...".to_string(),
            bytes_transferred: 0,
            total_bytes,
            status: "scanning".to_string(),
            skipped_duplicates: 0,
        });

        if Path::new(target_base_dir).exists() {
            for entry in WalkDir::new(target_base_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                // 检查取消
                if ctx.is_cancelled() {
                    return Ok(TransferResult {
                        success_count,
                        skip_count,
                        error_count,
                        errors: vec!["传输已取消".to_string()],
                    });
                }
                
                if entry.path().is_file() {
                    let _ = deduplicator.add_known_file(&entry.path().to_string_lossy());
                }
            }
        }
    }

    for (index, photo) in photos.iter().enumerate() {
        // 检查取消标志
        if ctx.is_cancelled() {
            let _ = ctx.app_handle.emit("transfer-progress", TransferProgress {
                current: index,
                total,
                current_file: "传输已取消".to_string(),
                bytes_transferred,
                total_bytes,
                status: "cancelled".to_string(),
                skipped_duplicates: skip_count,
            });
            
            errors.push("传输已取消".to_string());
            break;
        }

        // 发送进度事件
        let _ = ctx.app_handle.emit("transfer-progress", TransferProgress {
            current: index + 1,
            total,
            current_file: photo.file_name.clone(),
            bytes_transferred,
            total_bytes,
            status: "transferring".to_string(),
            skipped_duplicates: skip_count,
        });

        // 检查重复
        if skip_duplicates {
            match deduplicator.check_duplicate(&photo.path, photo.file_size) {
                Ok(Some(_original)) => {
                    skip_count += 1;
                    bytes_transferred += photo.file_size;
                    transferred_files.push(TransferredFile {
                        source_path: photo.path.clone(),
                        target_path: String::new(),
                        file_size: photo.file_size,
                        status: TransferFileStatus::Skipped,
                    });
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    errors.push(format!("检查重复失败 {}: {}", photo.file_name, e));
                }
            }
        }

        // 构建目标路径
        let target_dir = Path::new(target_base_dir).join(&photo.target_folder);
        
        // 生成新文件名（如果启用重命名）
        let new_filename = if ctx.rename_config.enabled {
            let metadata = read_exif(&photo.path).unwrap_or_default();
            let name = ctx.rename_config.generate_filename(&metadata, counter);
            counter += 1;
            name
        } else {
            photo.file_name.clone()
        };
        
        let target_path = target_dir.join(&new_filename);

        // 创建目标目录
        if let Err(e) = fs::create_dir_all(&target_dir) {
            error_count += 1;
            errors.push(format!("创建目录失败 {}: {}", target_dir.display(), e));
            transferred_files.push(TransferredFile {
                source_path: photo.path.clone(),
                target_path: target_path.to_string_lossy().to_string(),
                file_size: photo.file_size,
                status: TransferFileStatus::Error(e.to_string()),
            });
            continue;
        }

        // 如果目标文件已存在，添加序号
        let final_target_path = if target_path.exists() {
            let stem = Path::new(&new_filename)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let ext = Path::new(&new_filename)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            
            let mut file_counter = 1;
            loop {
                let new_name = if ext.is_empty() {
                    format!("{}_{}", stem, file_counter)
                } else {
                    format!("{}_{}.{}", stem, file_counter, ext)
                };
                let new_path = target_dir.join(&new_name);
                if !new_path.exists() {
                    break new_path;
                }
                file_counter += 1;
            }
        } else {
            target_path
        };

        // 复制文件
        match fs::copy(&photo.path, &final_target_path) {
            Ok(_) => {
                success_count += 1;
                bytes_transferred += photo.file_size;
                transferred_files.push(TransferredFile {
                    source_path: photo.path.clone(),
                    target_path: final_target_path.to_string_lossy().to_string(),
                    file_size: photo.file_size,
                    status: TransferFileStatus::Success,
                });
            }
            Err(e) => {
                error_count += 1;
                errors.push(format!("复制失败 {}: {}", photo.file_name, e));
                transferred_files.push(TransferredFile {
                    source_path: photo.path.clone(),
                    target_path: final_target_path.to_string_lossy().to_string(),
                    file_size: photo.file_size,
                    status: TransferFileStatus::Error(e.to_string()),
                });
            }
        }
    }

    let final_status = if ctx.is_cancelled() { "cancelled" } else { "completed" };
    
    // 发送完成事件
    let _ = ctx.app_handle.emit("transfer-progress", TransferProgress {
        current: total,
        total,
        current_file: if ctx.is_cancelled() { "传输已取消" } else { "传输完成" }.to_string(),
        bytes_transferred,
        total_bytes,
        status: final_status.to_string(),
        skipped_duplicates: skip_count,
    });

    // 保存历史记录
    let duration = start_time.elapsed().as_secs();
    let mut record = TransferHistory::create_record(
        &ctx.source_dir,
        &ctx.target_dir,
        &ctx.template,
    );
    record.total_files = total;
    record.success_count = success_count;
    record.skip_count = skip_count;
    record.error_count = error_count;
    record.total_size = total_bytes;
    record.duration_secs = duration;
    record.files = transferred_files;

    let mut history = TransferHistory::load();
    history.add_record(record);
    let _ = history.save();

    Ok(TransferResult {
        success_count,
        skip_count,
        error_count,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_context_cancel() {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        
        // 初始状态未取消
        assert!(!cancel_flag.load(Ordering::Relaxed));
        
        // 设置取消
        cancel_flag.store(true, Ordering::Relaxed);
        assert!(cancel_flag.load(Ordering::Relaxed));
    }
}
