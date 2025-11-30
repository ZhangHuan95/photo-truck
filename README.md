# Photo Truck 📷🚚

<p align="center">
  <img src="public/logo.svg" alt="Photo Truck Logo" width="128">
</p>

<p align="center">
  <strong>一个用于将照片传输和归类到 NAS 的桌面应用程序</strong>
</p>

<p align="center">
  <a href="https://github.com/ZhangHuan95/photo-truck/releases"><img src="https://img.shields.io/badge/version-1.1.0-blue.svg" alt="Version"></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg" alt="Platform">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  <img src="https://img.shields.io/badge/rust-1.70+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/tauri-2.0-blueviolet.svg" alt="Tauri">
</p>

---

## 📖 简介

Photo Truck 是一款专为摄影师和照片管理爱好者设计的桌面工具。它能够自动读取照片的 EXIF 信息，根据拍摄日期智能分类，并将照片传输到 NAS 或其他存储设备。支持佳能、尼康、索尼等主流相机的 RAW 格式。

## ✨ 功能特性

| 功能 | 描述 |
|------|------|
| 📂 **照片扫描** | 递归扫描源文件夹中的所有照片，支持子目录 |
| 🎨 **多格式支持** | 支持 30+ 种照片格式，包括各品牌 RAW |
| 📅 **智能分类** | 根据 EXIF 日期信息自动创建分类文件夹 |
| 🔄 **去重功能** | 使用 SHA-256 哈希检测重复文件 |
| 📊 **进度监控** | 实时显示传输进度和统计信息 |
| 🎯 **灵活模板** | 支持多种分类模板，可自定义 |
| 🖼️ **缩略图预览** | 传输前预览照片缩略图 |
| ⏹️ **传输取消** | 随时中断传输操作 |
| 📜 **历史记录** | 查看历史传输记录 |
| 💻 **命令行模式** | 支持 CLI 无界面批量传输 |
| 🌐 **跨平台** | 支持 macOS、Windows、Linux |

### 支持的照片格式

<details>
<summary>点击展开完整列表</summary>

**RAW 格式:**
| 品牌 | 格式 |
|------|------|
| Canon | CR3, CR2, CRW |
| Nikon | NEF, NRW |
| Sony | ARW, SRF, SR2 |
| Olympus | ORF |
| Fujifilm | RAF |
| Panasonic | RW2 |
| Pentax | PEF |
| Leica | RAW, RWL |
| Hasselblad | 3FR |
| Sigma | X3F |
| Adobe | DNG |

**通用格式:**
- JPEG: JPG, JPEG
- PNG: PNG
- TIFF: TIFF, TIF
- Apple: HEIC, HEIF
- Web: WebP
- 其他: BMP, GIF

</details>

## 📸 界面预览

```
┌─────────────────────────────────────────────────────────────┐
│  Photo Truck - 照片传输归类工具                    v1.0.0   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  📁 源文件夹:    /Volumes/SD-Card/DCIM         [选择]      │
│  📁 目标文件夹:  /Volumes/NAS/Photos           [选择]      │
│                                                             │
│  📋 分类模板:    [按年/月 ▼]                               │
│  ☑️ 跳过重复文件                                           │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 扫描结果: 找到 128 张照片，共 3.2 GB                   │ │
│  │                                                       │ │
│  │ 📷 IMG_0001.CR3  →  2024/03/15  (Canon EOS R5)       │ │
│  │ 📷 IMG_0002.CR3  →  2024/03/15  (Canon EOS R5)       │ │
│  │ 📷 DSC_0001.NEF  →  2024/03/16  (Nikon Z6)           │ │
│  │ ...                                                   │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  进度: ████████████████████░░░░░░░░░░ 65%  (83/128)        │
│  已传输: 2.1 GB / 3.2 GB | 跳过: 5 个重复文件              │
│                                                             │
│           [扫描照片]              [开始传输]                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 快速开始

### 系统要求

- **操作系统**: macOS 10.15+、Windows 10+、Linux (Ubuntu 20.04+)
- **依赖**: ExifTool (用于读取照片 EXIF 元数据)

### 安装

#### macOS

1. 下载最新的 DMG 安装包
2. 打开 DMG 文件，将 Photo Truck 拖入 Applications 文件夹
3. 安装 ExifTool:
   ```bash
   brew install exiftool
   ```

#### Windows

1. 下载最新的 MSI 安装包
2. 运行安装程序
3. 下载并安装 [ExifTool for Windows](https://exiftool.org/)

#### Linux

1. 下载最新的 AppImage 或 .deb 包
2. 安装 ExifTool:
   ```bash
   # Ubuntu/Debian
   sudo apt install libimage-exiftool-perl
   
   # Fedora
   sudo dnf install perl-Image-ExifTool
   ```

## 📖 使用指南

### 基本使用流程

```mermaid
graph LR
    A[选择源文件夹] --> B[选择目标文件夹]
    B --> C[选择分类模板]
    C --> D[扫描照片]
    D --> E[确认传输列表]
    E --> F[开始传输]
    F --> G[完成]
```

### 详细步骤

1. **选择源文件夹**
   - 点击源文件夹旁的"选择"按钮
   - 选择包含照片的文件夹（如 SD 卡、相机存储）
   - 支持选择包含子文件夹的目录

2. **选择目标文件夹**
   - 点击目标文件夹旁的"选择"按钮
   - 选择 NAS 挂载路径或本地存储位置

3. **选择分类模板**
   - 从下拉菜单选择预设模板
   - 可选模板见下方表格

4. **配置选项**
   - 勾选"跳过重复文件"可避免重复传输相同照片

5. **扫描照片**
   - 点击"扫描照片"按钮
   - 查看扫描结果和预览分类效果

6. **开始传输**
   - 确认无误后点击"开始传输"
   - 实时查看传输进度

### 分类模板详解

| 模板名称 | 格式 | 示例输入 | 示例输出 |
|----------|------|----------|----------|
| 按年/月 | `{year}/{month}` | 拍摄于 2024-03-15 | `2024/03/IMG_0001.CR3` |
| 按年/月/日 | `{year}/{month}/{day}` | 拍摄于 2024-03-15 | `2024/03/15/IMG_0001.CR3` |
| 按年/月-日 | `{year}/{month}-{day}` | 拍摄于 2024-03-15 | `2024/03-15/IMG_0001.CR3` |
| 按品牌/年/月 | `{make}/{year}/{month}` | Canon 相机 | `Canon/2024/03/IMG_0001.CR3` |
| 按相机/年/月 | `{camera}/{year}/{month}` | Canon EOS R5 | `Canon EOS R5/2024/03/IMG_0001.CR3` |
| 按年/相机/月 | `{year}/{camera}/{month}` | Canon EOS R5 | `2024/Canon EOS R5/03/IMG_0001.CR3` |

### 模板变量说明

| 变量 | 描述 | 示例 |
|------|------|------|
| `{year}` | 4位年份 | 2024 |
| `{month}` | 2位月份 | 03 |
| `{day}` | 2位日期 | 15 |
| `{camera}` | 相机型号 | Canon EOS R5 |
| `{make}` | 相机品牌 | Canon |

## 🔧 高级配置

### 命令行模式

Photo Truck 支持命令行模式，可在无图形界面的环境下使用：

```bash
# 基本用法
photo-truck -s /Volumes/SD/DCIM -t /Volumes/NAS/Photos

# 使用自定义模板
photo-truck -s ~/Pictures -t ~/Backup -p "{year}/{month}-{day}"

# 预览模式（不传输）
photo-truck -s ~/Pictures -t ~/Backup --dry-run

# 查看帮助
photo-truck --help
```

| 选项 | 说明 |
|------|------|
| `-s, --source <路径>` | 源文件夹路径 |
| `-t, --target <路径>` | 目标文件夹路径 |
| `-p, --template <模板>` | 分类模板 |
| `--no-skip-duplicates` | 不跳过重复文件 |
| `-n, --dry-run` | 预览模式 |
| `-h, --help` | 显示帮助 |
| `-v, --version` | 显示版本 |

### 配置文件位置

- **macOS**: `~/Library/Application Support/photo-truck/`
- **Windows**: `%APPDATA%/photo-truck/`
- **Linux**: `~/.config/photo-truck/`

## ❓ 常见问题

<details>
<summary><strong>Q: 为什么需要安装 ExifTool？</strong></summary>

A: ExifTool 是一个功能强大的 EXIF 元数据读取工具，支持几乎所有照片格式。Photo Truck 使用它来读取照片的拍摄日期、相机型号等信息，这些信息用于智能分类。
</details>

<details>
<summary><strong>Q: 支持哪些 RAW 格式？</strong></summary>

A: 支持所有主流相机品牌的 RAW 格式，包括：
- Canon: CR3, CR2, CRW
- Nikon: NEF, NRW
- Sony: ARW, SRF, SR2
- 更多格式请查看上方完整列表
</details>

<details>
<summary><strong>Q: 如何处理没有 EXIF 日期的照片？</strong></summary>

A: 对于无法读取日期信息的照片，会被放入"未知日期"文件夹。您可以在设置中自定义此文件夹的名称。
</details>

<details>
<summary><strong>Q: 去重功能是如何工作的？</strong></summary>

A: Photo Truck 使用两级哈希策略：
1. **快速哈希**: 读取文件头尾各 64KB + 文件大小，用于快速预筛选
2. **完整哈希**: 对可能重复的文件计算完整 SHA-256 哈希，确保准确性

这种方法在保证准确性的同时，大大提高了大文件的处理速度。
</details>

<details>
<summary><strong>Q: 传输过程中可以取消吗？</strong></summary>

A: 传输过程中可以点击"取消传输"按钮中断操作，已传输的文件会保留。
</details>

<details>
<summary><strong>Q: 照片是移动还是复制？</strong></summary>

A: Photo Truck 执行的是**复制**操作，原始文件不会被删除。这确保了数据安全。
</details>

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！详细的开发指南请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 📄 许可证

本项目基于 MIT 许可证开源 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [ExifTool](https://exiftool.org/) - 强大的 EXIF 元数据工具
- [sha2](https://crates.io/crates/sha2) - Rust SHA-2 哈希库

---

<p align="center">
  Made with ❤️ for photographers
</p>
