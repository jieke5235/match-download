<script setup>
import { ref, onMounted, computed } from 'vue';
import { invoke } from "@tauri-apps/api/core";

const emit = defineEmits(['login-success']);

const schools = ref([]);
const selectedSchool = ref(null);
const isLoadingSchools = ref(false);
const isLoading = ref(false);
const errorMessage = ref('');
const searchQuery = ref('');
const isDropdownOpen = ref(false);

onMounted(async () => {
  await loadSchools();
});

const loadSchools = async () => {
  try {
    isLoadingSchools.value = true;
    const allSchools = await invoke("get_schools");
    schools.value = allSchools.filter(school => school.domain);
    
    if (schools.value.length > 0) {
      selectedSchool.value = schools.value[0];
    }
  } catch (error) {
    console.error('Failed to load schools details:', error);
    errorMessage.value = '加载学校列表失败: ' + JSON.stringify(error);
  } finally {
    isLoadingSchools.value = false;
  }
};

const filteredSchools = computed(() => {
  if (!searchQuery.value) {
    return schools.value;
  }
  const query = searchQuery.value.toLowerCase();
  return schools.value.filter(school => 
    school.school_name.toLowerCase().includes(query)
  );
});

const selectSchool = (school) => {
  selectedSchool.value = school;
  isDropdownOpen.value = false;
  searchQuery.value = '';
};

const toggleDropdown = () => {
  if (!isLoadingSchools.value && !isLoading.value) {
    isDropdownOpen.value = !isDropdownOpen.value;
    if (isDropdownOpen.value) {
      searchQuery.value = '';
    }
  }
};

const login = async () => {
  if (!selectedSchool.value) {
    errorMessage.value = '请选择学校';
    return;
  }
  
  try {
    isLoading.value = true;
    errorMessage.value = '';
    
    const domain = selectedSchool.value.domain;
    
    // Step 1: Start OAuth flow (opens browser, returns code)
    const code = await invoke("start_oauth", { domain });
    console.log('Got authorization code:', code);
    
    // Step 2: Exchange code for access token
    const tokenData = await invoke("exchange_token", { code, domain });
    console.log('Got token data:', tokenData);
    
    // Step 3: Get user info
    const userInfo = await invoke("get_user_info", { 
      accessToken: tokenData.access_token, 
      domain 
    });
    console.log('Got user info:', userInfo);
    
    // 存储认证信息到 localStorage
    localStorage.setItem('access_token', tokenData.access_token);
    localStorage.setItem('school_domain', domain);
    localStorage.setItem('school_name', selectedSchool.value.school_name);
    localStorage.setItem('user_info', JSON.stringify(userInfo));
    
    // 发送登录成功事件，传递完整的认证数据
    emit('login-success', {
      token: tokenData.access_token,
      domain: domain,
      userInfo: userInfo
    });
    
  } catch (error) {
    console.error('OAuth error:', error);
    errorMessage.value = typeof error === 'string' ? error : '登录失败，请重试';
  } finally {
    isLoading.value = false;
  }
};
</script>

<template>
  <div class="login-container">
    <div class="login-window">
       <div class="window-controls">
         <span class="close"></span>
       </div>
       <div class="login-content">
          <div class="app-icon">
            <svg width="64" height="64" viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
              <rect width="64" height="64" rx="16" fill="#0A84FF"/>
              <path d="M32 16L48 48H16L32 16Z" fill="white"/>
            </svg>
          </div>
          <h1>位来足迹-比赛作品下载器</h1>
          <p>请选择学校并登录</p>
          
          <div v-if="errorMessage" class="error-message">
            {{ errorMessage }}
          </div>
          
          <!-- School Selection -->
          <div class="form-group">
            <label>选择学校</label>
            <div class="custom-select" :class="{ 'disabled': isLoadingSchools || isLoading }">
              <div class="select-trigger" @click="toggleDropdown">
                <span v-if="isLoadingSchools">加载中...</span>
                <span v-else-if="selectedSchool">{{ selectedSchool.school_name }}</span>
                <span v-else class="placeholder">请选择学校</span>
                <svg class="dropdown-icon" :class="{ 'open': isDropdownOpen }" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M4 6l4 4 4-4"/>
                </svg>
              </div>
              
              <div v-if="isDropdownOpen" class="dropdown-menu">
                <div class="search-box">
                  <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"/>
                  </svg>
                  <input 
                    type="text" 
                    v-model="searchQuery" 
                    placeholder="搜索学校..." 
                    class="search-input"
                    @click.stop
                  />
                </div>
                
                <div class="options-list">
                  <div 
                    v-for="school in filteredSchools" 
                    :key="school.id"
                    class="option-item"
                    :class="{ 'selected': selectedSchool && selectedSchool.id === school.id }"
                    @click="selectSchool(school)"
                  >
                    {{ school.school_name }}
                  </div>
                  <div v-if="filteredSchools.length === 0" class="no-results">
                    未找到匹配的学校
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <button 
            class="btn btn-large" 
            @click="login"
            :disabled="isLoading || isLoadingSchools || !selectedSchool"
          >
            {{ isLoading ? '登录中...' : 'OAuth 登录' }}
          </button>
          
          <div v-if="isLoading" class="loading-text">
            正在打开浏览器进行认证...
          </div>
       </div>
    </div>
  </div>
</template>

<style scoped>
/* Global Reset within component scope */
* {
  box-sizing: border-box;
}

.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  background: var(--bg-color);
}

/* ... existing styles ... */

.select-trigger {
  width: 100%;
  padding: 12px 16px; /* 增加左右内边距，增加高度 */
  background: rgba(255,255,255,0.1);
  border: 1px solid rgba(255,255,255,0.2);
  border-radius: 8px; /* 稍微加大圆角 */
  color: var(--text-primary);
  font-size: 14px; /* 稍微增大字号 */
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.2s;
  height: 48px; /* 固定高度确保与按钮一致 */
}

/* ... */

.btn-large {
  width: 100%;
  height: 48px; /* 确保按钮高度一致 */
  font-size: 14px;
}

.login-window {
  width: 360px;
  background: #2C2C2C;
  border-radius: 12px;
  box-shadow: 0 20px 40px rgba(0,0,0,0.4);
  border: 1px solid rgba(255,255,255,0.1);
  /* overflow: hidden; Removed to allow dropdown to overflow */
  display: flex;
  flex-direction: column;
  position: relative; /* Ensure z-index context works */
}

.window-controls {
  height: 28px;
  background: rgba(255,255,255,0.03);
  display: flex;
  align-items: center;
  padding-left: 10px;
  border-radius: 12px 12px 0 0; /* Add radius since overflow is no longer hidden */
}
.close {
  width: 10px; height: 10px; background: #FF5F56; border-radius: 50%;
}

.login-content {
  padding: 40px 30px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.app-icon {
  margin-bottom: 24px;
  filter: drop-shadow(0 4px 8px rgba(0,0,0,0.2));
}

h1 {
  font-size: 20px;
  margin-bottom: 8px;
}

p {
  color: var(--text-secondary);
  font-size: 13px;
  margin-bottom: 24px;
}

.error-message {
  background: rgba(245, 101, 101, 0.1);
  border: 1px solid #f56565;
  color: #f56565;
  padding: 12px;
  border-radius: 6px;
  margin-bottom: 16px;
  font-size: 12px;
  width: 100%;
}

.form-group {
  width: 100%;
  margin-bottom: 20px;
  text-align: left;
}

.form-group label {
  display: block;
  font-size: 11px;
  color: var(--text-secondary);
  margin-bottom: 6px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.custom-select {
  position: relative;
  width: 100%;
}

.custom-select.disabled {
  opacity: 0.5;
  pointer-events: none;
}

.select-trigger {
  width: 100%;
  padding: 10px 12px;
  background: rgba(255,255,255,0.1);
  border: 1px solid rgba(255,255,255,0.2);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.2s;
}

.select-trigger:hover {
  background: rgba(255,255,255,0.15);
}

.select-trigger .placeholder {
  color: var(--text-secondary);
}

.dropdown-icon {
  transition: transform 0.2s;
  color: var(--text-secondary);
}

.dropdown-icon.open {
  transform: rotate(180deg);
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 8px); /* 增加垂直间距 */
  left: 0;
  right: 0;
  background: #2C2C2C;
  border: 1px solid rgba(255,255,255,0.2);
  border-radius: 8px; /* 圆角加大 */
  overflow: hidden;
  z-index: 1000;
  box-shadow: 0 10px 25px rgba(0,0,0,0.5); /* 阴影加深 */
  padding: 4px 0; /* 增加整体内边距 */
}

.search-box {
  padding: 10px 12px; /* 增加搜索框内边距 */
  border-bottom: 1px solid rgba(255,255,255,0.1);
  display: flex;
  align-items: center;
  gap: 10px;
}

.search-icon {
  color: var(--text-secondary);
  flex-shrink: 0;
  opacity: 0.7;
}

.search-input {
  flex: 1;
  background: rgba(255,255,255,0.08); /* 稍微调亮背景 */
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 4px;
  padding: 8px 10px; /* 增加输入框高度 */
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  transition: all 0.2s;
}

.search-input:focus {
  background: rgba(255,255,255,0.12);
  border-color: var(--accent-color);
}

.options-list {
  max-height: 220px; /* 稍微增高 */
  overflow-y: auto;
  padding: 4px 0;
}

.option-item {
  padding: 12px 16px; /* 增加选项点击区域 */
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
  transition: background 0.15s;
  display: flex;
  align-items: center;
}


.option-item:hover {
  background: rgba(255,255,255,0.1);
}

.option-item.selected {
  background: rgba(10, 132, 255, 0.2);
  color: var(--accent-color);
}

.no-results {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
}

.school-select {
  width: 100%;
  padding: 10px 12px;
  background: rgba(255,255,255,0.1);
  border: 1px solid rgba(255,255,255,0.2);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  cursor: pointer;
  transition: all 0.2s;
}

.school-select:hover:not(:disabled) {
  background: rgba(255,255,255,0.15);
}

.school-select:focus {
  background: rgba(255,255,255,0.15);
  border-color: var(--accent-color);
  box-shadow: 0 0 0 2px rgba(10, 132, 255, 0.2);
}

.school-select:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.school-select option {
  background: #2C2C2C;
  color: white;
}

.btn-large {
  width: 100%;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.loading-text {
  margin-top: 16px;
  font-size: 12px;
  color: var(--text-secondary);
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
</style>
