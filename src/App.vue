<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

// 类型定义
interface EnvironmentInfo {
  exiftool_installed: boolean;
  exiftool_version: string | null;
  supported_formats: string[];
}

interface TemplateInfo {
  name: string;
  template: string;
}

interface PhotoInfo {
  path: string;
  file_name: string;
  file_size: number;
  date_time: string | null;
  camera: string | null;
  target_folder: string;
  is_duplicate: boolean;
  duplicate_of: string | null;
}

interface ScanResult {
  total_files: number;
  total_size: number;
  photos: PhotoInfo[];
}

interface ClassificationPreview {
  folder: string;
  file_count: number;
  files: string[];
}

interface TransferProgress {
  current: number;
  total: number;
  current_file: string;
  bytes_transferred: number;
  total_bytes: number;
  status: string;
  skipped_duplicates: number;
}

interface TransferResult {
  success_count: number;
  skip_count: number;
  error_count: number;
  errors: string[];
}

// 响应式状态
const envInfo = ref<EnvironmentInfo | null>(null);
const templates = ref<TemplateInfo[]>([]);
const selectedTemplate = ref("");
const customTemplate = ref("{year}/{month}");
const fallbackFolder = ref("未知日期");
const sourceDir = ref("");
const targetDir = ref("");
const scanResult = ref<ScanResult | null>(null);
const classificationPreview = ref<ClassificationPreview[]>([]);
const skipDuplicates = ref(true);
const isScanning = ref(false);
const isTransferring = ref(false);
const transferProgress = ref<TransferProgress | null>(null);
const transferResult = ref<TransferResult | null>(null);
const errorMessage = ref("");
const activeTab = ref("config");

// 计算属性
const currentTemplate = computed(() => {
  if (selectedTemplate.value === "custom") {
    return customTemplate.value;
  }
  return selectedTemplate.value || "{year}/{month}";
});

const totalSizeFormatted = computed(() => {
  if (!scanResult.value) return "0 B";
  return formatSize(scanResult.value.total_size);
});

const progressPercent = computed(() => {
  if (!transferProgress.value) return 0;
  return Math.round(
    (transferProgress.value.current / transferProgress.value.total) * 100
  );
});

// 生命周期
onMounted(async () => {
  await checkEnvironment();
  await loadTemplates();
  setupEventListeners();
});

// 方法
async function checkEnvironment() {
  try {
    envInfo.value = await invoke<EnvironmentInfo>("check_environment");
  } catch (e) {
    errorMessage.value = "环境检查失败: " + e;
  }
}

async function loadTemplates() {
  try {
    templates.value = await invoke<TemplateInfo[]>("get_templates");
    if (templates.value.length > 0) {
      selectedTemplate.value = templates.value[0].template;
    }
  } catch (e) {
    errorMessage.value = "加载模板失败: " + e;
  }
}

function setupEventListeners() {
  listen<TransferProgress>("transfer-progress", (event) => {
    transferProgress.value = event.payload;
  });
}

async function selectSourceDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择源文件夹（照片所在位置）",
  });
  if (selected) {
    sourceDir.value = selected as string;
    scanResult.value = null;
    classificationPreview.value = [];
  }
}

async function selectTargetDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择目标文件夹（NAS 或目标位置）",
  });
  if (selected) {
    targetDir.value = selected as string;
  }
}

async function updateConfig() {
  try {
    await invoke("set_classify_config", {
      template: currentTemplate.value,
      fallbackFolder: fallbackFolder.value,
    });
  } catch (e) {
    errorMessage.value = "配置更新失败: " + e;
  }
}

async function scanPhotos() {
  if (!sourceDir.value) {
    errorMessage.value = "请先选择源文件夹";
    return;
  }

  isScanning.value = true;
  errorMessage.value = "";
  scanResult.value = null;

  try {
    await updateConfig();
    scanResult.value = await invoke<ScanResult>("scan_source_folder", {
      sourceDir: sourceDir.value,
    });

    classificationPreview.value = await invoke<ClassificationPreview[]>(
      "preview_classification"
    );
    activeTab.value = "preview";
  } catch (e) {
    errorMessage.value = "扫描失败: " + e;
  } finally {
    isScanning.value = false;
  }
}

async function startTransfer() {
  if (!targetDir.value) {
    errorMessage.value = "请先选择目标文件夹";
    return;
  }

  if (!scanResult.value || scanResult.value.total_files === 0) {
    errorMessage.value = "没有可传输的照片";
    return;
  }

  isTransferring.value = true;
  errorMessage.value = "";
  transferResult.value = null;
  transferProgress.value = null;
  activeTab.value = "transfer";

  try {
    transferResult.value = await invoke<TransferResult>("start_transfer", {
      targetDir: targetDir.value,
      skipDuplicates: skipDuplicates.value,
    });
  } catch (e) {
    errorMessage.value = "传输失败: " + e;
  } finally {
    isTransferring.value = false;
  }
}

function formatSize(bytes: number): string {
  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;

  if (bytes >= GB) {
    return (bytes / GB).toFixed(2) + " GB";
  } else if (bytes >= MB) {
    return (bytes / MB).toFixed(2) + " MB";
  } else if (bytes >= KB) {
    return (bytes / KB).toFixed(2) + " KB";
  }
  return bytes + " B";
}

function resetAll() {
  scanResult.value = null;
  classificationPreview.value = [];
  transferResult.value = null;
  transferProgress.value = null;
  errorMessage.value = "";
  activeTab.value = "config";
}
</script>

<template>
  <div class="app">
    <header class="header">
      <h1>📷 PhotoTruck</h1>
      <p>照片传输归类工具</p>
    </header>

    <div v-if="envInfo && !envInfo.exiftool_installed" class="warning-banner">
      ⚠️ ExifTool 未安装，部分功能可能受限。请运行: <code>brew install exiftool</code>
    </div>
    <div v-else-if="envInfo" class="success-banner">
      ✅ ExifTool v{{ envInfo.exiftool_version }} 已就绪
    </div>

    <div v-if="errorMessage" class="error-banner">
      ❌ {{ errorMessage }}
      <button @click="errorMessage = ''" class="close-btn">×</button>
    </div>

    <main class="main-content">
      <aside class="sidebar">
        <section class="config-section">
          <h3>📁 文件夹设置</h3>

          <div class="form-group">
            <label>源文件夹</label>
            <div class="input-with-button">
              <input type="text" v-model="sourceDir" placeholder="选择照片所在文件夹" readonly />
              <button @click="selectSourceDir" class="btn btn-secondary">浏览</button>
            </div>
          </div>

          <div class="form-group">
            <label>目标文件夹</label>
            <div class="input-with-button">
              <input type="text" v-model="targetDir" placeholder="选择 NAS 或目标位置" readonly />
              <button @click="selectTargetDir" class="btn btn-secondary">浏览</button>
            </div>
          </div>
        </section>

        <section class="config-section">
          <h3>📂 分类规则</h3>

          <div class="form-group">
            <label>分类模板</label>
            <select v-model="selectedTemplate">
              <option v-for="t in templates" :key="t.template" :value="t.template">
                {{ t.name }} ({{ t.template }})
              </option>
              <option value="custom">自定义...</option>
            </select>
          </div>

          <div v-if="selectedTemplate === 'custom'" class="form-group">
            <label>自定义模板</label>
            <input type="text" v-model="customTemplate" placeholder="{year}/{month}/{day}" />
            <small>支持: {year}, {month}, {day}, {camera}, {make}</small>
          </div>

          <div class="form-group">
            <label>无日期时使用</label>
            <input type="text" v-model="fallbackFolder" />
          </div>
        </section>

        <section class="config-section">
          <h3>⚙️ 选项</h3>
          <label class="checkbox-label">
            <input type="checkbox" v-model="skipDuplicates" />
            跳过重复文件
          </label>
        </section>

        <div class="action-buttons">
          <button @click="scanPhotos" :disabled="!sourceDir || isScanning" class="btn btn-primary btn-large">
            {{ isScanning ? "扫描中..." : "🔍 扫描照片" }}
          </button>

          <button @click="startTransfer" :disabled="!scanResult || !targetDir || isTransferring" class="btn btn-success btn-large">
            {{ isTransferring ? "传输中..." : "🚀 开始传输" }}
          </button>

          <button @click="resetAll" class="btn btn-outline">重置</button>
        </div>
      </aside>

      <div class="content-area">
        <div class="tabs">
          <button :class="{ active: activeTab === 'config' }" @click="activeTab = 'config'">配置</button>
          <button :class="{ active: activeTab === 'preview' }" @click="activeTab = 'preview'" :disabled="!scanResult">
            预览 ({{ scanResult?.total_files || 0 }})
          </button>
          <button :class="{ active: activeTab === 'transfer' }" @click="activeTab = 'transfer'">传输</button>
        </div>

        <div v-show="activeTab === 'config'" class="tab-content">
          <div class="welcome-card">
            <h2>欢迎使用 PhotoTruck</h2>
            <ol>
              <li>选择源文件夹（您的照片所在位置）</li>
              <li>选择目标文件夹（NAS 或目标存储位置）</li>
              <li>配置分类规则</li>
              <li>点击"扫描照片"预览分类结果</li>
              <li>确认无误后点击"开始传输"</li>
            </ol>
            <div v-if="envInfo" class="supported-formats">
              <h4>支持的格式:</h4>
              <div class="format-tags">
                <span v-for="fmt in envInfo.supported_formats.slice(0, 15)" :key="fmt" class="tag">.{{ fmt }}</span>
                <span v-if="envInfo.supported_formats.length > 15" class="tag">+{{ envInfo.supported_formats.length - 15 }} 更多</span>
              </div>
            </div>
          </div>
        </div>

        <div v-show="activeTab === 'preview'" class="tab-content">
          <div v-if="scanResult" class="scan-summary">
            <div class="stat-card">
              <span class="stat-value">{{ scanResult.total_files }}</span>
              <span class="stat-label">张照片</span>
            </div>
            <div class="stat-card">
              <span class="stat-value">{{ totalSizeFormatted }}</span>
              <span class="stat-label">总大小</span>
            </div>
            <div class="stat-card">
              <span class="stat-value">{{ classificationPreview.length }}</span>
              <span class="stat-label">个文件夹</span>
            </div>
          </div>

          <div class="classification-list">
            <div v-for="group in classificationPreview" :key="group.folder" class="classification-group">
              <div class="group-header">
                <span class="folder-icon">📁</span>
                <span class="folder-name">{{ group.folder }}</span>
                <span class="file-count">{{ group.file_count }} 个文件</span>
              </div>
              <div class="group-files">
                <span v-for="file in group.files.slice(0, 5)" :key="file" class="file-tag">{{ file }}</span>
                <span v-if="group.files.length > 5" class="file-tag more">+{{ group.files.length - 5 }} 更多</span>
              </div>
            </div>
          </div>
        </div>

        <div v-show="activeTab === 'transfer'" class="tab-content">
          <div v-if="transferProgress" class="transfer-progress">
            <div class="progress-header">
              <span>{{ transferProgress.status === 'completed' ? '传输完成' : '正在传输...' }}</span>
              <span>{{ transferProgress.current }} / {{ transferProgress.total }}</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
            </div>
            <div class="progress-details">
              <p>当前文件: {{ transferProgress.current_file }}</p>
              <p>已传输: {{ formatSize(transferProgress.bytes_transferred) }} / {{ formatSize(transferProgress.total_bytes) }}</p>
              <p v-if="skipDuplicates">跳过重复: {{ transferProgress.skipped_duplicates }} 个</p>
            </div>
          </div>

          <div v-if="transferResult" class="transfer-result">
            <h3>传输完成</h3>
            <div class="result-stats">
              <div class="result-stat success">
                <span class="num">{{ transferResult.success_count }}</span>
                <span class="label">成功</span>
              </div>
              <div class="result-stat skip">
                <span class="num">{{ transferResult.skip_count }}</span>
                <span class="label">跳过</span>
              </div>
              <div class="result-stat error">
                <span class="num">{{ transferResult.error_count }}</span>
                <span class="label">失败</span>
              </div>
            </div>

            <div v-if="transferResult.errors.length > 0" class="error-list">
              <h4>错误详情:</h4>
              <ul>
                <li v-for="(err, idx) in transferResult.errors" :key="idx">{{ err }}</li>
              </ul>
            </div>
          </div>

          <div v-if="!transferProgress && !transferResult" class="transfer-waiting">
            <p>准备传输...</p>
            <p>请先扫描照片，然后点击"开始传输"</p>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body, #app {
  height: 100%;
  overflow: hidden;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
  background: #f5f7fa;
  color: #333;
}

.app {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.header {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  padding: 12px 20px;
  text-align: center;
  flex-shrink: 0;
}

.header h1 {
  font-size: 18px;
  margin-bottom: 2px;
}

.header p {
  opacity: 0.9;
  font-size: 12px;
}

.warning-banner, .success-banner, .error-banner {
  padding: 6px 15px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  flex-shrink: 0;
}

.warning-banner {
  background: #fff3cd;
  color: #856404;
}

.success-banner {
  background: #d4edda;
  color: #155724;
}

.error-banner {
  background: #f8d7da;
  color: #721c24;
}

.error-banner code {
  background: rgba(0, 0, 0, 0.1);
  padding: 2px 6px;
  border-radius: 3px;
}

.close-btn {
  background: none;
  border: none;
  font-size: 16px;
  cursor: pointer;
  opacity: 0.7;
}

.main-content {
  display: flex;
  flex: 1;
  gap: 15px;
  padding: 15px;
  overflow: hidden;
  min-height: 0;
}

.sidebar {
  width: 280px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
}

.config-section {
  background: white;
  border-radius: 10px;
  padding: 12px 15px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.config-section h3 {
  font-size: 13px;
  margin-bottom: 10px;
  color: #555;
}

.form-group {
  margin-bottom: 10px;
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-group label {
  display: block;
  font-size: 11px;
  color: #666;
  margin-bottom: 4px;
}

.form-group input, .form-group select {
  width: 100%;
  padding: 7px 10px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 12px;
  transition: border-color 0.2s;
}

.form-group input:focus, .form-group select:focus {
  outline: none;
  border-color: #667eea;
}

.form-group small {
  display: block;
  font-size: 10px;
  color: #999;
  margin-top: 3px;
}

.input-with-button {
  display: flex;
  gap: 6px;
}

.input-with-button input {
  flex: 1;
  min-width: 0;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 12px;
}

.checkbox-label input {
  width: 14px;
  height: 14px;
}

.btn {
  padding: 7px 12px;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.btn-secondary {
  background: #e9ecef;
  color: #495057;
}

.btn-success {
  background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
  color: white;
}

.btn-outline {
  background: transparent;
  border: 1px solid #ddd;
  color: #666;
}

.btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-large {
  width: 100%;
  padding: 10px;
  font-size: 13px;
}

.action-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: auto;
  padding-top: 10px;
}

.content-area {
  flex: 1;
  background: white;
  border-radius: 10px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.tabs {
  display: flex;
  border-bottom: 1px solid #eee;
  flex-shrink: 0;
}

.tabs button {
  flex: 1;
  padding: 10px;
  background: none;
  border: none;
  font-size: 12px;
  color: #666;
  cursor: pointer;
  transition: all 0.2s;
  border-bottom: 2px solid transparent;
}

.tabs button.active {
  color: #667eea;
  border-bottom-color: #667eea;
}

.tabs button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.tab-content {
  flex: 1;
  padding: 15px;
  overflow-y: auto;
  min-height: 0;
}

.welcome-card {
  max-width: 500px;
  margin: 0 auto;
}

.welcome-card h2 {
  color: #667eea;
  margin-bottom: 15px;
  font-size: 18px;
}

.welcome-card ol {
  margin-left: 20px;
  line-height: 1.8;
  color: #555;
  font-size: 13px;
}

.supported-formats {
  margin-top: 20px;
  padding-top: 15px;
  border-top: 1px solid #eee;
}

.supported-formats h4 {
  margin-bottom: 8px;
  color: #666;
  font-size: 12px;
}

.format-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag {
  background: #f0f0f0;
  padding: 3px 8px;
  border-radius: 4px;
  font-size: 11px;
  color: #666;
}

.scan-summary {
  display: flex;
  gap: 12px;
  margin-bottom: 15px;
}

.stat-card {
  flex: 1;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  padding: 12px;
  border-radius: 10px;
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 20px;
  font-weight: bold;
}

.stat-label {
  font-size: 11px;
  opacity: 0.9;
}

.classification-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.classification-group {
  border: 1px solid #eee;
  border-radius: 6px;
  overflow: hidden;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f8f9fa;
}

.folder-icon {
  font-size: 16px;
}

.folder-name {
  flex: 1;
  font-weight: 500;
  font-size: 13px;
}

.file-count {
  font-size: 11px;
  color: #888;
}

.group-files {
  padding: 8px 12px;
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.file-tag {
  background: #e9ecef;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 11px;
  color: #555;
}

.file-tag.more {
  background: #667eea;
  color: white;
}

.transfer-progress {
  max-width: 500px;
  margin: 0 auto;
}

.progress-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
  font-size: 12px;
  color: #666;
}

.progress-bar {
  height: 16px;
  background: #e9ecef;
  border-radius: 8px;
  overflow: hidden;
  margin-bottom: 15px;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  transition: width 0.3s;
}

.progress-details {
  background: #f8f9fa;
  padding: 12px;
  border-radius: 6px;
}

.progress-details p {
  margin-bottom: 5px;
  font-size: 12px;
  color: #555;
}

.progress-details p:last-child {
  margin-bottom: 0;
}

.transfer-result {
  max-width: 500px;
  margin: 0 auto;
  text-align: center;
}

.transfer-result h3 {
  color: #28a745;
  margin-bottom: 15px;
  font-size: 16px;
}

.result-stats {
  display: flex;
  justify-content: center;
  gap: 25px;
  margin-bottom: 20px;
}

.result-stat {
  text-align: center;
}

.result-stat .num {
  display: block;
  font-size: 28px;
  font-weight: bold;
}

.result-stat.success .num {
  color: #28a745;
}

.result-stat.skip .num {
  color: #ffc107;
}

.result-stat.error .num {
  color: #dc3545;
}

.result-stat .label {
  font-size: 12px;
  color: #666;
}

.error-list {
  text-align: left;
  background: #fff3cd;
  padding: 12px;
  border-radius: 6px;
}

.error-list h4 {
  margin-bottom: 8px;
  color: #856404;
  font-size: 13px;
}

.error-list ul {
  margin-left: 20px;
  font-size: 12px;
  color: #856404;
}

.transfer-waiting {
  text-align: center;
  padding: 30px;
  color: #888;
}

.transfer-waiting p:first-child {
  font-size: 16px;
  margin-bottom: 8px;
}

.transfer-waiting p:last-child {
  font-size: 13px;
}

/* 响应式调整 - 小窗口 */
@media (max-height: 700px) {
  .header {
    padding: 8px 15px;
  }
  .header h1 {
    font-size: 16px;
  }
  .header p {
    display: none;
  }
  .main-content {
    padding: 10px;
    gap: 10px;
  }
  .sidebar {
    width: 250px;
    gap: 8px;
  }
  .config-section {
    padding: 10px 12px;
  }
  .config-section h3 {
    font-size: 12px;
    margin-bottom: 8px;
  }
  .form-group {
    margin-bottom: 8px;
  }
  .form-group input, .form-group select {
    padding: 6px 8px;
  }
  .btn-large {
    padding: 8px;
  }
}

/* 响应式调整 - 更宽的窗口 */
@media (min-width: 1200px) {
  .sidebar {
    width: 320px;
  }
}
</style>
