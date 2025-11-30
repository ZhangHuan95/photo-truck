use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Local};

/// 传输历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferHistory {
    pub records: Vec<TransferRecord>,
}

/// 单次传输记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    pub id: String,
    pub timestamp: String,
    pub source_dir: String,
    pub target_dir: String,
    pub template: String,
    pub total_files: usize,
    pub success_count: usize,
    pub skip_count: usize,
    pub error_count: usize,
    pub total_size: u64,
    pub duration_secs: u64,
    pub files: Vec<TransferredFile>,
}

/// 传输的单个文件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferredFile {
    pub source_path: String,
    pub target_path: String,
    pub file_size: u64,
    pub status: TransferFileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferFileStatus {
    Success,
    Skipped,
    Error(String),
}

impl Default for TransferHistory {
    fn default() -> Self {
        Self { records: Vec::new() }
    }
}

impl TransferHistory {
    /// 获取历史记录文件路径
    pub fn get_history_file_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("photo-truck");
        fs::create_dir_all(&config_dir).ok();
        config_dir.join("history.json")
    }

    /// 加载历史记录
    pub fn load() -> Self {
        let path = Self::get_history_file_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// 保存历史记录
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_history_file_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化失败: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("保存失败: {}", e))
    }

    /// 添加新记录
    pub fn add_record(&mut self, record: TransferRecord) {
        self.records.insert(0, record);
        // 只保留最近 100 条记录
        if self.records.len() > 100 {
            self.records.truncate(100);
        }
    }

    /// 创建新的传输记录
    pub fn create_record(
        source_dir: &str,
        target_dir: &str,
        template: &str,
    ) -> TransferRecord {
        let now: DateTime<Local> = Local::now();
        TransferRecord {
            id: now.format("%Y%m%d%H%M%S%3f").to_string(),
            timestamp: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            source_dir: source_dir.to_string(),
            target_dir: target_dir.to_string(),
            template: template.to_string(),
            total_files: 0,
            success_count: 0,
            skip_count: 0,
            error_count: 0,
            total_size: 0,
            duration_secs: 0,
            files: Vec::new(),
        }
    }

    /// 清空历史记录
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// 删除指定记录
    pub fn delete_record(&mut self, id: &str) {
        self.records.retain(|r| r.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_record() {
        let record = TransferHistory::create_record(
            "/source",
            "/target",
            "{year}/{month}",
        );
        assert!(!record.id.is_empty());
        assert_eq!(record.source_dir, "/source");
        assert_eq!(record.target_dir, "/target");
    }

    #[test]
    fn test_add_record() {
        let mut history = TransferHistory::default();
        let record = TransferHistory::create_record("/src", "/dst", "{year}");
        history.add_record(record);
        assert_eq!(history.records.len(), 1);
    }

    #[test]
    fn test_max_records() {
        let mut history = TransferHistory::default();
        for i in 0..110 {
            let mut record = TransferHistory::create_record("/src", "/dst", "{year}");
            record.id = format!("{}", i);
            history.add_record(record);
        }
        assert_eq!(history.records.len(), 100);
    }

    #[test]
    fn test_delete_record() {
        let mut history = TransferHistory::default();
        let mut record = TransferHistory::create_record("/src", "/dst", "{year}");
        record.id = "test123".to_string();
        history.add_record(record);
        assert_eq!(history.records.len(), 1);
        
        history.delete_record("test123");
        assert_eq!(history.records.len(), 0);
    }
}
