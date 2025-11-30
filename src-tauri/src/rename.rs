use crate::exif::PhotoMetadata;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 重命名规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameConfig {
    /// 是否启用重命名
    pub enabled: bool,
    /// 重命名模板
    pub template: String,
    /// 计数器起始值
    pub counter_start: u32,
    /// 计数器位数
    pub counter_digits: u32,
}

impl Default for RenameConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            template: "{original}".to_string(),
            counter_start: 1,
            counter_digits: 4,
        }
    }
}

impl RenameConfig {
    /// 根据照片元数据生成新文件名
    /// 
    /// 支持的变量:
    /// - {original} - 原文件名（不含扩展名）
    /// - {year}, {month}, {day} - 日期
    /// - {hour}, {minute}, {second} - 时间
    /// - {camera}, {make} - 相机信息
    /// - {counter} - 自增计数器
    /// - {date} - 日期 YYYYMMDD
    /// - {time} - 时间 HHMMSS
    /// - {datetime} - 日期时间 YYYYMMDD_HHMMSS
    pub fn generate_filename(
        &self,
        metadata: &PhotoMetadata,
        counter: u32,
    ) -> String {
        if !self.enabled {
            return metadata.file_name.clone();
        }

        let original_name = Path::new(&metadata.file_name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let extension = Path::new(&metadata.file_name)
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut name = self.template.clone();

        // 替换原文件名
        name = name.replace("{original}", &original_name);

        // 替换计数器
        let counter_str = format!("{:0width$}", counter, width = self.counter_digits as usize);
        name = name.replace("{counter}", &counter_str);

        // 解析日期时间
        let datetime = metadata.date_time_original
            .as_ref()
            .or(metadata.create_date.as_ref())
            .and_then(|dt| parse_datetime(dt));

        if let Some((year, month, day, hour, minute, second)) = datetime {
            name = name.replace("{year}", &format!("{:04}", year));
            name = name.replace("{month}", &format!("{:02}", month));
            name = name.replace("{day}", &format!("{:02}", day));
            name = name.replace("{hour}", &format!("{:02}", hour));
            name = name.replace("{minute}", &format!("{:02}", minute));
            name = name.replace("{second}", &format!("{:02}", second));
            name = name.replace("{date}", &format!("{:04}{:02}{:02}", year, month, day));
            name = name.replace("{time}", &format!("{:02}{:02}{:02}", hour, minute, second));
            name = name.replace("{datetime}", &format!("{:04}{:02}{:02}_{:02}{:02}{:02}", 
                year, month, day, hour, minute, second));
        } else {
            // 无日期时移除日期相关占位符
            name = name.replace("{year}", "");
            name = name.replace("{month}", "");
            name = name.replace("{day}", "");
            name = name.replace("{hour}", "");
            name = name.replace("{minute}", "");
            name = name.replace("{second}", "");
            name = name.replace("{date}", "");
            name = name.replace("{time}", "");
            name = name.replace("{datetime}", "");
        }

        // 替换相机信息
        let camera = sanitize_filename(metadata.model.as_deref().unwrap_or(""));
        let make = sanitize_filename(metadata.make.as_deref().unwrap_or(""));
        name = name.replace("{camera}", &camera);
        name = name.replace("{make}", &make);

        // 清理文件名
        name = sanitize_filename(&name);
        
        // 移除连续的下划线或空格
        while name.contains("__") {
            name = name.replace("__", "_");
        }
        name = name.trim_matches('_').to_string();

        // 如果文件名为空，使用原文件名
        if name.is_empty() {
            name = original_name;
        }

        // 添加扩展名
        if !extension.is_empty() {
            format!("{}.{}", name, extension)
        } else {
            name
        }
    }
}

/// 解析日期时间字符串，返回 (year, month, day, hour, minute, second)
fn parse_datetime(datetime_str: &str) -> Option<(u32, u32, u32, u32, u32, u32)> {
    // EXIF 标准格式: "2024:03:15 10:30:45"
    let parts: Vec<&str> = datetime_str.split(|c| c == ':' || c == ' ').collect();
    if parts.len() >= 6 {
        let year = parts[0].parse().ok()?;
        let month = parts[1].parse().ok()?;
        let day = parts[2].parse().ok()?;
        let hour = parts[3].parse().ok()?;
        let minute = parts[4].parse().ok()?;
        let second = parts[5].parse().ok()?;
        return Some((year, month, day, hour, minute, second));
    }
    
    // 尝试 ISO 格式: "2024-03-15 10:30:45"
    let parts: Vec<&str> = datetime_str.split(|c| c == '-' || c == ' ' || c == ':').collect();
    if parts.len() >= 6 {
        let year = parts[0].parse().ok()?;
        let month = parts[1].parse().ok()?;
        let day = parts[2].parse().ok()?;
        let hour = parts[3].parse().ok()?;
        let minute = parts[4].parse().ok()?;
        let second = parts[5].parse().ok()?;
        return Some((year, month, day, hour, minute, second));
    }

    None
}

/// 清理文件名中的非法字符
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// 预设的重命名模板
pub fn get_rename_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("保持原名", "{original}"),
        ("日期_原名", "{date}_{original}"),
        ("日期时间_原名", "{datetime}_{original}"),
        ("日期_计数", "{date}_{counter}"),
        ("相机_日期_计数", "{camera}_{date}_{counter}"),
        ("年月日_计数", "{year}{month}{day}_{counter}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata(filename: &str, datetime: Option<&str>) -> PhotoMetadata {
        PhotoMetadata {
            file_path: format!("/test/{}", filename),
            file_name: filename.to_string(),
            file_size: 1000,
            date_time_original: datetime.map(|s| s.to_string()),
            create_date: None,
            make: Some("Canon".to_string()),
            model: Some("EOS R5".to_string()),
            mime_type: None,
        }
    }

    #[test]
    fn test_keep_original_name() {
        let config = RenameConfig::default();
        let metadata = create_test_metadata("IMG_0001.CR3", None);
        let result = config.generate_filename(&metadata, 1);
        assert_eq!(result, "IMG_0001.CR3");
    }

    #[test]
    fn test_rename_with_date() {
        let config = RenameConfig {
            enabled: true,
            template: "{date}_{original}".to_string(),
            counter_start: 1,
            counter_digits: 4,
        };
        let metadata = create_test_metadata("IMG_0001.CR3", Some("2024:03:15 10:30:45"));
        let result = config.generate_filename(&metadata, 1);
        assert_eq!(result, "20240315_IMG_0001.CR3");
    }

    #[test]
    fn test_rename_with_counter() {
        let config = RenameConfig {
            enabled: true,
            template: "{date}_{counter}".to_string(),
            counter_start: 1,
            counter_digits: 4,
        };
        let metadata = create_test_metadata("IMG_0001.CR3", Some("2024:03:15 10:30:45"));
        let result = config.generate_filename(&metadata, 42);
        assert_eq!(result, "20240315_0042.CR3");
    }

    #[test]
    fn test_rename_with_camera() {
        let config = RenameConfig {
            enabled: true,
            template: "{camera}_{counter}".to_string(),
            counter_start: 1,
            counter_digits: 3,
        };
        let metadata = create_test_metadata("IMG_0001.JPG", Some("2024:03:15 10:30:45"));
        let result = config.generate_filename(&metadata, 1);
        assert_eq!(result, "EOS R5_001.JPG");
    }

    #[test]
    fn test_rename_with_datetime() {
        let config = RenameConfig {
            enabled: true,
            template: "{datetime}".to_string(),
            counter_start: 1,
            counter_digits: 4,
        };
        let metadata = create_test_metadata("IMG_0001.CR3", Some("2024:03:15 10:30:45"));
        let result = config.generate_filename(&metadata, 1);
        assert_eq!(result, "20240315_103045.CR3");
    }

    #[test]
    fn test_parse_datetime_exif_format() {
        let result = parse_datetime("2024:03:15 10:30:45");
        assert_eq!(result, Some((2024, 3, 15, 10, 30, 45)));
    }

    #[test]
    fn test_parse_datetime_iso_format() {
        let result = parse_datetime("2024-03-15 10:30:45");
        assert_eq!(result, Some((2024, 3, 15, 10, 30, 45)));
    }

    #[test]
    fn test_get_rename_templates() {
        let templates = get_rename_templates();
        assert!(templates.len() >= 5);
    }
}
