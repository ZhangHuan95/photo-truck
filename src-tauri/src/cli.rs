// 命令行模式支持
use std::env;
use std::path::Path;

use crate::classify::ClassifyConfig;
use crate::exif::check_exiftool;
use crate::transfer::{scan_photos, format_size};

/// 命令行参数
pub struct CliArgs {
    pub source_dir: String,
    pub target_dir: String,
    pub template: String,
    pub skip_duplicates: bool,
    pub dry_run: bool,
    pub help: bool,
    pub version: bool,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            source_dir: String::new(),
            target_dir: String::new(),
            template: "{year}/{month}".to_string(),
            skip_duplicates: true,
            dry_run: false,
            help: false,
            version: false,
        }
    }
}

/// 解析命令行参数
pub fn parse_args() -> Option<CliArgs> {
    let args: Vec<String> = env::args().collect();
    
    // 如果没有参数，返回 None 表示使用 GUI 模式
    if args.len() <= 1 {
        return None;
    }

    let mut cli_args = CliArgs::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                cli_args.help = true;
                return Some(cli_args);
            }
            "-v" | "--version" => {
                cli_args.version = true;
                return Some(cli_args);
            }
            "-s" | "--source" => {
                if i + 1 < args.len() {
                    cli_args.source_dir = args[i + 1].clone();
                    i += 1;
                }
            }
            "-t" | "--target" => {
                if i + 1 < args.len() {
                    cli_args.target_dir = args[i + 1].clone();
                    i += 1;
                }
            }
            "-p" | "--template" => {
                if i + 1 < args.len() {
                    cli_args.template = args[i + 1].clone();
                    i += 1;
                }
            }
            "--no-skip-duplicates" => {
                cli_args.skip_duplicates = false;
            }
            "-n" | "--dry-run" => {
                cli_args.dry_run = true;
            }
            _ => {
                // 忽略未知参数
            }
        }
        i += 1;
    }

    Some(cli_args)
}

/// 显示帮助信息
pub fn print_help() {
    println!(r#"
Photo Truck - 照片传输归类工具

用法:
    photo-truck [选项]

选项:
    -s, --source <路径>       源文件夹路径（照片所在位置）
    -t, --target <路径>       目标文件夹路径（NAS或存储位置）
    -p, --template <模板>     分类模板（默认: {{year}}/{{month}}）
    --no-skip-duplicates      不跳过重复文件
    -n, --dry-run             预览模式，不实际传输文件
    -h, --help                显示帮助信息
    -v, --version             显示版本信息

模板变量:
    {{year}}   - 4位年份 (如: 2024)
    {{month}}  - 2位月份 (如: 03)
    {{day}}    - 2位日期 (如: 15)
    {{camera}} - 相机型号 (如: Canon EOS R5)
    {{make}}   - 相机品牌 (如: Canon)

示例:
    # 基本用法
    photo-truck -s /Volumes/SD/DCIM -t /Volumes/NAS/Photos

    # 使用自定义模板
    photo-truck -s ~/Pictures -t ~/Backup -p "{{year}}/{{month}}-{{day}}"

    # 预览模式（不传输）
    photo-truck -s ~/Pictures -t ~/Backup --dry-run

    # 不跳过重复文件
    photo-truck -s ~/Pictures -t ~/Backup --no-skip-duplicates
"#);
}

/// 显示版本信息
pub fn print_version() {
    println!("Photo Truck v{}", env!("CARGO_PKG_VERSION"));
    println!("照片传输归类工具 - 支持RAW格式、智能分类、去重功能");
}

/// 运行命令行模式
pub fn run_cli(args: CliArgs) -> i32 {
    if args.help {
        print_help();
        return 0;
    }

    if args.version {
        print_version();
        return 0;
    }

    // 检查必要参数
    if args.source_dir.is_empty() {
        eprintln!("错误: 请指定源文件夹 (-s 或 --source)");
        eprintln!("使用 --help 查看帮助");
        return 1;
    }

    if args.target_dir.is_empty() && !args.dry_run {
        eprintln!("错误: 请指定目标文件夹 (-t 或 --target)");
        eprintln!("使用 --help 查看帮助");
        return 1;
    }

    // 检查路径是否存在
    if !Path::new(&args.source_dir).exists() {
        eprintln!("错误: 源文件夹不存在: {}", args.source_dir);
        return 1;
    }

    // 检查 ExifTool
    println!("检查环境...");
    match check_exiftool() {
        Ok(version) => println!("✓ ExifTool {} 已就绪", version),
        Err(_) => {
            eprintln!("⚠ ExifTool 未安装，可能无法读取照片日期");
            eprintln!("  安装: brew install exiftool");
        }
    }

    // 创建配置
    let config = ClassifyConfig {
        template: args.template.clone(),
        fallback_folder: "未知日期".to_string(),
    };

    // 扫描照片
    println!("\n扫描照片中...");
    println!("源文件夹: {}", args.source_dir);

    let scan_result = match scan_photos(&args.source_dir, &config) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("扫描失败: {}", e);
            return 1;
        }
    };

    println!("\n扫描完成:");
    println!("  找到 {} 张照片", scan_result.total_files);
    println!("  总大小: {}", format_size(scan_result.total_size));

    if scan_result.total_files == 0 {
        println!("\n没有找到照片，退出");
        return 0;
    }

    // 预览分类
    println!("\n分类预览 (模板: {}):", args.template);
    let mut folder_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for photo in &scan_result.photos {
        *folder_counts.entry(photo.target_folder.clone()).or_insert(0) += 1;
    }
    let mut folders: Vec<_> = folder_counts.into_iter().collect();
    folders.sort_by(|a, b| a.0.cmp(&b.0));
    for (folder, count) in folders.iter().take(10) {
        println!("  📁 {} ({} 个文件)", folder, count);
    }
    if folders.len() > 10 {
        println!("  ... 还有 {} 个文件夹", folders.len() - 10);
    }

    // 预览模式
    if args.dry_run {
        println!("\n[预览模式] 不执行实际传输");
        return 0;
    }

    // 确认传输
    println!("\n目标文件夹: {}", args.target_dir);
    if args.skip_duplicates {
        println!("重复文件: 跳过");
    } else {
        println!("重复文件: 覆盖");
    }

    // 创建目标目录
    if !Path::new(&args.target_dir).exists() {
        println!("创建目标目录...");
        if let Err(e) = std::fs::create_dir_all(&args.target_dir) {
            eprintln!("创建目录失败: {}", e);
            return 1;
        }
    }

    // 执行传输
    println!("\n开始传输...");
    
    use crate::hash::Deduplicator;
    use walkdir::WalkDir;

    let mut deduplicator = Deduplicator::new();
    let mut success_count = 0;
    let mut skip_count = 0;
    let mut error_count = 0;

    // 扫描目标目录已有文件（用于去重）
    if args.skip_duplicates && Path::new(&args.target_dir).exists() {
        print!("扫描目标目录...");
        for entry in WalkDir::new(&args.target_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().is_file() {
                let _ = deduplicator.add_known_file(&entry.path().to_string_lossy());
            }
        }
        println!(" 完成");
    }

    let total = scan_result.photos.len();
    for (index, photo) in scan_result.photos.iter().enumerate() {
        // 进度显示
        if (index + 1) % 10 == 0 || index + 1 == total {
            print!("\r传输进度: {}/{} ({:.0}%)  ", 
                index + 1, total, 
                ((index + 1) as f64 / total as f64) * 100.0);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }

        // 检查重复
        if args.skip_duplicates {
            if let Ok(Some(_)) = deduplicator.check_duplicate(&photo.path, photo.file_size) {
                skip_count += 1;
                continue;
            }
        }

        // 构建目标路径
        let target_dir = Path::new(&args.target_dir).join(&photo.target_folder);
        let target_path = target_dir.join(&photo.file_name);

        // 创建目录
        if let Err(_) = std::fs::create_dir_all(&target_dir) {
            error_count += 1;
            continue;
        }

        // 处理文件名冲突
        let final_path = if target_path.exists() {
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
                let new_name = if ext.is_empty() {
                    format!("{}_{}", stem, counter)
                } else {
                    format!("{}_{}.{}", stem, counter, ext)
                };
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
        match std::fs::copy(&photo.path, &final_path) {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    println!("\n\n传输完成!");
    println!("  ✓ 成功: {} 个", success_count);
    println!("  ⊘ 跳过: {} 个", skip_count);
    println!("  ✗ 失败: {} 个", error_count);

    if error_count > 0 { 1 } else { 0 }
}
