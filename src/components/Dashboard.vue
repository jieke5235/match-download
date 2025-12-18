<script setup>
import { ref, onMounted, computed } from 'vue';
import { invoke } from "@tauri-apps/api/core";
import { open, ask, message } from '@tauri-apps/plugin-dialog';
import { check } from '@tauri-apps/plugin-updater';

const props = defineProps({
  accessToken: String,
  domain: String,
  userInfo: Object,
  batches: Array  // 接收批次数据用于判断下载状态
});

const emit = defineEmits(['start-download', 'logout']);

const matches = ref([]);
const stages = ref([]);
const works = ref([]);
const selectedMatch = ref(null);
const selectedStage = ref(null);
const savePath = ref('');
const isLoading = ref(false);

// 系统信息和控制
const systemInfo = ref({
  cpu_cores: 0,
  recommended_concurrency: 0,
  max_concurrency: 0
});
const concurrency = ref(10);
const downloadState = ref('Idle');
// 移除全局 isDownloading，改为基于批次的状态管理

// 计算属性：判断是否有正在下载的任务
const hasActiveDownloads = computed(() => {
  if (!props.batches || props.batches.length === 0) return false;
  return props.batches.some(batch => 
    batch.status === 'downloading' || batch.status === 'paused'
  );
});

// 更新状态
const isCheckingUpdate = ref(false);
const updateAvailable = ref(null);

onMounted(async () => {
  await loadMatches();
  await loadSystemInfo();
});

const loadSystemInfo = async () => {
  try {
    systemInfo.value = await invoke('get_system_info');
    concurrency.value = await invoke('get_current_concurrency');
  } catch (error) {
    console.error('Failed to load system info:', error);
  }
};

const loadMatches = async () => {
  try {
    isLoading.value = true;
    matches.value = await invoke("fetch_matches", {
      accessToken: props.accessToken,
      domain: props.domain
    });
    
    if (matches.value.length > 0) {
      await selectMatch(matches.value[0]);
    }
  } catch (error) {
    console.error('Failed to load matches:', error);
    alert('加载比赛列表失败: ' + error);
  } finally {
    isLoading.value = false;
  }
};

const selectMatch = async (match) => {
  try {
    selectedMatch.value = match;
    selectedStage.value = null;
    works.value = [];
    
    // 加载赛段
    stages.value = await invoke("fetch_stages", {
      accessToken: props.accessToken,
      domain: props.domain,
      matchId: match.id
    });
    
    if (stages.value.length > 0) {
      await selectStage(stages.value[0]);
    }
  } catch (error) {
    console.error('Failed to load stages:', error);
    alert('加载赛段失败: ' + error);
  }
};

const selectStage = async (stage) => {
  try {
    selectedStage.value = stage;
    
    // 加载作品
    works.value = await invoke("fetch_works", {
      accessToken: props.accessToken,
      domain: props.domain,
      matchId: selectedMatch.value.id,
      stageId: stage.id
    });
  } catch (error) {
    console.error('Failed to load works:', error);
    alert('加载作品失败: ' + error);
  }
};

const selectFolder = async () => {
    const selected = await open({
        directory: true,
        multiple: false,
    });
    if (selected) {
        savePath.value = selected;
    }
};

const startDownload = () => {
  if (!selectedMatch.value || !selectedStage.value) return;
  if (!savePath.value) {
      alert("请先选择保存位置");
      return;
  }
  
  if (works.value.length === 0) {
      alert("没有可下载的作品");
      return;
  }
  
  // 过滤出有文件的作品
  const worksToDownload = works.value.filter(work => work.files && work.files.length > 0);
  
  if (worksToDownload.length === 0) {
      alert("没有找到可下载的文件");
      return;
  }
  
  // 统计总文件数
  const totalFiles = worksToDownload.reduce((sum, work) => sum + work.files.length, 0);
  
  const batchId = crypto.randomUUID();
  const title = `${selectedMatch.value.title} - ${selectedStage.value.title}`;
  
  // 移除全局 isDownloading 和 downloadState 设置
  // 每个批次独立管理自己的状态
  
  emit('start-download', {
      works: worksToDownload,
      batchId,
      savePath: savePath.value,
      title,
      totalFiles
  });
};

// 下载控制功能
const pauseDownload = async () => {
  try {
    await invoke('pause_downloads');
    downloadState.value = 'Paused';
    console.log('✅ 下载已暂停');
  } catch (error) {
    console.error('Failed to pause:', error);
  }
};

const resumeDownload = async () => {
  try {
    await invoke('resume_downloads');
    downloadState.value = 'Running';
    console.log('✅ 下载已继续');
  } catch (error) {
    console.error('Failed to resume:', error);
  }
};

const stopDownload = async () => {
  if (!confirm('确定停止所有下载吗？已下载的文件会保留，未完成的会丢失。')) {
    return;
  }
  
  try {
    await invoke('stop_downloads');
    downloadState.value = 'Stopped';
    console.log('✅ 已停止所有下载');
  } catch (error) {
    console.error('Failed to stop:', error);
  }
};

// 检查更新功能
const checkForUpdates = async () => {
  if (isCheckingUpdate.value) return;
  
  isCheckingUpdate.value = true;
  try {
    const update = await check();
    console.log('📥 更新检查结果:', update);
    
    if (update) {
      updateAvailable.value = update;
      const yes = await ask(
        `发现新版本 ${update.version}\n\n更新内容:\n${update.body}\n\n是否立即下载并安装？`,
        { title: '发现新版本', kind: 'info', okLabel: '立即更新', cancelLabel: '暂不更新' }
      );
      
      if (yes) {
        console.log('开始下载更新...');
        /* 
           注意：在 macOS 上，downloadAndInstall 会下载并在后台替换 .app。
           如果需要重启，可以使用 relaunch()，但在 dev 环境下可能不生效。
           这里我们提示用户手动重启。
        */
        await update.downloadAndInstall();
        
        await message('更新已完成！\n\n请手动关闭并重新打开应用以使用新版本。', { title: '更新完成', kind: 'info' });
      }
    } else {
      await message('当前已是最新版本', { title: '检查更新', kind: 'info' });
    }
  } catch (error) {
    console.error('检查更新失败:', error);
    await message('检查更新失败: ' + error, { title: '错误', kind: 'error' });
  } finally {
    isCheckingUpdate.value = false;
  }
};

// 退出登录
const logout = async () => {
  const yes = await ask(
    '确定要退出登录吗？',
    { title: '退出登录', kind: 'warning', okLabel: '退出', cancelLabel: '取消' }
  );
  
  if (yes) {
    emit('logout');
  }
};
</script>

<template>
  <div class="dashboard-window">
    <div class="sidebar">
      <div class="sidebar-header">
        <div class="traffic-lights">
          <span class="red"></span><span class="yellow"></span><span class="green"></span>
        </div>
      </div>
      <div class="sidebar-section">
        <h3>比赛列表</h3>
        <div v-if="isLoading" style="padding: 20px; text-align: center; color: var(--text-secondary);">
          加载中...
        </div>
        <ul v-else class="match-list">
          <li 
            v-for="match in matches" 
            :key="match.id" 
            class="match-item"
            :class="{ active: selectedMatch && selectedMatch.id === match.id }"
            @click="selectMatch(match)"
          >
            <span class="icon">🏆</span>
            <span class="title">{{ match.title }}</span>
          </li>
        </ul>
      </div>
    </div>
    
    <div class="main-content" v-if="selectedMatch">
      <div class="content-header">
        <h2>{{ selectedMatch.title }}</h2>
      </div>
      
      <div class="content-body">
        <!-- 系统信息面板 -->
        <div class="system-info-panel">
          <div class="info-section">
            <div class="info-row">
              <span class="info-label">🖥️ CPU 核心数:</span>
              <span class="info-value">{{ systemInfo.cpu_cores }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">⚡ 建议并发:</span>
              <span class="info-value">{{ systemInfo.recommended_concurrency }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">🔧 当前并发:</span>
              <span class="info-value">{{ concurrency }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">📊 下载状态:</span>
              <span class="info-value" :class="{
                'status-running': downloadState === 'Running', 
                'status-paused': downloadState === 'Paused',
                'status-stopped': downloadState === 'Stopped'
              }">
                {{ downloadState }}
              </span>
            </div>
          </div>
          
          <!-- 控制按钮（小巧紧凑） -->
          <div class="control-buttons-compact">
            <!-- 检查更新按钮 - 始终显示 -->
            <button @click="checkForUpdates" :disabled="isCheckingUpdate" class="btn-compact btn-update-compact">
              {{ isCheckingUpdate ? '检查中...' : '🔄 检查更新' }}
            </button>
            
            <!-- 退出登录按钮 -->
            <button @click="logout" class="btn-compact btn-logout-compact">
              🚪 退出登录
            </button>
          </div>
        </div>

        <div class="control-panel">
          <div class="control-group">
            <label>赛段</label>
            <select v-model="selectedStage" @change="selectStage(selectedStage)" class="mac-select">
              <option v-for="stage in stages" :key="stage.id" :value="stage">
                {{ stage.title }}
              </option>
            </select>
          </div>

          <div class="control-group">
            <label>保存位置</label>
             <button class="btn btn-secondary" @click="selectFolder" style="min-width: 150px; justify-content: flex-start; overflow: hidden; text-overflow: ellipsis;">
                {{ savePath ? savePath : '选择文件夹...' }}
             </button>
          </div>
          
          <div class="control-group info">
            <span class="label">作品数量</span>
            <span class="value">{{ works.length }}</span>
          </div>
          
          <button class="btn btn-large" @click="startDownload" :disabled="!selectedStage || works.length === 0">
            开始下载
          </button>
        </div>
        
        <div class="separator"></div>
        
        <!-- Slot for download tasks -->
        <slot name="downloads"></slot>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard-window {
  display: flex;
  width: 100%;
  height: 100vh;
  background-color: var(--bg-color);
}

.sidebar {
  width: 250px;
  background-color: var(--sidebar-bg); /* macOS vibrant dark */
  border-right: 1px solid var(--sidebar-border);
  display: flex;
  flex-direction: column;
  padding-top: 10px;
}

.sidebar-header {
  padding: 10px 16px;
  margin-bottom: 8px;
}

/* Fake macOS Traffic Lights */
.traffic-lights {
  display: flex;
  gap: 8px;
}
.traffic-lights span {
  width: 12px; height: 12px; border-radius: 50%;
}
.red { background: #FF5F56; }
.yellow { background: #FFBD2E; }
.green { background: #27C93F; }

.sidebar-section {
  padding: 0 10px;
}

.match-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.match-item {
  padding: 6px 10px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 2px;
  color: var(--text-primary);
  font-size: 13px;
}

.match-item:hover {
  background-color: var(--item-hover);
}

.match-item.active {
  background-color: var(--item-active);
  color: white;
}
.match-item.active .icon {
  filter: brightness(2);
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-color);
}

.content-header {
  height: 52px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  border-bottom: 1px solid var(--divider);
}

.content-body {
  padding: 24px;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.control-panel {
  display: flex;
  align-items: center;
  gap: 24px;
  margin-bottom: 24px;
  background-color: #262626;
  padding: 16px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--divider);
}

.control-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.control-group label, .control-group .label {
  font-size: 11px;
  color: var(--text-secondary);
  font-weight: 500;
}

.mac-select {
  min-width: 120px;
}

.value {
  font-size: 15px;
  font-weight: 600;
}

.btn-secondary {
    background-color: rgba(255,255,255,0.1);
    color: var(--text-primary);
}
.btn-secondary:hover {
    background-color: rgba(255,255,255,0.15);
}

.separator {
  height: 1px;
  background-color: var(--divider);
  margin-bottom: 24px;
}

/* 系统信息面板 */
.system-info-panel {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.1), rgba(168, 85, 247, 0.1));
  border: 1px solid rgba(99, 102, 241, 0.3);
  border-radius: var(--radius-lg);
  padding: 16px;
  margin-bottom: 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 24px;
}

.info-section {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  flex: 1;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 6px;
}

.info-label {
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 500;
}

.info-value {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
}

.status-running {
  color: #10b981 !important;
}

.status-paused {
  color: #f59e0b !important;
}

.status-stopped {
  color: #ef4444 !important;
}

/* 紧凑控制按钮 */
.control-buttons-compact {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.btn-compact {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.btn-warning-compact {
  background: linear-gradient(135deg, #f59e0b, #d97706);
  color: #fff;
}

.btn-warning-compact:hover {
  background: linear-gradient(135deg, #d97706, #b45309);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(245, 158, 11, 0.4);
}

.btn-success-compact {
  background: linear-gradient(135deg, #10b981, #059669);
  color: #fff;
}

.btn-success-compact:hover {
  background: linear-gradient(135deg, #059669, #047857);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(16, 185, 129, 0.4);
}

.btn-danger-compact {
  background: linear-gradient(135deg, #ef4444, #dc2626);
  color: #fff;
}

.btn-danger-compact:hover {
  background: linear-gradient(135deg, #dc2626, #b91c1c);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(239, 68, 68, 0.4);
}

.btn-update-compact {
  background: linear-gradient(135deg, #3b82f6, #2563eb);
  color: #fff;
}

.btn-update-compact:hover:not(:disabled) {
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(59, 130, 246, 0.4);
}

.btn-update-compact:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-logout-compact {
  background: linear-gradient(135deg, #6b7280, #4b5563);
  color: #fff;
}

.btn-logout-compact:hover {
  background: linear-gradient(135deg, #4b5563, #374151);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(107, 114, 128, 0.4);
}

</style>
```
