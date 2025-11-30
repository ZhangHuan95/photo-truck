//! 集成测试模块
//! 
//! 这些测试验证各模块之间的协作是否正确

use photo_truck_lib::classify::{ClassifyConfig, is_supported_photo, get_preset_templates};
use photo_truck_lib::exif::{PhotoMetadata, check_exiftool};
use photo_truck_lib::hash::{calculate_hash, Deduplicator};
use photo_truck_lib::transfer::{scan_photos, format_size, PhotoInfo, ScanResult};

use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

// ==================== 辅助函数 ====================

fn create_test_file(dir: &TempDir, path: &str, content: &[u8]) -> String {
    let full_path = dir.path().join(path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = File::create(&full_path).unwrap();
    file.write_all(content).unwrap();
    full_path.to_string_lossy().to_string()
}

// ==================== 端到端工作流测试 ====================

#[test]
fn test_full_scan_workflow() {
    // 模拟完整的扫描工作流
    let dir = TempDir::new().unwrap();
    
    // 创建测试照片
    create_test_file(&dir, "photos/IMG_0001.jpg", b"photo content 1");
    create_test_file(&dir, "photos/IMG_0002.cr3", b"photo content 2");
    create_test_file(&dir, "photos/subdir/IMG_0003.nef", b"photo content 3");
    create_test_file(&dir, "photos/document.pdf", b"not a photo");
    
    // 使用默认配置
    let config = ClassifyConfig::default();
    
    // 扫描照片
    let source_dir = dir.path().join("photos").to_string_lossy().to_string();
    let result = scan_photos(&source_dir, &config);
    
    assert!(result.is_ok());
    let scan_result = result.unwrap();
    
    // 验证结果
    assert_eq!(scan_result.total_files, 3);  // 只有照片文件
    assert!(scan_result.total_size > 0);
    
    // 验证照片信息
    for photo in &scan_result.photos {
        assert!(is_supported_photo(&photo.path));
        assert!(!photo.file_name.is_empty());
    }
}

#[test]
fn test_classification_with_templates() {
    // 测试不同模板的分类效果
    let templates = get_preset_templates();
    
    let metadata = PhotoMetadata {
        file_path: "/test/IMG_0001.CR3".to_string(),
        file_name: "IMG_0001.CR3".to_string(),
        file_size: 25_000_000,
        date_time_original: Some("2024:12:25 10:30:00".to_string()),
        create_date: None,
        model: Some("Canon EOS R5".to_string()),
        make: Some("Canon".to_string()),
        mime_type: Some("image/x-canon-cr3".to_string()),
    };
    
    // 测试每个预设模板
    for (name, template) in &templates {
        let config = ClassifyConfig {
            template: template.to_string(),
            fallback_folder: "未知日期".to_string(),
        };
        
        let path = config.generate_path(&metadata);
        println!("模板 '{}' ({}) -> {}", name, template, path);
        
        // 路径不应该是 fallback（因为有日期）
        assert_ne!(path, "未知日期", "模板 {} 不应使用 fallback", name);
        
        // 路径应该包含年份
        assert!(path.contains("2024"), "模板 {} 应该包含年份", name);
    }
}

#[test]
fn test_deduplication_workflow() {
    let dir = TempDir::new().unwrap();
    
    // 创建原始照片和副本
    let original_content = b"original photo content for deduplication test";
    let different_content = b"different photo content";
    
    let original = create_test_file(&dir, "original.jpg", original_content);
    let copy1 = create_test_file(&dir, "copy1.jpg", original_content);
    let copy2 = create_test_file(&dir, "subfolder/copy2.jpg", original_content);
    let different = create_test_file(&dir, "different.jpg", different_content);
    
    let mut dedup = Deduplicator::new();
    
    // 添加原始文件
    let result = dedup.check_duplicate(&original, original_content.len() as u64);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // 第一个不是重复
    
    // 检查副本1
    let result = dedup.check_duplicate(&copy1, original_content.len() as u64);
    assert!(result.is_ok());
    assert!(result.unwrap().is_some()); // 是重复的
    
    // 检查副本2
    let result = dedup.check_duplicate(&copy2, original_content.len() as u64);
    assert!(result.is_ok());
    assert!(result.unwrap().is_some()); // 也是重复的
    
    // 检查不同的文件
    let result = dedup.check_duplicate(&different, different_content.len() as u64);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // 不是重复
}

#[test]
fn test_hash_consistency() {
    let dir = TempDir::new().unwrap();
    
    let content = b"test content for hash consistency";
    let file1 = create_test_file(&dir, "file1.jpg", content);
    let file2 = create_test_file(&dir, "file2.jpg", content);
    let file3 = create_test_file(&dir, "different.jpg", b"different");
    
    let hash1 = calculate_hash(&file1).unwrap();
    let hash2 = calculate_hash(&file2).unwrap();
    let hash3 = calculate_hash(&file3).unwrap();
    
    // 相同内容应该有相同哈希
    assert_eq!(hash1, hash2);
    
    // 不同内容应该有不同哈希
    assert_ne!(hash1, hash3);
    
    // 哈希应该是有效的 SHA-256（64位十六进制）
    assert_eq!(hash1.len(), 64);
    assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
}

// ==================== 扩展名支持测试 ====================

#[test]
fn test_all_supported_extensions() {
    // RAW 格式
    let raw_extensions = vec![
        "cr3", "cr2", "crw",  // Canon
        "nef", "nrw",         // Nikon
        "arw", "srf", "sr2",  // Sony
        "orf",                // Olympus
        "raf",                // Fujifilm
        "rw2",                // Panasonic
        "pef", "dng",         // Pentax/DNG
    ];
    
    for ext in &raw_extensions {
        let filename = format!("photo.{}", ext);
        assert!(is_supported_photo(&filename), "应支持 RAW 格式: {}", ext);
        
        // 大写也应该支持
        let filename_upper = format!("photo.{}", ext.to_uppercase());
        assert!(is_supported_photo(&filename_upper), "应支持大写 RAW 格式: {}", ext.to_uppercase());
    }
    
    // 通用格式
    let common_extensions = vec![
        "jpg", "jpeg", "png", "tiff", "tif", "heic", "heif", "webp", "bmp", "gif"
    ];
    
    for ext in &common_extensions {
        let filename = format!("photo.{}", ext);
        assert!(is_supported_photo(&filename), "应支持通用格式: {}", ext);
    }
}

#[test]
fn test_unsupported_extensions() {
    let unsupported = vec![
        "mp4", "mov", "avi",    // 视频
        "mp3", "wav", "flac",   // 音频
        "pdf", "doc", "docx",   // 文档
        "txt", "md", "rs",      // 文本
        "zip", "tar", "gz",     // 压缩包
    ];
    
    for ext in &unsupported {
        let filename = format!("file.{}", ext);
        assert!(!is_supported_photo(&filename), "不应支持: {}", ext);
    }
}

// ==================== 文件大小格式化测试 ====================

#[test]
fn test_format_size_real_world_sizes() {
    // 典型照片大小
    assert_eq!(format_size(5 * 1024 * 1024), "5.00 MB");       // 5MB JPEG
    assert_eq!(format_size(25 * 1024 * 1024), "25.00 MB");     // 25MB RAW
    assert_eq!(format_size(60 * 1024 * 1024), "60.00 MB");     // 60MB 高分辨率 RAW
    
    // 存储卡/硬盘容量
    assert_eq!(format_size(32u64 * 1024 * 1024 * 1024), "32.00 GB");
    assert_eq!(format_size(256u64 * 1024 * 1024 * 1024), "256.00 GB");
}

// ==================== 边界情况测试 ====================

#[test]
fn test_empty_directory_scan() {
    let dir = TempDir::new().unwrap();
    let config = ClassifyConfig::default();
    
    let result = scan_photos(&dir.path().to_string_lossy(), &config);
    assert!(result.is_ok());
    
    let scan_result = result.unwrap();
    assert_eq!(scan_result.total_files, 0);
    assert_eq!(scan_result.total_size, 0);
    assert!(scan_result.photos.is_empty());
}

#[test]
fn test_deep_directory_scan() {
    let dir = TempDir::new().unwrap();
    
    // 创建深层嵌套目录
    create_test_file(&dir, "a/b/c/d/e/f/deep.jpg", b"deep photo");
    
    let config = ClassifyConfig::default();
    let result = scan_photos(&dir.path().to_string_lossy(), &config);
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().total_files, 1);
}

#[test]
fn test_many_files_scan() {
    let dir = TempDir::new().unwrap();
    
    // 创建多个文件
    for i in 0..100 {
        let filename = format!("photo_{:04}.jpg", i);
        create_test_file(&dir, &filename, format!("content {}", i).as_bytes());
    }
    
    let config = ClassifyConfig::default();
    let result = scan_photos(&dir.path().to_string_lossy(), &config);
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().total_files, 100);
}

// ==================== ExifTool 依赖测试 ====================

#[test]
fn test_exiftool_availability() {
    let result = check_exiftool();
    
    // 记录结果，不管是否安装都不应 panic
    match &result {
        Ok(version) => {
            println!("ExifTool 已安装，版本: {}", version);
            // 验证版本格式
            assert!(!version.is_empty());
        }
        Err(e) => {
            println!("ExifTool 未安装或检查失败: {}", e);
            // 错误信息应该有用
            assert!(!e.is_empty());
        }
    }
}

// ==================== 配置测试 ====================

#[test]
fn test_classify_config_serialization() {
    let config = ClassifyConfig {
        template: "{year}/{month}/{day}".to_string(),
        fallback_folder: "未分类照片".to_string(),
    };
    
    // 序列化
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("{year}/{month}/{day}"));
    
    // 反序列化
    let restored: ClassifyConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.template, config.template);
    assert_eq!(restored.fallback_folder, config.fallback_folder);
}

#[test]
fn test_photo_info_serialization() {
    let info = PhotoInfo {
        path: "/test/photo.cr3".to_string(),
        file_name: "photo.cr3".to_string(),
        file_size: 25_000_000,
        date_time: Some("2024:12:25 10:30:00".to_string()),
        camera: Some("Canon EOS R5".to_string()),
        target_folder: "Canon/2024/12".to_string(),
        is_duplicate: false,
        duplicate_of: None,
    };
    
    // 序列化
    let json = serde_json::to_string(&info).unwrap();
    
    // 反序列化
    let restored: PhotoInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.path, info.path);
    assert_eq!(restored.file_size, info.file_size);
    assert_eq!(restored.camera, info.camera);
}

#[test]
fn test_scan_result_serialization() {
    let result = ScanResult {
        total_files: 10,
        total_size: 250_000_000,
        photos: vec![
            PhotoInfo {
                path: "/test/photo1.jpg".to_string(),
                file_name: "photo1.jpg".to_string(),
                file_size: 5_000_000,
                date_time: None,
                camera: None,
                target_folder: "未知日期".to_string(),
                is_duplicate: false,
                duplicate_of: None,
            }
        ],
    };
    
    let json = serde_json::to_string(&result).unwrap();
    let restored: ScanResult = serde_json::from_str(&json).unwrap();
    
    assert_eq!(restored.total_files, result.total_files);
    assert_eq!(restored.photos.len(), result.photos.len());
}

// ==================== 并发安全测试 ====================

#[test]
fn test_multiple_scans() {
    let dir = TempDir::new().unwrap();
    
    create_test_file(&dir, "photo.jpg", b"content");
    
    let config = ClassifyConfig::default();
    let path = dir.path().to_string_lossy().to_string();
    
    // 多次扫描应该得到相同结果
    let result1 = scan_photos(&path, &config).unwrap();
    let result2 = scan_photos(&path, &config).unwrap();
    let result3 = scan_photos(&path, &config).unwrap();
    
    assert_eq!(result1.total_files, result2.total_files);
    assert_eq!(result2.total_files, result3.total_files);
}

#[test]
fn test_deduplicator_isolation() {
    let dir = TempDir::new().unwrap();
    let file = create_test_file(&dir, "test.jpg", b"content");
    
    // 不同的 Deduplicator 实例应该独立
    let mut dedup1 = Deduplicator::new();
    let mut dedup2 = Deduplicator::new();
    
    // 在 dedup1 中标记为已存在
    let _ = dedup1.check_duplicate(&file, 7);
    
    // dedup2 应该认为是新文件
    let result = dedup2.check_duplicate(&file, 7);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // 对 dedup2 来说是新文件
}
