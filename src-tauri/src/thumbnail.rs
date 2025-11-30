
use std::path::Path;
use std::process::{Command, Stdio};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// 缩略图信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThumbnailInfo {
    pub file_path: String,
    pub data: String,  // Base64 编码的图片数据
    pub width: u32,
    pub height: u32,
    pub format: String,
}

/// 缩略图大小
pub const THUMBNAIL_SIZE: u32 = 160;

/// 使用 ExifTool 提取内嵌缩略图
pub fn extract_thumbnail(file_path: &str) -> Result<ThumbnailInfo, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    // 尝试使用 ExifTool 提取缩略图
    let exiftool_path = crate::exif::get_exiftool_path()
        .ok_or("ExifTool 未安装")?;

    // 提取缩略图数据
    let output = Command::new(&exiftool_path)
        .args(["-b", "-ThumbnailImage", file_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("执行 ExifTool 失败: {}", e))?;

    if output.stdout.is_empty() {
        // 尝试提取预览图
        let output = Command::new(&exiftool_path)
            .args(["-b", "-PreviewImage", file_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("执行 ExifTool 失败: {}", e))?;

        if output.stdout.is_empty() {
            return Err("无法提取缩略图".to_string());
        }

        let base64_data = BASE64.encode(&output.stdout);
        return Ok(ThumbnailInfo {
            file_path: file_path.to_string(),
            data: base64_data,
            width: THUMBNAIL_SIZE,
            height: THUMBNAIL_SIZE,
            format: "image/jpeg".to_string(),
        });
    }

    let base64_data = BASE64.encode(&output.stdout);
    Ok(ThumbnailInfo {
        file_path: file_path.to_string(),
        data: base64_data,
        width: THUMBNAIL_SIZE,
        height: THUMBNAIL_SIZE,
        format: "image/jpeg".to_string(),
    })
}

/// 批量提取缩略图
pub fn extract_thumbnails(file_paths: &[String], max_count: usize) -> Vec<ThumbnailInfo> {
    let mut thumbnails = Vec::new();
    let count = std::cmp::min(file_paths.len(), max_count);

    for path in file_paths.iter().take(count) {
        if let Ok(thumb) = extract_thumbnail(path) {
            thumbnails.push(thumb);
        }
    }

    thumbnails
}

/// 检查文件是否有内嵌缩略图
pub fn has_embedded_thumbnail(file_path: &str) -> bool {
    let exiftool_path = match crate::exif::get_exiftool_path() {
        Some(p) => p,
        None => return false,
    };

    let output = Command::new(&exiftool_path)
        .args(["-ThumbnailImage", "-PreviewImage", file_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_size() {
        assert_eq!(THUMBNAIL_SIZE, 160);
    }

    #[test]
    fn test_extract_thumbnail_nonexistent() {
        let result = extract_thumbnail("/nonexistent/file.jpg");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_thumbnails_empty() {
        let paths: Vec<String> = vec![];
        let result = extract_thumbnails(&paths, 10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_has_embedded_thumbnail_nonexistent() {
        let result = has_embedded_thumbnail("/nonexistent/file.jpg");
        assert!(!result);
    }
}
