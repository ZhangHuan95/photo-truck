use crate::classify::{is_supported_photo, ClassifyConfig};
use crate::exif::{read_exif, PhotoMetadata};
use crate::hash::Deduplicator;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

/// 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_files: usize,
    pub total_size: u64,
    pub photos: Vec<PhotoInfo>,
}

/// 照片信息（用于前端显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoInfo {
    pub path: String,
    pub file_name: String,
    pub file_size: u64,
    pub date_time: Option<String>,
    pub camera: Option<String>,
    pub target_folder: String,
    pub is_duplicate: bool,
    pub duplicate_of: Option<String>,
}

/// 传输进度事件
#[derive(Debug, Clone, Serialize)]
pub struct TransferProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub status: String,
    pub skipped_duplicates: usize,
}

/// 传输结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub success_count: usize,
    pub skip_count: usize,
    pub error_count: usize,
    pub errors: Vec<String>,
}

/// 扫描源文件夹中的照片
pub fn scan_photos(source_dir: &str, config: &ClassifyConfig) -> Result<ScanResult, String> {
    let mut photos = Vec::new();
    let mut total_size = 0u64;

    let path = Path::new(source_dir);
    if !path.exists() {
        return Err(format!("源文件夹不存在: {}", source_dir));
    }

    for entry in WalkDir::new(source_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }

        let file_path_str = file_path.to_string_lossy().to_string();
        if !is_supported_photo(&file_path_str) {
            continue;
        }

        // 读取文件大小
        let file_size = fs::metadata(file_path)
            .map(|m| m.len())
            .unwrap_or(0);
        total_size += file_size;

        // 读取 EXIF 信息
        let metadata = read_exif(&file_path_str).unwrap_or_else(|_| PhotoMetadata {
            file_path: file_path_str.clone(),
            file_name: file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            file_size,
            ..Default::default()
        });

        // 生成目标文件夹路径
        let target_folder = config.generate_path(&metadata);

        photos.push(PhotoInfo {
            path: file_path_str,
            file_name: metadata.file_name,
            file_size,
            date_time: metadata.date_time_original.or(metadata.create_date),
            camera: metadata.model,
            target_folder,
            is_duplicate: false,
            duplicate_of: None,
        });
    }

    Ok(ScanResult {
        total_files: photos.len(),
        total_size,
        photos,
    })
}

/// 执行照片传输
pub fn transfer_photos(
    app_handle: &AppHandle,
    photos: &[PhotoInfo],
    target_base_dir: &str,
    skip_duplicates: bool,
) -> Result<TransferResult, String> {
    let mut success_count = 0;
    let mut skip_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();
    let mut deduplicator = Deduplicator::new();
    let total = photos.len();
    let total_bytes: u64 = photos.iter().map(|p| p.file_size).sum();
    let mut bytes_transferred = 0u64;

    // 如果启用去重，先扫描目标目录中已有的文件
    if skip_duplicates {
        let _ = app_handle.emit("transfer-progress", TransferProgress {
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
                if entry.path().is_file() {
                    let _ = deduplicator.add_known_file(&entry.path().to_string_lossy());
                }
            }
        }
    }

    for (index, photo) in photos.iter().enumerate() {
        // 发送进度事件
        let _ = app_handle.emit("transfer-progress", TransferProgress {
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
        let target_path = target_dir.join(&photo.file_name);

        // 创建目标目录
        if let Err(e) = fs::create_dir_all(&target_dir) {
            error_count += 1;
            errors.push(format!("创建目录失败 {}: {}", target_dir.display(), e));
            continue;
        }

        // 如果目标文件已存在，添加序号
        let final_target_path = if target_path.exists() {
            let stem = Path::new(&photo.file_name)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let ext = Path::new(&photo.file_name)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            
            let mut counter = 1;
            loop {
                let new_name = format!("{}_{}.{}", stem, counter, ext);
                let new_path = target_dir.join(&new_name);
                if !new_path.exists() {
                    break new_path;
                }
                counter += 1;
            }
        } else {
            target_path
        };

        // 复制文件
        match fs::copy(&photo.path, &final_target_path) {
            Ok(_) => {
                success_count += 1;
                bytes_transferred += photo.file_size;
            }
            Err(e) => {
                error_count += 1;
                errors.push(format!("复制失败 {}: {}", photo.file_name, e));
            }
        }
    }

    // 发送完成事件
    let _ = app_handle.emit("transfer-progress", TransferProgress {
        current: total,
        total,
        current_file: "传输完成".to_string(),
        bytes_transferred: total_bytes,
        total_bytes,
        status: "completed".to_string(),
        skipped_duplicates: skip_count,
    });

    Ok(TransferResult {
        success_count,
        skip_count,
        error_count,
        errors,
    })
}

/// 格式化文件大小
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    // ==================== 辅助函数 ====================

    fn create_test_photo(dir: &TempDir, subdir: &str, name: &str, content: &[u8]) -> String {
        let folder = dir.path().join(subdir);
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path.to_string_lossy().to_string()
    }

    fn create_test_photo_root(dir: &TempDir, name: &str, content: &[u8]) -> String {
        let path = dir.path().join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path.to_string_lossy().to_string()
    }

    // ==================== format_size 测试 ====================

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(10240), "10.00 KB");
        assert_eq!(format_size(1024 * 1023), "1023.00 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 5), "5.00 MB");
        assert_eq!(format_size(1024 * 1024 * 25), "25.00 MB"); // 典型RAW大小
        assert_eq!(format_size(1024 * 1024 * 100), "100.00 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 2), "2.00 GB");
        assert_eq!(format_size(1024u64 * 1024 * 1024 * 100), "100.00 GB");
    }

    #[test]
    fn test_format_size_decimal() {
        // 测试小数位
        let size = 1024 * 1024 + 512 * 1024; // 1.5 MB
        assert_eq!(format_size(size), "1.50 MB");
    }

    // ==================== ScanResult 测试 ====================

    #[test]
    fn test_scan_result_empty() {
        let result = ScanResult {
            total_files: 0,
            total_size: 0,
            photos: vec![],
        };
        assert_eq!(result.total_files, 0);
        assert!(result.photos.is_empty());
    }

    #[test]
    fn test_scan_result_with_photos() {
        let photos = vec![
            PhotoInfo {
                path: "/test/photo1.jpg".to_string(),
                file_name: "photo1.jpg".to_string(),
                file_size: 1000,
                date_time: Some("2024:03:15 14:30:00".to_string()),
                camera: Some("Canon".to_string()),
                target_folder: "2024/03".to_string(),
                is_duplicate: false,
                duplicate_of: None,
            },
            PhotoInfo {
                path: "/test/photo2.jpg".to_string(),
                file_name: "photo2.jpg".to_string(),
                file_size: 2000,
                date_time: None,
                camera: None,
                target_folder: "未知日期".to_string(),
                is_duplicate: false,
                duplicate_of: None,
            },
        ];

        let result = ScanResult {
            total_files: 2,
            total_size: 3000,
            photos,
        };

        assert_eq!(result.total_files, 2);
        assert_eq!(result.total_size, 3000);
        assert_eq!(result.photos.len(), 2);
    }

    // ==================== PhotoInfo 测试 ====================

    #[test]
    fn test_photo_info_default_values() {
        let info = PhotoInfo {
            path: String::new(),
            file_name: String::new(),
            file_size: 0,
            date_time: None,
            camera: None,
            target_folder: String::new(),
            is_duplicate: false,
            duplicate_of: None,
        };
        assert!(!info.is_duplicate);
        assert!(info.duplicate_of.is_none());
    }

    #[test]
    fn test_photo_info_duplicate() {
        let info = PhotoInfo {
            path: "/test/copy.jpg".to_string(),
            file_name: "copy.jpg".to_string(),
            file_size: 1000,
            date_time: None,
            camera: None,
            target_folder: "2024/03".to_string(),
            is_duplicate: true,
            duplicate_of: Some("/test/original.jpg".to_string()),
        };
        assert!(info.is_duplicate);
        assert_eq!(info.duplicate_of.as_deref(), Some("/test/original.jpg"));
    }

    #[test]
    fn test_photo_info_serialization() {
        let info = PhotoInfo {
            path: "/test/photo.cr3".to_string(),
            file_name: "photo.cr3".to_string(),
            file_size: 25000000,
            date_time: Some("2024:12:25 10:30:00".to_string()),
            camera: Some("Canon EOS R5".to_string()),
            target_folder: "2024/12/25".to_string(),
            is_duplicate: false,
            duplicate_of: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("photo.cr3"));
        assert!(json.contains("Canon EOS R5"));

        let deserialized: PhotoInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_name, "photo.cr3");
    }

    // ==================== TransferProgress 测试 ====================

    #[test]
    fn test_transfer_progress_initial() {
        let progress = TransferProgress {
            current: 0,
            total: 100,
            current_file: "准备中...".to_string(),
            bytes_transferred: 0,
            total_bytes: 1000000,
            status: "preparing".to_string(),
            skipped_duplicates: 0,
        };

        assert_eq!(progress.current, 0);
        assert_eq!(progress.status, "preparing");
    }

    #[test]
    fn test_transfer_progress_in_progress() {
        let progress = TransferProgress {
            current: 50,
            total: 100,
            current_file: "IMG_0050.CR3".to_string(),
            bytes_transferred: 500000,
            total_bytes: 1000000,
            status: "transferring".to_string(),
            skipped_duplicates: 5,
        };

        assert_eq!(progress.current, 50);
        assert_eq!(progress.skipped_duplicates, 5);
        // 计算进度百分比
        let percentage = (progress.current as f64 / progress.total as f64) * 100.0;
        assert_eq!(percentage, 50.0);
    }

    #[test]
    fn test_transfer_progress_completed() {
        let progress = TransferProgress {
            current: 100,
            total: 100,
            current_file: "传输完成".to_string(),
            bytes_transferred: 1000000,
            total_bytes: 1000000,
            status: "completed".to_string(),
            skipped_duplicates: 10,
        };

        assert_eq!(progress.status, "completed");
        assert_eq!(progress.bytes_transferred, progress.total_bytes);
    }

    // ==================== TransferResult 测试 ====================

    #[test]
    fn test_transfer_result_all_success() {
        let result = TransferResult {
            success_count: 100,
            skip_count: 0,
            error_count: 0,
            errors: vec![],
        };

        assert_eq!(result.success_count, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_transfer_result_with_skips() {
        let result = TransferResult {
            success_count: 80,
            skip_count: 20,
            error_count: 0,
            errors: vec![],
        };

        assert_eq!(result.success_count + result.skip_count, 100);
    }

    #[test]
    fn test_transfer_result_with_errors() {
        let result = TransferResult {
            success_count: 95,
            skip_count: 0,
            error_count: 5,
            errors: vec![
                "复制失败 photo1.jpg: 权限不足".to_string(),
                "复制失败 photo2.jpg: 磁盘空间不足".to_string(),
            ],
        };

        assert_eq!(result.error_count, 5);
        assert_eq!(result.errors.len(), 2);
    }

    // ==================== scan_photos 测试 ====================

    #[test]
    fn test_scan_photos_empty_directory() {
        let dir = TempDir::new().unwrap();
        let config = ClassifyConfig::default();
        
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        assert!(result.is_ok());
        
        let scan_result = result.unwrap();
        assert_eq!(scan_result.total_files, 0);
        assert!(scan_result.photos.is_empty());
    }

    #[test]
    fn test_scan_photos_nonexistent_directory() {
        let config = ClassifyConfig::default();
        let result = scan_photos("/nonexistent/directory/path", &config);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[test]
    fn test_scan_photos_with_supported_files() {
        let dir = TempDir::new().unwrap();
        
        // 创建测试照片文件
        create_test_photo_root(&dir, "photo1.jpg", b"fake jpeg content");
        create_test_photo_root(&dir, "photo2.png", b"fake png content");
        create_test_photo_root(&dir, "photo3.cr3", b"fake cr3 content");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        let scan_result = result.unwrap();
        assert_eq!(scan_result.total_files, 3);
    }

    #[test]
    fn test_scan_photos_filters_unsupported() {
        let dir = TempDir::new().unwrap();
        
        // 创建混合文件
        create_test_photo_root(&dir, "photo.jpg", b"jpeg");
        create_test_photo_root(&dir, "document.pdf", b"pdf");
        create_test_photo_root(&dir, "video.mp4", b"mp4");
        create_test_photo_root(&dir, "readme.txt", b"text");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        let scan_result = result.unwrap();
        // 只应该找到 1 个照片文件
        assert_eq!(scan_result.total_files, 1);
    }

    #[test]
    fn test_scan_photos_recursive() {
        let dir = TempDir::new().unwrap();
        
        // 在根目录创建照片
        create_test_photo_root(&dir, "root.jpg", b"root jpeg");
        
        // 在子目录创建照片
        create_test_photo(&dir, "subdir1", "sub1.jpg", b"sub1 jpeg");
        create_test_photo(&dir, "subdir2/nested", "nested.png", b"nested png");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        let scan_result = result.unwrap();
        // 应该递归找到所有照片
        assert_eq!(scan_result.total_files, 3);
    }

    #[test]
    fn test_scan_photos_calculates_total_size() {
        let dir = TempDir::new().unwrap();
        
        let content1 = vec![0u8; 1000];
        let content2 = vec![0u8; 2000];
        
        create_test_photo_root(&dir, "photo1.jpg", &content1);
        create_test_photo_root(&dir, "photo2.jpg", &content2);
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        let scan_result = result.unwrap();
        assert_eq!(scan_result.total_size, 3000);
    }

    #[test]
    fn test_scan_photos_generates_target_folders() {
        let dir = TempDir::new().unwrap();
        
        // ExifTool 不会在假文件上工作，所以会使用 fallback
        create_test_photo_root(&dir, "photo.jpg", b"fake content");
        
        let config = ClassifyConfig {
            template: "{year}/{month}".to_string(),
            fallback_folder: "未分类".to_string(),
        };
        
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        assert!(result.is_ok());
        
        let scan_result = result.unwrap();
        assert_eq!(scan_result.photos.len(), 1);
        // 没有 EXIF 数据，应该使用 fallback
        assert_eq!(scan_result.photos[0].target_folder, "未分类");
    }

    // ==================== 文件名处理测试 ====================

    #[test]
    fn test_scan_photos_unicode_filename() {
        let dir = TempDir::new().unwrap();
        
        create_test_photo_root(&dir, "照片_2024.jpg", b"chinese name");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        let scan_result = result.unwrap();
        assert_eq!(scan_result.total_files, 1);
        assert!(scan_result.photos[0].file_name.contains("照片"));
    }

    #[test]
    fn test_scan_photos_special_characters_filename() {
        let dir = TempDir::new().unwrap();
        
        create_test_photo_root(&dir, "photo with spaces.jpg", b"spaces");
        create_test_photo_root(&dir, "photo-with-dashes.jpg", b"dashes");
        create_test_photo_root(&dir, "photo_with_underscores.jpg", b"underscores");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total_files, 3);
    }

    // ==================== 边界情况测试 ====================

    #[test]
    fn test_scan_photos_empty_file() {
        let dir = TempDir::new().unwrap();
        
        create_test_photo_root(&dir, "empty.jpg", b"");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        let scan_result = result.unwrap();
        assert_eq!(scan_result.total_files, 1);
        assert_eq!(scan_result.photos[0].file_size, 0);
    }

    #[test]
    fn test_scan_photos_hidden_files() {
        let dir = TempDir::new().unwrap();
        
        create_test_photo_root(&dir, ".hidden.jpg", b"hidden");
        create_test_photo_root(&dir, "visible.jpg", b"visible");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        // 隐藏文件也应该被扫描到
        assert_eq!(result.unwrap().total_files, 2);
    }

    #[test]
    fn test_scan_photos_case_insensitive_extension() {
        let dir = TempDir::new().unwrap();
        
        create_test_photo_root(&dir, "photo1.JPG", b"uppercase");
        create_test_photo_root(&dir, "photo2.Jpg", b"mixed");
        create_test_photo_root(&dir, "photo3.jpg", b"lowercase");
        create_test_photo_root(&dir, "photo4.CR3", b"raw uppercase");
        
        let config = ClassifyConfig::default();
        let result = scan_photos(&dir.path().to_string_lossy(), &config);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total_files, 4);
    }
}
