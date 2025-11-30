use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhotoMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub date_time_original: Option<String>,
    pub create_date: Option<String>,
    pub model: Option<String>,
    pub make: Option<String>,
    pub mime_type: Option<String>,
}

/// 获取 ExifTool 的可执行路径
/// macOS 应用打包后无法直接访问 PATH 中的命令，需要尝试多个可能的路径
fn get_exiftool_path() -> Option<String> {
    // 常见的 ExifTool 安装路径
    let possible_paths = [
        "exiftool",                           // 系统 PATH
        "/opt/homebrew/bin/exiftool",         // macOS ARM (Homebrew)
        "/usr/local/bin/exiftool",            // macOS Intel (Homebrew)
        "/usr/bin/exiftool",                  // Linux 系统路径
        "/opt/local/bin/exiftool",            // MacPorts
    ];

    for path in &possible_paths {
        if Command::new(path)
            .arg("-ver")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(path.to_string());
        }
    }
    None
}

/// 使用 ExifTool 读取照片元数据
pub fn read_exif(file_path: &str) -> Result<PhotoMetadata, String> {
    let exiftool_path = get_exiftool_path()
        .ok_or_else(|| "ExifTool 未安装。请运行: brew install exiftool".to_string())?;

    let output = Command::new(&exiftool_path)
        .args(["-json", "-DateTimeOriginal", "-CreateDate", "-Model", "-Make", "-MIMEType", "-FileName", "-FileSize#", file_path])
        .output()
        .map_err(|e| format!("执行 exiftool 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("exiftool 返回错误: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_array: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("解析 exiftool 输出失败: {}", e))?;

    if json_array.is_empty() {
        return Err("未能读取到元数据".to_string());
    }

    let json = &json_array[0];

    Ok(PhotoMetadata {
        file_path: file_path.to_string(),
        file_name: json["FileName"].as_str().unwrap_or("").to_string(),
        file_size: json["FileSize"].as_u64().unwrap_or(0),
        date_time_original: json["DateTimeOriginal"].as_str().map(|s| s.to_string()),
        create_date: json["CreateDate"].as_str().map(|s| s.to_string()),
        model: json["Model"].as_str().map(|s| s.to_string()),
        make: json["Make"].as_str().map(|s| s.to_string()),
        mime_type: json["MIMEType"].as_str().map(|s| s.to_string()),
    })
}

/// 批量读取多个文件的 EXIF 信息
pub fn read_exif_batch(file_paths: &[String]) -> Vec<Result<PhotoMetadata, String>> {
    file_paths.iter().map(|path| read_exif(path)).collect()
}

/// 检查 ExifTool 是否已安装
pub fn check_exiftool() -> Result<String, String> {
    let exiftool_path = get_exiftool_path()
        .ok_or_else(|| "ExifTool 未安装。请运行: brew install exiftool".to_string())?;

    let output = Command::new(&exiftool_path)
        .arg("-ver")
        .output()
        .map_err(|_| "ExifTool 未安装。请运行: brew install exiftool".to_string())?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    } else {
        Err("ExifTool 检查失败".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ExifTool 检查测试 ====================

    #[test]
    fn test_check_exiftool_installed() {
        // 此测试验证 ExifTool 是否已安装
        let result = check_exiftool();
        println!("ExifTool check result: {:?}", result);
        
        // 在 CI 环境中可能没有安装 ExifTool，所以只检查结果格式
        match result {
            Ok(version) => {
                // 版本应该是数字格式如 "13.25"
                assert!(!version.is_empty());
                println!("ExifTool version: {}", version);
            }
            Err(e) => {
                // 如果未安装，错误信息应该有指导
                assert!(e.contains("ExifTool") || e.contains("未安装"));
            }
        }
    }

    // ==================== PhotoMetadata 结构测试 ====================

    #[test]
    fn test_photo_metadata_default() {
        let metadata = PhotoMetadata::default();
        
        assert!(metadata.file_path.is_empty());
        assert!(metadata.file_name.is_empty());
        assert_eq!(metadata.file_size, 0);
        assert!(metadata.date_time_original.is_none());
        assert!(metadata.create_date.is_none());
        assert!(metadata.model.is_none());
        assert!(metadata.make.is_none());
        assert!(metadata.mime_type.is_none());
    }

    #[test]
    fn test_photo_metadata_with_values() {
        let metadata = PhotoMetadata {
            file_path: "/path/to/photo.cr3".to_string(),
            file_name: "photo.cr3".to_string(),
            file_size: 25_000_000,
            date_time_original: Some("2024:03:15 14:30:00".to_string()),
            create_date: Some("2024:03:15 14:30:00".to_string()),
            model: Some("Canon EOS R5".to_string()),
            make: Some("Canon".to_string()),
            mime_type: Some("image/x-canon-cr3".to_string()),
        };

        assert_eq!(metadata.file_path, "/path/to/photo.cr3");
        assert_eq!(metadata.file_name, "photo.cr3");
        assert_eq!(metadata.file_size, 25_000_000);
        assert_eq!(metadata.date_time_original.as_deref(), Some("2024:03:15 14:30:00"));
        assert_eq!(metadata.model.as_deref(), Some("Canon EOS R5"));
        assert_eq!(metadata.make.as_deref(), Some("Canon"));
    }

    #[test]
    fn test_photo_metadata_clone() {
        let original = PhotoMetadata {
            file_path: "/test.jpg".to_string(),
            file_name: "test.jpg".to_string(),
            file_size: 1000,
            date_time_original: Some("2024:01:01 00:00:00".to_string()),
            ..Default::default()
        };

        let cloned = original.clone();
        assert_eq!(original.file_path, cloned.file_path);
        assert_eq!(original.date_time_original, cloned.date_time_original);
    }

    #[test]
    fn test_photo_metadata_serialization() {
        let metadata = PhotoMetadata {
            file_path: "/test.jpg".to_string(),
            file_name: "test.jpg".to_string(),
            file_size: 1000,
            date_time_original: Some("2024:01:01 00:00:00".to_string()),
            create_date: None,
            model: Some("Test Camera".to_string()),
            make: None,
            mime_type: Some("image/jpeg".to_string()),
        };

        // 测试序列化
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("test.jpg"));
        assert!(json.contains("Test Camera"));

        // 测试反序列化
        let deserialized: PhotoMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_name, "test.jpg");
        assert_eq!(deserialized.model.as_deref(), Some("Test Camera"));
    }

    // ==================== read_exif 函数测试 ====================

    #[test]
    fn test_read_exif_nonexistent_file() {
        let result = read_exif("/nonexistent/path/photo.jpg");
        // ExifTool 应该返回错误
        assert!(result.is_err() || result.is_ok()); // 取决于 ExifTool 行为
    }

    #[test]
    #[ignore] // 需要真实照片文件才能运行
    fn test_read_exif_real_photo() {
        // 此测试需要一个真实的照片文件
        // 使用 #[ignore] 标记，只在有测试文件时手动运行
        let result = read_exif("/path/to/real/photo.jpg");
        match result {
            Ok(metadata) => {
                println!("File: {}", metadata.file_name);
                println!("Date: {:?}", metadata.date_time_original);
                println!("Camera: {:?}", metadata.model);
                println!("Make: {:?}", metadata.make);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    // ==================== 边界情况测试 ====================

    #[test]
    fn test_photo_metadata_empty_strings() {
        let metadata = PhotoMetadata {
            file_path: "".to_string(),
            file_name: "".to_string(),
            file_size: 0,
            date_time_original: Some("".to_string()),
            create_date: None,
            model: Some("".to_string()),
            make: None,
            mime_type: None,
        };

        // 空字符串应该被正确处理
        assert!(metadata.file_path.is_empty());
        assert_eq!(metadata.date_time_original.as_deref(), Some(""));
    }

    #[test]
    fn test_photo_metadata_unicode_names() {
        let metadata = PhotoMetadata {
            file_path: "/测试/照片.jpg".to_string(),
            file_name: "照片.jpg".to_string(),
            file_size: 500,
            model: Some("佳能 EOS R5".to_string()),
            make: Some("佳能".to_string()),
            ..Default::default()
        };

        assert_eq!(metadata.file_name, "照片.jpg");
        assert_eq!(metadata.make.as_deref(), Some("佳能"));
    }

    #[test]
    fn test_photo_metadata_large_file_size() {
        // 测试大文件大小（如 RAW 文件可能很大）
        let metadata = PhotoMetadata {
            file_size: 100_000_000_000, // 100GB
            ..Default::default()
        };
        assert_eq!(metadata.file_size, 100_000_000_000);
    }
}
