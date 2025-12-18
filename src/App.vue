<script setup>
import { ref, onMounted, computed } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import Login from './components/Login.vue';
import Dashboard from './components/Dashboard.vue';
import DownloadPanel from './components/DownloadPanel.vue';

const isLoggedIn = ref(false);
const accessToken = ref('');
const domain = ref('');
const userInfo = ref(null);
const batches = ref([]); // Array of { id, title, totalBytes, downloadedBytes, status, totalFiles, completedFiles, savePath, createdAt }

const handleLoginSuccess = (authData) => {
  isLoggedIn.value = true;
  accessToken.value = authData.token;
  domain.value = authData.domain;
  userInfo.value = authData.userInfo;
};

const handleLogout = () => {
  // 清除 localStorage
  localStorage.removeItem('access_token');
  localStorage.removeItem('school_domain');
  localStorage.removeItem('user_info');
  
  // 重置状态
  isLoggedIn.value = false;
  accessToken.value = '';
  domain.value = '';
  userInfo.value = null;
  batches.value = [];
};

// 保存批次到 localStorage
const saveBatchesToStorage = () => {
  try {
    localStorage.setItem('download_batches', JSON.stringify(batches.value));
  } catch (e) {
    console.error('Failed to save batches to storage:', e);
  }
};

// 从 localStorage 恢复批次
const loadBatchesFromStorage = () => {
  try {
    const saved = localStorage.getItem('download_batches');
    if (saved) {
      batches.value = JSON.parse(saved);
    }
  } catch (e) {
    console.error('Failed to load batches from storage:', e);
  }
};

// 检查是否已有登录状态
onMounted(() => {
  const savedToken = localStorage.getItem('access_token');
  const savedDomain = localStorage.getItem('school_domain');
  const savedUserInfo = localStorage.getItem('user_info');
  
  if (savedToken && savedDomain && savedUserInfo) {
    // 自动恢复登录状态
    isLoggedIn.value = true;
    accessToken.value = savedToken;
    domain.value = savedDomain;
    userInfo.value = JSON.parse(savedUserInfo);
  }
  
  // 恢复下载历史
  loadBatchesFromStorage();
});

const handleStartDownload = async ({ works, batchId, savePath, title, totalFiles }) => {
  try {
    // Register batch locally
    batches.value.unshift({
        id: batchId,
        title: title,
        totalBytes: 0, // We might not know total bytes yet
        downloadedBytes: 0,
        status: 'downloading',
        totalFiles: totalFiles,
        completedFiles: 0,
        subTasks: [], // Store individual file progress if needed, but we summarize for now
        savePath: savePath,
        createdAt: new Date().toISOString()
    });
    
    // 保存到 localStorage
    saveBatchesToStorage();

    await invoke('download_works', { works, batchId, savePath });
  } catch (e) {
    console.error("Failed to start download", e);
  }
};

// 删除批次
const handleDeleteBatch = (batchId) => {
  const index = batches.value.findIndex(b => b.id === batchId);
  if (index !== -1) {
    batches.value.splice(index, 1);
    saveBatchesToStorage();
    console.log('✅ 批次已删除:', batchId);
  }
};

onMounted(async () => {
  await listen('download://progress', (event) => {
    const payload = event.payload; // { id, batch_id, total, current, status }
    
    if (payload.batch_id) {
        const batch = batches.value.find(b => b.id === payload.batch_id);
        if (batch) {
            // Very basic aggregation logic. 
            // Ideally we track each file. For now let's just track completion count and rudimentary bytes.
            const taskIndex = batch.subTasks.findIndex(t => t.id === payload.id);
            if (taskIndex === -1) {
                batch.subTasks.push({ id: payload.id, current: payload.current, total: payload.total, status: payload.status });
            } else {
                batch.subTasks[taskIndex] = { ...batch.subTasks[taskIndex], ...payload };
            }
            
            // Re-calculate batch stats
            let bTotal = 0;
            let bCurrent = 0;
            let bCompleted = 0;
            
            batch.subTasks.forEach(t => {
                bTotal += t.total;
                bCurrent += t.current;
                if (t.status === 'completed') bCompleted++;
            });
            
            batch.totalBytes = bTotal;
            batch.downloadedBytes = bCurrent;
            batch.completedFiles = bCompleted;
            
            // 修复：检查所有任务是否都已完成（包括 completed 和 error 状态）
            const allTasksFinished = batch.subTasks.length === batch.totalFiles && 
                                    batch.subTasks.every(t => t.status === 'completed' || t.status === 'error');
            
            // 优先处理明确的状态信号
            if (payload.status === 'paused') {
                batch.status = 'paused';
            } else if (payload.status === 'stopped') {
                batch.status = 'stopped';
            } else if (allTasksFinished) {
                // 如果所有文件都成功下载，状态为 completed
                if (bCompleted === batch.totalFiles) {
                    batch.status = 'completed';
                } else {
                    // 如果有失败的文件，状态为 partial（部分完成）
                    batch.status = 'partial';
                }
            } else if (payload.status === 'downloading') {
                // 只有收到下载信号时才设置为 downloading
                batch.status = 'downloading';
            }
            
            // 每次更新都保存到 localStorage
            saveBatchesToStorage();
        }
    }
  });
});
</script>

<template>
  <div class="app-container">
    <transition name="fade" mode="out-in">
      <Login v-if="!isLoggedIn" @login-success="handleLoginSuccess" />
      
      <Dashboard 
        v-else 
        :accessToken="accessToken"
        :domain="domain"
        :userInfo="userInfo"
        :batches="batches"
        @start-download="handleStartDownload"
        @logout="handleLogout"
      >
        <template #downloads>
          <DownloadPanel :batches="batches" @delete-batch="handleDeleteBatch" />
        </template>
      </Dashboard>
    </transition>
  </div>
</template>

<style scoped>
.app-container {
  width: 100%;
  height: 100%;
  border-radius: 16px;
  overflow: hidden;
}
</style>
