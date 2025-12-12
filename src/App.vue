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
const batches = ref([]); // Array of { id, title, totalBytes, downloadedBytes, status, totalFiles, completedFiles }

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
        subTasks: [] // Store individual file progress if needed, but we summarize for now
    });

    await invoke('download_works', { works, batchId, savePath });
  } catch (e) {
    console.error("Failed to start download", e);
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
            
            if (bCompleted === batch.totalFiles) {
                batch.status = 'completed';
            }
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
        @start-download="handleStartDownload"
        @logout="handleLogout"
      >
        <template #downloads>
          <DownloadPanel :batches="batches" />
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
