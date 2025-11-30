use crate::exif::PhotoMetadata;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 分类规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyConfig {
    /// 分类模板，支持以下占位符：
    /// {year} - 年份 (4位)
    /// {month} - 月份 (2位)
    /// {day} - 日期 (2位)
    /// {camera} - 相机型号
    /// {make} - 相机品牌
    pub template: String,
    
    /// 当无法获取日期时使用的备用文件夹名
    pub fallback_folder: String,
}

impl Default for ClassifyConfig {
    fn default() -> Self {
        Self {
            template: "{year}/{month}".to_string(),
            fallback_folder: "未知日期".to_string(),
        }
    }
}

impl ClassifyConfig {
    /// 根据照片元数据生成分类路径
    pub fn generate_path(&self, metadata: &PhotoMetadata) -> String {
        let mut path = self.template.clone();
        
        // 尝试解析日期时间
        let datetime = metadata.date_time_original
            .as_ref()
            .or(metadata.create_date.as_ref())
            .and_then(|dt| parse_exif_datetime(dt));

        if let Some(dt) = datetime {
            path = path.replace("{year}", &format!("{:04}", dt.year()));
            path = path.replace("{month}", &format!("{:02}", dt.month()));
            path = path.replace("{day}", &format!("{:02}", dt.day()));
        } else {
            // 无法解析日期，使用备用文件夹
            return self.fallback_folder.clone();
        }

        // 替换相机信息
        let camera = metadata.model.as_deref().unwrap_or("未知相机");
        let make = metadata.make.as_deref().unwrap_or("未知品牌");
        
        path = path.replace("{camera}", &sanitize_folder_name(camera));
        path = path.replace("{make}", &sanitize_folder_name(make));

        path
    }
}

/// 解析 EXIF 日期时间格式 (YYYY:MM:DD HH:MM:SS)
fn parse_exif_datetime(datetime_str: &str) -> Option<NaiveDateTime> {
    // EXIF 标准格式
    if let Ok(dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y:%m:%d %H:%M:%S") {
        return Some(dt);
    }
    
    // 尝试其他常见格式
    if let Ok(dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }

    // 只有日期的情况
    if let Ok(date) = chrono::NaiveDate::parse_from_str(datetime_str, "%Y:%m:%d") {
        return Some(date.and_hms_opt(0, 0, 0)?);
    }

    None
}

/// 清理文件夹名称中的非法字符
fn sanitize_folder_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// 支持的照片文件扩展名
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // RAW 格式
    "cr3", "cr2", "crw",    // Canon
    "nef", "nrw",           // Nikon
    "arw", "srf", "sr2",    // Sony
    "orf",                   // Olympus
    "raf",                   // Fujifilm
    "rw2",                   // Panasonic
    "pef", "dng",           // Pentax / Adobe DNG
    "raw", "rwl",           // Leica
    "3fr",                   // Hasselblad
    "erf",                   // Epson
    "kdc", "dcr",           // Kodak
    "x3f",                   // Sigma
    
    // 通用格式
    "jpg", "jpeg",
    "png",
    "tiff", "tif",
    "heic", "heif",
    "webp",
    "bmp",
    "gif",
];

/// 检查文件是否为支持的照片格式
pub fn is_supported_photo(file_path: &str) -> bool {
    Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 预设的分类模板
pub fn get_preset_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("按年/月", "{year}/{month}"),
        ("按年/月/日", "{year}/{month}/{day}"),
        ("按年/月-日", "{year}/{month}-{day}"),
        ("按品牌/年/月", "{make}/{year}/{month}"),
        ("按相机/年/月", "{camera}/{year}/{month}"),
        ("按年/相机/月", "{year}/{camera}/{month}"),
    ]
}

use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    // ==================== 日期解析测试 ====================
    
    #[test]
    fn test_parse_exif_datetime_standard_format() {
        // 标准 EXIF 格式: YYYY:MM:DD HH:MM:SS
        let dt = parse_exif_datetime("2024:03:15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_parse_exif_datetime_dash_format() {
        // 备用格式: YYYY-MM-DD HH:MM:SS
        let dt = parse_exif_datetime("2024-03-15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_exif_datetime_date_only() {
        // 只有日期的情况
        let dt = parse_exif_datetime("2024:03:15").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_parse_exif_datetime_invalid() {
        // 无效格式
        assert!(parse_exif_datetime("invalid date").is_none());
        assert!(parse_exif_datetime("").is_none());
        assert!(parse_exif_datetime("2024/03/15").is_none());
        assert!(parse_exif_datetime("03-15-2024").is_none());
    }

    #[test]
    fn test_parse_exif_datetime_edge_cases() {
        // 边界情况：年初
        let dt = parse_exif_datetime("2024:01:01 00:00:00").unwrap();
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
        
        // 边界情况：年末
        let dt = parse_exif_datetime("2024:12:31 23:59:59").unwrap();
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 31);
    }

    // ==================== 文件夹名称清理测试 ====================

    #[test]
    fn test_sanitize_folder_name_normal() {
        assert_eq!(sanitize_folder_name("Canon EOS R5"), "Canon EOS R5");
        assert_eq!(sanitize_folder_name("Nikon Z6"), "Nikon Z6");
        assert_eq!(sanitize_folder_name("iPhone 15 Pro"), "iPhone 15 Pro");
    }

    #[test]
    fn test_sanitize_folder_name_special_chars() {
        // 各种非法字符
        assert_eq!(sanitize_folder_name("Test/Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test\\Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test:Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test*Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test?Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test\"Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test<Camera>"), "Test_Camera_");
        assert_eq!(sanitize_folder_name("Test|Camera"), "Test_Camera");
    }

    #[test]
    fn test_sanitize_folder_name_whitespace() {
        // 前后空格应该被去除
        assert_eq!(sanitize_folder_name("  Trimmed  "), "Trimmed");
        assert_eq!(sanitize_folder_name("\tTabs\t"), "Tabs");
    }

    #[test]
    fn test_sanitize_folder_name_multiple_special() {
        // 多个连续特殊字符
        assert_eq!(sanitize_folder_name("A///B"), "A___B");
        assert_eq!(sanitize_folder_name("A:*?B"), "A___B");
    }

    // ==================== 分类配置测试 ====================

    #[test]
    fn test_classify_config_default() {
        let config = ClassifyConfig::default();
        assert_eq!(config.template, "{year}/{month}");
        assert_eq!(config.fallback_folder, "未知日期");
    }

    #[test]
    fn test_generate_path_year_month() {
        let config = ClassifyConfig::default();
        let metadata = PhotoMetadata {
            date_time_original: Some("2024:03:15 14:30:00".to_string()),
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "2024/03");
    }

    #[test]
    fn test_generate_path_year_month_day() {
        let config = ClassifyConfig {
            template: "{year}/{month}/{day}".to_string(),
            fallback_folder: "未知日期".to_string(),
        };
        let metadata = PhotoMetadata {
            date_time_original: Some("2024:03:15 14:30:00".to_string()),
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "2024/03/15");
    }

    #[test]
    fn test_generate_path_with_camera() {
        let config = ClassifyConfig {
            template: "{year}/{camera}".to_string(),
            fallback_folder: "未知日期".to_string(),
        };
        let metadata = PhotoMetadata {
            date_time_original: Some("2024:03:15 14:30:00".to_string()),
            model: Some("Canon EOS R5".to_string()),
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "2024/Canon EOS R5");
    }

    #[test]
    fn test_generate_path_with_make() {
        let config = ClassifyConfig {
            template: "{make}/{year}/{month}".to_string(),
            fallback_folder: "未知日期".to_string(),
        };
        let metadata = PhotoMetadata {
            date_time_original: Some("2024:03:15 14:30:00".to_string()),
            make: Some("Canon".to_string()),
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "Canon/2024/03");
    }

    #[test]
    fn test_generate_path_unknown_camera() {
        let config = ClassifyConfig {
            template: "{year}/{camera}".to_string(),
            fallback_folder: "未知日期".to_string(),
        };
        let metadata = PhotoMetadata {
            date_time_original: Some("2024:03:15 14:30:00".to_string()),
            model: None,
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "2024/未知相机");
    }

    #[test]
    fn test_generate_path_fallback_no_date() {
        let config = ClassifyConfig {
            template: "{year}/{month}".to_string(),
            fallback_folder: "无日期照片".to_string(),
        };
        let metadata = PhotoMetadata {
            date_time_original: None,
            create_date: None,
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "无日期照片");
    }

    #[test]
    fn test_generate_path_use_create_date() {
        // 当 DateTimeOriginal 不存在时，使用 CreateDate
        let config = ClassifyConfig::default();
        let metadata = PhotoMetadata {
            date_time_original: None,
            create_date: Some("2024:06:20 10:00:00".to_string()),
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "2024/06");
    }

    #[test]
    fn test_generate_path_special_camera_name() {
        // 相机名称包含特殊字符的情况
        let config = ClassifyConfig {
            template: "{camera}/{year}".to_string(),
            fallback_folder: "未知日期".to_string(),
        };
        let metadata = PhotoMetadata {
            date_time_original: Some("2024:03:15 14:30:00".to_string()),
            model: Some("Canon/Nikon*Test".to_string()),
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "Canon_Nikon_Test/2024");
    }

    // ==================== 支持的照片格式测试 ====================

    #[test]
    fn test_is_supported_photo_raw_formats() {
        // Canon RAW
        assert!(is_supported_photo("photo.cr3"));
        assert!(is_supported_photo("photo.CR3"));
        assert!(is_supported_photo("photo.cr2"));
        assert!(is_supported_photo("photo.crw"));
        
        // Nikon RAW
        assert!(is_supported_photo("photo.nef"));
        assert!(is_supported_photo("photo.NEF"));
        assert!(is_supported_photo("photo.nrw"));
        
        // Sony RAW
        assert!(is_supported_photo("photo.arw"));
        assert!(is_supported_photo("photo.ARW"));
        assert!(is_supported_photo("photo.srf"));
        assert!(is_supported_photo("photo.sr2"));
        
        // Other RAW
        assert!(is_supported_photo("photo.orf"));   // Olympus
        assert!(is_supported_photo("photo.raf"));   // Fujifilm
        assert!(is_supported_photo("photo.rw2"));   // Panasonic
        assert!(is_supported_photo("photo.pef"));   // Pentax
        assert!(is_supported_photo("photo.dng"));   // Adobe DNG
    }

    #[test]
    fn test_is_supported_photo_common_formats() {
        // JPEG
        assert!(is_supported_photo("photo.jpg"));
        assert!(is_supported_photo("photo.JPG"));
        assert!(is_supported_photo("photo.jpeg"));
        assert!(is_supported_photo("photo.JPEG"));
        
        // PNG
        assert!(is_supported_photo("photo.png"));
        assert!(is_supported_photo("photo.PNG"));
        
        // TIFF
        assert!(is_supported_photo("photo.tiff"));
        assert!(is_supported_photo("photo.tif"));
        
        // HEIC (Apple)
        assert!(is_supported_photo("photo.heic"));
        assert!(is_supported_photo("photo.HEIC"));
        assert!(is_supported_photo("photo.heif"));
        
        // WebP
        assert!(is_supported_photo("photo.webp"));
        
        // Others
        assert!(is_supported_photo("photo.bmp"));
        assert!(is_supported_photo("photo.gif"));
    }

    #[test]
    fn test_is_supported_photo_unsupported() {
        // 不支持的格式
        assert!(!is_supported_photo("document.pdf"));
        assert!(!is_supported_photo("video.mp4"));
        assert!(!is_supported_photo("video.mov"));
        assert!(!is_supported_photo("audio.mp3"));
        assert!(!is_supported_photo("text.txt"));
        assert!(!is_supported_photo("code.rs"));
    }

    #[test]
    fn test_is_supported_photo_edge_cases() {
        // 没有扩展名
        assert!(!is_supported_photo("noextension"));
        assert!(!is_supported_photo("photo."));
        
        // 路径中的照片
        assert!(is_supported_photo("/path/to/photo.jpg"));
        assert!(is_supported_photo("./relative/photo.cr3"));
        assert!(is_supported_photo("C:\\Windows\\photo.png"));
        
        // 文件名特殊情况
        assert!(is_supported_photo(".hidden.jpg"));
        assert!(is_supported_photo("file.with.dots.jpg"));
    }

    // ==================== 预设模板测试 ====================

    #[test]
    fn test_get_preset_templates() {
        let templates = get_preset_templates();
        
        // 确保有模板
        assert!(!templates.is_empty());
        
        // 检查必要的模板存在
        let template_names: Vec<&str> = templates.iter().map(|(name, _)| *name).collect();
        assert!(template_names.contains(&"按年/月"));
        assert!(template_names.contains(&"按年/月/日"));
        assert!(template_names.contains(&"按品牌/年/月"));
        assert!(template_names.contains(&"按相机/年/月"));
        
        // 检查模板值
        let template_values: Vec<&str> = templates.iter().map(|(_, tpl)| *tpl).collect();
        assert!(template_values.contains(&"{year}/{month}"));
        assert!(template_values.contains(&"{year}/{month}/{day}"));
    }

    // ==================== 综合测试 ====================

    #[test]
    fn test_full_workflow() {
        // 模拟完整的分类工作流
        let config = ClassifyConfig {
            template: "{make}/{year}/{month}/{day}".to_string(),
            fallback_folder: "未分类".to_string(),
        };
        
        // 有完整信息的照片
        let metadata1 = PhotoMetadata {
            file_path: "/path/to/IMG_0001.CR3".to_string(),
            file_name: "IMG_0001.CR3".to_string(),
            file_size: 25_000_000,
            date_time_original: Some("2024:12:25 10:30:00".to_string()),
            create_date: None,
            model: Some("Canon EOS R5".to_string()),
            make: Some("Canon".to_string()),
            mime_type: Some("image/x-canon-cr3".to_string()),
        };
        assert_eq!(config.generate_path(&metadata1), "Canon/2024/12/25");
        
        // 缺少品牌信息的照片
        let metadata2 = PhotoMetadata {
            date_time_original: Some("2024:06:15 14:00:00".to_string()),
            make: None,
            ..Default::default()
        };
        assert_eq!(config.generate_path(&metadata2), "未知品牌/2024/06/15");
        
        // 完全没有元数据的照片
        let metadata3 = PhotoMetadata::default();
        assert_eq!(config.generate_path(&metadata3), "未分类");
    }
}
