<script setup>
import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';

const props = defineProps({
  batches: {
    type: Array,
    required: true
  }
});

const emit = defineEmits(['delete-batch']);

// 打开保存的文件夹
const openFolder = async (savePath) => {
  if (!savePath) {
    console.error('No save path provided');
    return;
  }
  
  try {
    await invoke('open_folder', { path: savePath });
  } catch (error) {
    console.error('Failed to open folder:', error);
    // 使用非阻塞通知或自定义UI更好，这里暂用console
  }
};

// 暂停单个任务
const pauseBatch = async (batchId) => {
  try {
    console.log('暂停任务:', batchId);
    // 乐观更新：立即在界面上反馈
    const batch = props.batches.find(b => b.id === batchId);
    if (batch) batch.status = 'paused';
    
    await invoke('pause_batch', { batchId });
  } catch (error) {
    console.error('暂停任务失败:', error);
  }
};

// 继续单个任务
const resumeBatch = async (batchId) => {
  try {
    console.log('继续任务:', batchId);
    // 乐观更新：立即在界面上反馈
    const batch = props.batches.find(b => b.id === batchId);
    if (batch) batch.status = 'downloading';

    await invoke('resume_batch', { batchId });
  } catch (error) {
    console.error('继续任务失败:', error);
    // 回滚
    const batch = props.batches.find(b => b.id === batchId);
    if (batch) batch.status = 'paused';
  }
};

// 停止单个任务
const stopBatch = async (batchId) => {
  const confirmed = await ask('确定要停止这个下载任务吗？已下载的文件会保留。', {
    title: '停止下载',
    kind: 'warning',
    okLabel: '停止',
    cancelLabel: '取消'
  });
  
  if (!confirmed) return;
  
  // 乐观更新
  const batch = props.batches.find(b => b.id === batchId);
  if (batch) batch.status = 'stopped';
  
  try {
    await invoke('stop_batch', { batchId });
    console.log('✅ 任务已停止:', batchId);
  } catch (error) {
    console.error('停止任务失败:', error);
  }
};

// 删除批次记录
const deleteBatch = async (batchId) => {
  const confirmed = await ask('确定要删除这条记录吗？文件不会被删除。', {
    title: '删除记录',
    kind: 'warning',
    okLabel: '删除',
    cancelLabel: '取消'
  });
  
  if (!confirmed) return;
  
  emit('delete-batch', batchId);
  console.log('🗑️ 删除批次:', batchId);
};

// 获取状态显示文本
const getStatusText = (status) => {
  const statusMap = {
    'downloading': '下载中',
    'completed': '已完成',
    'partial': '部分完成',
    'error': '失败',
    'paused': '已暂停',
    'stopped': '已停止'
  };
  return statusMap[status] || status;
};

// SVG 图标定义在下面
const getStatusIcon = (status) => {
  if (status === 'downloading') return Icons.Play; // 或者其他下载图标
  if (status === 'paused') return Icons.Pause;
  if (status === 'stopped') return Icons.Stop;
  if (status === 'completed') return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#10b981" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
  if (status === 'error') return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>`;
  return Icons.Folder; 
};

// SVG 图标
const Icons = {
  Pause: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="6" y="4" width="4" height="16"></rect><rect x="14" y="4" width="4" height="16"></rect></svg>`,
  Play: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>`,
  Stop: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect></svg>`,
  Folder: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`,
  Trash: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>`
};
</script>

<style scoped>
/* Apple Design Style Buttons */
.task-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.btn-task-action {
  width: 32px;
  height: 32px;
  border-radius: 50%; /* 圆形按钮 */
  border: none;
  background-color: #f1f5f9; /* 浅灰色背景 */
  color: #475569; /* 深灰色图标 */
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.btn-task-action:hover {
  transform: scale(1.05); /* 轻微放大 */
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
}

.btn-task-action:active {
  transform: scale(0.95);
  box-shadow: inset 0 2px 4px 0 rgba(0, 0, 0, 0.06);
}

/* Specific button styles */
.btn-pause:hover {
  background-color: #fff7ed;
  color: #f59e0b;
}

.btn-resume:hover {
  background-color: #f0fdf4;
  color: #10b981;
}

.btn-stop:hover {
  background-color: #fef2f2;
  color: #ef4444;
}

.btn-folder:hover {
  background-color: #eff6ff;
  color: #3b82f6;
}

.btn-delete:hover {
  background-color: #fef2f2;
  color: #ef4444; /* Standard destructive red */
}
</style>

<template>
  <div class="download-panel">
    <div class="panel-header">
      <h3>下载任务</h3>
      <span class="count" v-if="batches.length">{{ batches.length }} 项</span>
    </div>
    <div class="tasks-list">
      <div v-for="batch in batches" :key="batch.id" class="task-item">
        <div class="task-icon" v-html="getStatusIcon(batch.status)"></div>
        <div class="task-details">
          <div class="task-line-1">
            <span class="task-name">{{ batch.title }}</span>
            <span class="task-status" :class="`status-${batch.status}`">
              {{ getStatusText(batch.status) }}
            </span>
          </div>
          <div class="task-line-2">
            <span class="task-files">{{ batch.completedFiles }} / {{ batch.totalFiles }} 文件</span>
            <span class="task-date" v-if="batch.createdAt">
              {{ new Date(batch.createdAt).toLocaleString('zh-CN', { 
                month: '2-digit', 
                day: '2-digit', 
                hour: '2-digit', 
                minute: '2-digit' 
              }) }}
            </span>
          </div>
          <div class="task-progress-bg">
            <div 
              class="task-progress-fill" 
              :class="`progress-${batch.status}`"
              :style="{ width: (batch.totalFiles > 0 ? (batch.completedFiles / batch.totalFiles * 100) : 0) + '%' }"
            ></div>
          </div>
        </div>
        
        <!-- 任务控制按钮 -->
        <div class="task-actions">
          <!-- 下载中：显示暂停停止 -->
          <template v-if="batch.status === 'downloading'">
            <button 
              @click="pauseBatch(batch.id)" 
              class="btn-task-action btn-pause"
              title="暂停"
              v-html="Icons.Pause"
            >
            </button>
            <button 
              @click="stopBatch(batch.id)" 
              class="btn-task-action btn-stop"
              title="停止"
              v-html="Icons.Stop"
            >
            </button>
          </template>
          
          <!-- 已暂停：显示继续和停止 -->
          <template v-else-if="batch.status === 'paused'">
            <button 
              @click="resumeBatch(batch.id)" 
              class="btn-task-action btn-resume"
              title="继续"
              v-html="Icons.Play"
            >
            </button>
            <button 
              @click="stopBatch(batch.id)" 
              class="btn-task-action btn-stop"
              title="停止"
              v-html="Icons.Stop"
            >
            </button>
          </template>
          
          <!-- 其他状态：显示打开文件夹按钮 -->
          <template v-else>
            <button 
              v-if="batch.savePath" 
              @click="openFolder(batch.savePath)" 
              class="btn-task-action btn-folder"
              title="打开保存目录"
              v-html="Icons.Folder"
            >
            </button>
          </template>
          
          <!-- 删除按钮 - 所有状态都显示 -->
          <button 
            @click="deleteBatch(batch.id)" 
            class="btn-task-action btn-delete"
            title="删除记录"
            v-html="Icons.Trash"
          >
          </button>
        </div>
      </div>
      
      <div v-if="batches.length === 0" class="empty-state">
        <p>暂无下载任务</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.download-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: #232323;
  border: 1px solid var(--divider);
  border-radius: var(--radius-lg);
}

.panel-header {
  padding: 12px 16px;
  background: rgba(255,255,255,0.03);
  border-bottom: 1px solid var(--divider);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.count {
  font-size: 11px;
  background: rgba(255,255,255,0.1);
  padding: 2px 8px;
  border-radius: 10px;
  color: var(--text-secondary);
}

.tasks-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.task-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 6px;
  background: rgba(255,255,255,0.02);
  margin-bottom: 8px;
  transition: all 0.2s;
}

.task-item:hover {
  background: var(--item-hover);
}

.task-icon {
  font-size: 24px;
  opacity: 0.8;
  flex-shrink: 0;
}

.task-details {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.task-line-1 {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.task-line-2 {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: var(--text-secondary);
}

.task-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-status {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 4px;
  white-space: nowrap;
  flex-shrink: 0;
}

.status-downloading {
  background: rgba(59, 130, 246, 0.2);
  color: #3b82f6;
}

.status-completed {
  background: rgba(16, 185, 129, 0.2);
  color: #10b981;
}

.status-partial {
  background: rgba(245, 158, 11, 0.2);
  color: #f59e0b;
}

.status-error {
  background: rgba(239, 68, 68, 0.2);
  color: #ef4444;
}

.status-paused {
  background: rgba(156, 163, 175, 0.2);
  color: #9ca3af;
}

.task-files {
  color: var(--text-secondary);
}

.task-date {
  color: var(--text-tertiary);
  font-size: 10px;
}

.task-progress-bg {
  height: 6px;
  background: rgba(255,255,255,0.1);
  border-radius: 3px;
  overflow: hidden;
  margin-top: 2px;
}

.task-progress-fill {
  height: 100%;
  transition: width 0.2s linear;
}

.progress-downloading {
  background: linear-gradient(90deg, #3b82f6, #2563eb);
}

.progress-completed {
  background: linear-gradient(90deg, #10b981, #059669);
}

.progress-partial {
  background: linear-gradient(90deg, #f59e0b, #d97706);
}

.progress-error {
  background: linear-gradient(90deg, #ef4444, #dc2626);
}

.progress-paused {
  background: linear-gradient(90deg, #9ca3af, #6b7280);
}

/* 任务操作按钮容器 */
.task-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
  align-items: center;
}

/* 任务操作按钮基础样式 */
.btn-task-action {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 16px;
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
  min-width: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-task-action:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.2);
  transform: translateY(-1px);
}

.btn-task-action:active {
  transform: translateY(0);
}

/* 暂停按钮 */
.btn-pause:hover {
  background: rgba(245, 158, 11, 0.2);
  border-color: rgba(245, 158, 11, 0.4);
}

/* 继续按钮 */
.btn-resume:hover {
  background: rgba(16, 185, 129, 0.2);
  border-color: rgba(16, 185, 129, 0.4);
}

/* 停止按钮 */
.btn-stop:hover {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.4);
}

/* 文件夹按钮 */
.btn-folder:hover {
  background: rgba(59, 130, 246, 0.2);
  border-color: rgba(59, 130, 246, 0.4);
}

/* 删除按钮 */
.btn-delete:hover {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.4);
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  color: var(--text-tertiary);
  font-size: 13px;
}

</style>
