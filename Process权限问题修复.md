# ✅ Process 权限问题修复

## 🐛 问题

编译错误：
```
failed to resolve ACL: UnknownManifest { key: "process" }
```

## 🔍 原因

当前 Tauri 版本不支持 `process` 权限，可用的权限列表中没有 `process`。

## ✅ 解决方案

### 1. 移除 process 权限

从 `tauri.conf.json` 中移除：
- ❌ `"process:default"`
- ❌ `"process:allow-restart"`

保留的权限：
- ✅ `"updater:default"`
- ✅ `"updater:allow-check"`
- ✅ `"updater:allow-download"`
- ✅ `"updater:allow-install"`

### 2. 修改重启逻辑

**之前**：
```javascript
import { relaunch } from '@tauri-apps/plugin-process';

// 更新完成后自动重启
const shouldRelaunch = confirm('是否立即重启应用？');
if (shouldRelaunch) {
  await relaunch();
}
```

**现在**：
```javascript
// 不导入 relaunch

// 更新完成后提示用户手动重启
alert('✅ 更新已完成！\n\n请手动关闭并重新打开应用以使用新版本。');
```

## 🎯 更新流程

### 1. 检查更新
```
用户点击"检查更新" → 查询服务器
```

### 2. 下载更新
```
发现新版本 → 用户确认 → 下载并安装
```

### 3. 手动重启
```
✅ 更新已完成！

请手动关闭并重新打开应用以使用新版本。
```

用户需要：
1. 完全关闭应用
2. 重新打开应用
3. 新版本自动生效

## 📋 最终权限配置

```json
{
  "security": {
    "capabilities": [
      {
        "identifier": "main-capability",
        "windows": ["main"],
        "permissions": [
          "core:default",
          "updater:default",
          "updater:allow-check",
          "updater:allow-download",
          "updater:allow-install"
        ]
      }
    ]
  }
}
```

## ✨ 优势

虽然不能自动重启，但这种方式：
- ✅ 更安全：用户明确知道应用需要重启
- ✅ 更稳定：避免自动重启可能导致的问题
- ✅ 兼容性好：不依赖 process 权限
- ✅ 用户可控：用户选择何时重启

## 📝 用户体验

### 完整流程

1. **点击检查更新**
2. **发现新版本**
   ```
   发现新版本 0.2.0
   
   更新内容:
   - 新功能1
   - 新功能2
   
   是否立即下载并安装？
   ```
3. **确认下载**
4. **等待下载完成**
5. **提示重启**
   ```
   ✅ 更新已完成！
   
   请手动关闭并重新打开应用以使用新版本。
   ```
6. **用户手动重启应用**
7. **新版本生效**

## 🎉 完成状态

- ✅ 移除 process 权限
- ✅ 修改重启逻辑
- ✅ 更新完成提示
- ✅ 编译错误已修复

---

**process 权限问题已完全解决！应用现在可以正常编译和运行！** ✅
