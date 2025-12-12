<script setup>
defineProps({
  batches: {
    type: Array,
    required: true
  }
});
</script>

<template>
  <div class="download-panel">
    <div class="panel-header">
      <h3>下载任务</h3>
      <span class="count" v-if="batches.length">{{ batches.length }} 项</span>
    </div>
    <div class="tasks-list">
      <div v-for="batch in batches" :key="batch.id" class="task-item">
        <div class="task-icon">📦</div>
        <div class="task-details">
          <div class="task-line-1">
            <span class="task-name">{{ batch.title }}</span>
            <span class="task-status">{{ batch.completedFiles }} / {{ batch.totalFiles }} 文件</span>
          </div>
          <div class="task-progress-bg">
            <div 
              class="task-progress-fill" 
              :style="{ width: (batch.totalFiles > 0 ? (batch.completedFiles / batch.totalFiles * 100) : 0) + '%' }"
            ></div>
          </div>
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
}

.task-item:hover {
  background: var(--item-hover);
}

.task-icon {
  font-size: 24px;
  opacity: 0.8;
}

.task-details {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.task-line-1 {
  display: flex;
  justify-content: space-between;
}

.task-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.task-status {
  font-size: 11px;
  color: var(--text-secondary);
}

.task-progress-bg {
  height: 6px;
  background: rgba(255,255,255,0.1);
  border-radius: 3px;
  overflow: hidden;
  margin-top: 4px;
}

.task-progress-fill {
  height: 100%;
  background: var(--accent-color);
  transition: width 0.2s linear;
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
