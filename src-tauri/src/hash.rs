use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// 计算文件的 SHA-256 哈希值
pub fn calculate_hash(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    
    let mut reader = BufReader::with_capacity(1024 * 1024, file); // 1MB buffer
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| format!("读取文件失败: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// 快速哈希：只读取文件头部和尾部（用于快速预筛选）
pub fn calculate_quick_hash(file_path: &str, sample_size: usize) -> Result<String, String> {
    let path = Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let metadata = file.metadata().map_err(|e| format!("无法读取文件元数据: {}", e))?;
    let file_size = metadata.len() as usize;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    // 读取文件头部
    let head_size = sample_size.min(file_size);
    let mut head_buffer = vec![0u8; head_size];
    reader.read_exact(&mut head_buffer).map_err(|e| format!("读取文件头部失败: {}", e))?;
    hasher.update(&head_buffer);

    // 如果文件足够大，也读取尾部
    if file_size > sample_size * 2 {
        use std::io::Seek;
        let tail_start = file_size - sample_size;
        reader.seek(std::io::SeekFrom::Start(tail_start as u64))
            .map_err(|e| format!("定位文件尾部失败: {}", e))?;
        let mut tail_buffer = vec![0u8; sample_size];
        reader.read_exact(&mut tail_buffer).map_err(|e| format!("读取文件尾部失败: {}", e))?;
        hasher.update(&tail_buffer);
    }

    // 加入文件大小作为哈希的一部分
    hasher.update(file_size.to_le_bytes());

    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// 文件去重器
pub struct Deduplicator {
    /// 已知文件的哈希 -> 文件路径
    hash_map: HashMap<String, String>,
    /// 使用快速哈希进行预筛选
    quick_hash_map: HashMap<String, Vec<String>>,
}

impl Deduplicator {
    pub fn new() -> Self {
        Self {
            hash_map: HashMap::new(),
            quick_hash_map: HashMap::new(),
        }
    }

    /// 检查文件是否重复
    /// 返回 Some(原文件路径) 如果是重复的，None 如果是新文件
    pub fn check_duplicate(&mut self, file_path: &str, _file_size: u64) -> Result<Option<String>, String> {
        // 第一步：快速哈希预筛选
        let quick_hash = calculate_quick_hash(file_path, 64 * 1024)?; // 64KB 样本

        if let Some(_candidates) = self.quick_hash_map.get(&quick_hash) {
            // 有潜在重复，进行完整哈希比对
            let full_hash = calculate_hash(file_path)?;
            
            if let Some(original_path) = self.hash_map.get(&full_hash) {
                return Ok(Some(original_path.clone()));
            }
            
            // 不是重复文件，记录它
            self.hash_map.insert(full_hash, file_path.to_string());
        } else {
            // 快速哈希没有匹配，这是新文件
            self.quick_hash_map
                .entry(quick_hash)
                .or_insert_with(Vec::new)
                .push(file_path.to_string());
            
            // 计算并存储完整哈希
            let full_hash = calculate_hash(file_path)?;
            self.hash_map.insert(full_hash, file_path.to_string());
        }

        Ok(None)
    }

    /// 添加已知文件（用于加载目标目录中已有的文件）
    pub fn add_known_file(&mut self, file_path: &str) -> Result<(), String> {
        let quick_hash = calculate_quick_hash(file_path, 64 * 1024)?;
        let full_hash = calculate_hash(file_path)?;
        
        self.quick_hash_map
            .entry(quick_hash)
            .or_insert_with(Vec::new)
            .push(file_path.to_string());
        self.hash_map.insert(full_hash, file_path.to_string());
        
        Ok(())
    }

    /// 获取已记录的文件数量
    pub fn len(&self) -> usize {
        self.hash_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hash_map.is_empty()
    }
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    // 辅助函数：创建测试文件
    fn create_test_file(dir: &TempDir, name: &str, content: &[u8]) -> String {
        let path = dir.path().join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path.to_string_lossy().to_string()
    }

    // ==================== 哈希计算测试 ====================

    #[test]
    fn test_calculate_hash_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "empty.txt", b"");
        
        let hash = calculate_hash(&path).unwrap();
        // SHA-256 of empty string
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_calculate_hash_simple_content() {
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "hello.txt", b"hello world");
        
        let hash = calculate_hash(&path).unwrap();
        // SHA-256 of "hello world"
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_calculate_hash_same_content_same_hash() {
        let dir = TempDir::new().unwrap();
        let path1 = create_test_file(&dir, "file1.txt", b"identical content");
        let path2 = create_test_file(&dir, "file2.txt", b"identical content");
        
        let hash1 = calculate_hash(&path1).unwrap();
        let hash2 = calculate_hash(&path2).unwrap();
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_calculate_hash_different_content_different_hash() {
        let dir = TempDir::new().unwrap();
        let path1 = create_test_file(&dir, "file1.txt", b"content A");
        let path2 = create_test_file(&dir, "file2.txt", b"content B");
        
        let hash1 = calculate_hash(&path1).unwrap();
        let hash2 = calculate_hash(&path2).unwrap();
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_calculate_hash_nonexistent_file() {
        let result = calculate_hash("/nonexistent/path/file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无法打开文件"));
    }

    #[test]
    fn test_calculate_hash_large_file() {
        let dir = TempDir::new().unwrap();
        // 创建 2MB 的文件
        let content: Vec<u8> = (0..2_000_000).map(|i| (i % 256) as u8).collect();
        let path = create_test_file(&dir, "large.bin", &content);
        
        let hash = calculate_hash(&path);
        assert!(hash.is_ok());
        assert_eq!(hash.unwrap().len(), 64); // SHA-256 hex = 64 chars
    }

    // ==================== 快速哈希测试 ====================

    #[test]
    fn test_calculate_quick_hash_small_file() {
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "small.txt", b"small content");
        
        let hash = calculate_quick_hash(&path, 1024);
        assert!(hash.is_ok());
    }

    #[test]
    fn test_calculate_quick_hash_large_file() {
        let dir = TempDir::new().unwrap();
        // 创建 200KB 文件
        let content: Vec<u8> = (0..200_000).map(|i| (i % 256) as u8).collect();
        let path = create_test_file(&dir, "large.bin", &content);
        
        let hash = calculate_quick_hash(&path, 64 * 1024); // 64KB sample
        assert!(hash.is_ok());
    }

    #[test]
    fn test_quick_hash_includes_file_size() {
        let dir = TempDir::new().unwrap();
        // 两个文件内容不同但头部相同，大小也不同
        let mut content1 = vec![0u8; 1000];
        let mut content2 = vec![0u8; 2000]; // 不同大小
        content1[999] = 1;
        content2[999] = 1; // 头部相同
        content2[1999] = 2; // 尾部不同
        
        let path1 = create_test_file(&dir, "file1.bin", &content1);
        let path2 = create_test_file(&dir, "file2.bin", &content2);
        
        // 快速哈希应该因为文件大小不同而不同
        let hash1 = calculate_quick_hash(&path1, 500).unwrap();
        let hash2 = calculate_quick_hash(&path2, 500).unwrap();
        
        // 由于文件大小被包含在哈希中，哈希应该不同
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_quick_hash_different_sizes() {
        let dir = TempDir::new().unwrap();
        let path1 = create_test_file(&dir, "file1.txt", b"short");
        let path2 = create_test_file(&dir, "file2.txt", b"short longer content");
        
        let hash1 = calculate_quick_hash(&path1, 1024).unwrap();
        let hash2 = calculate_quick_hash(&path2, 1024).unwrap();
        
        // 文件大小不同，快速哈希应该不同
        assert_ne!(hash1, hash2);
    }

    // ==================== 去重器测试 ====================

    #[test]
    fn test_deduplicator_new() {
        let dedup = Deduplicator::new();
        assert!(dedup.is_empty());
        assert_eq!(dedup.len(), 0);
    }

    #[test]
    fn test_deduplicator_default() {
        let dedup = Deduplicator::default();
        assert!(dedup.is_empty());
    }

    #[test]
    fn test_deduplicator_check_new_file() {
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "new.txt", b"new content");
        
        let mut dedup = Deduplicator::new();
        let result = dedup.check_duplicate(&path, 11);
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // 不是重复的
    }

    #[test]
    fn test_deduplicator_detect_duplicate() {
        let dir = TempDir::new().unwrap();
        let content = b"duplicate content here";
        let path1 = create_test_file(&dir, "original.txt", content);
        let path2 = create_test_file(&dir, "copy.txt", content);
        
        let mut dedup = Deduplicator::new();
        
        // 第一个文件应该是新的
        let result1 = dedup.check_duplicate(&path1, content.len() as u64);
        assert!(result1.is_ok());
        assert!(result1.unwrap().is_none());
        
        // 第二个文件应该被检测为重复
        let result2 = dedup.check_duplicate(&path2, content.len() as u64);
        assert!(result2.is_ok());
        let duplicate_of = result2.unwrap();
        assert!(duplicate_of.is_some());
        assert_eq!(duplicate_of.unwrap(), path1);
    }

    #[test]
    fn test_deduplicator_different_files() {
        let dir = TempDir::new().unwrap();
        let path1 = create_test_file(&dir, "file1.txt", b"content one");
        let path2 = create_test_file(&dir, "file2.txt", b"content two");
        
        let mut dedup = Deduplicator::new();
        
        let result1 = dedup.check_duplicate(&path1, 11);
        assert!(result1.unwrap().is_none());
        
        let result2 = dedup.check_duplicate(&path2, 11);
        assert!(result2.unwrap().is_none());
        
        assert_eq!(dedup.len(), 2);
    }

    #[test]
    fn test_deduplicator_add_known_file() {
        let dir = TempDir::new().unwrap();
        let content = b"known content";
        let path1 = create_test_file(&dir, "existing.txt", content);
        let path2 = create_test_file(&dir, "new.txt", content);
        
        let mut dedup = Deduplicator::new();
        
        // 添加已知文件
        dedup.add_known_file(&path1).unwrap();
        assert_eq!(dedup.len(), 1);
        
        // 检查相同内容的新文件应该被检测为重复
        let result = dedup.check_duplicate(&path2, content.len() as u64);
        assert!(result.is_ok());
        let duplicate_of = result.unwrap();
        assert!(duplicate_of.is_some());
        assert_eq!(duplicate_of.unwrap(), path1);
    }

    #[test]
    fn test_deduplicator_multiple_duplicates() {
        let dir = TempDir::new().unwrap();
        let content = b"same content";
        let path1 = create_test_file(&dir, "file1.txt", content);
        let path2 = create_test_file(&dir, "file2.txt", content);
        let path3 = create_test_file(&dir, "file3.txt", content);
        
        let mut dedup = Deduplicator::new();
        
        // 第一个是原始文件
        assert!(dedup.check_duplicate(&path1, content.len() as u64).unwrap().is_none());
        
        // 后续都是重复的，指向第一个
        let dup2 = dedup.check_duplicate(&path2, content.len() as u64).unwrap();
        assert_eq!(dup2.unwrap(), path1);
        
        let dup3 = dedup.check_duplicate(&path3, content.len() as u64).unwrap();
        assert_eq!(dup3.unwrap(), path1);
    }

    #[test]
    fn test_deduplicator_with_large_files() {
        let dir = TempDir::new().unwrap();
        // 创建 500KB 的相同内容文件
        let content: Vec<u8> = (0..500_000).map(|i| (i % 256) as u8).collect();
        let path1 = create_test_file(&dir, "large1.bin", &content);
        let path2 = create_test_file(&dir, "large2.bin", &content);
        
        let mut dedup = Deduplicator::new();
        
        let result1 = dedup.check_duplicate(&path1, content.len() as u64);
        assert!(result1.unwrap().is_none());
        
        let result2 = dedup.check_duplicate(&path2, content.len() as u64);
        let duplicate = result2.unwrap();
        assert!(duplicate.is_some());
        assert_eq!(duplicate.unwrap(), path1);
    }
}
