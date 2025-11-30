// 测试模块
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exif_datetime_standard() {
        let dt = parse_exif_datetime("2024:03:15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_exif_datetime_dash_format() {
        let dt = parse_exif_datetime("2024-03-15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
    }

    #[test]
    fn test_parse_exif_datetime_date_only() {
        let dt = parse_exif_datetime("2024:03:15").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_exif_datetime_invalid() {
        let result = parse_exif_datetime("invalid date");
        assert!(result.is_none());
    }

    #[test]
    fn test_sanitize_folder_name() {
        assert_eq!(sanitize_folder_name("Canon EOS R5"), "Canon EOS R5");
        assert_eq!(sanitize_folder_name("Test/Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test:Camera"), "Test_Camera");
        assert_eq!(sanitize_folder_name("Test*Camera?"), "Test_Camera_");
        assert_eq!(sanitize_folder_name("  Trimmed  "), "Trimmed");
    }

    #[test]
    fn test_classify_config_default() {
        let config = ClassifyConfig::default();
        assert_eq!(config.template, "{year}/{month}");
        assert_eq!(config.fallback_folder, "未知日期");
    }

    #[test]
    fn test_generate_path_with_date() {
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
    fn test_generate_path_fallback() {
        let config = ClassifyConfig {
            template: "{year}/{month}".to_string(),
            fallback_folder: "无日期".to_string(),
        };
        let metadata = PhotoMetadata {
            date_time_original: None,
            create_date: None,
            ..Default::default()
        };
        let path = config.generate_path(&metadata);
        assert_eq!(path, "无日期");
    }

    #[test]
    fn test_is_supported_photo() {
        assert!(is_supported_photo("photo.jpg"));
        assert!(is_supported_photo("photo.JPG"));
        assert!(is_supported_photo("photo.cr3"));
        assert!(is_supported_photo("photo.CR3"));
        assert!(is_supported_photo("photo.nef"));
        assert!(is_supported_photo("photo.arw"));
        assert!(is_supported_photo("photo.heic"));
        assert!(!is_supported_photo("document.pdf"));
        assert!(!is_supported_photo("video.mp4"));
        assert!(!is_supported_photo("noextension"));
    }

    #[test]
    fn test_get_preset_templates() {
        let templates = get_preset_templates();
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|(name, _)| *name == "按年/月"));
        assert!(templates.iter().any(|(_, tpl)| *tpl == "{year}/{month}"));
    }
}

use crate::exif::PhotoMetadata;
use chrono::{Datelike, NaiveDateTime, Timelike};
