# GitHub Actions 自动构建使用说明

## 📋 功能说明

这个 GitHub Actions 工作流可以自动构建以下平台的安装包：

- ✅ **Windows x64** (.msi, .exe)
- ✅ **macOS ARM64** (M1/M2/M3/M4) (.dmg, .app)
- ✅ **macOS x64** (Intel) (.dmg, .app)
- ✅ **Linux x64** (.AppImage, .deb)

## 🚀 使用方法

### 方法一：通过 Git Tag 触发（推荐）

1. **提交你的代码**
   ```bash
   git add .
   git commit -m "准备发布 v1.0.0"
   ```

2. **创建并推送 tag**
   ```bash
   # 创建 tag（版本号格式：v主版本.次版本.修订号）
   git tag v1.0.0
   
   # 推送 tag 到 GitHub
   git push origin v1.0.0
   ```

3. **自动构建**
   - GitHub Actions 会自动检测到新 tag
   - 开始构建所有平台的安装包
   - 构建完成后自动创建 Release
   - 所有安装包会自动上传到 Release

### 方法二：手动触发

1. 打开你的 GitHub 仓库
2. 点击 **Actions** 标签
3. 选择 **Build and Release** 工作流
4. 点击右侧 **Run workflow** 按钮
5. 输入版本号（可选，默认 v0.0.1）
6. 点击 **Run workflow** 开始构建

## 🔐 配置签名（可选但推荐）

为了让应用更新功能正常工作，需要配置 Tauri 签名密钥：

### 1. 生成密钥对

项目中已有 `gen_key.py` 脚本，运行它生成密钥：

```bash
python3 gen_key.py
```

这会生成两个文件：
- `match_download.key` - 私钥（保密！）
- `match_download.pub` - 公钥

### 2. 配置 GitHub Secrets

1. 打开 GitHub 仓库设置：**Settings** > **Secrets and variables** > **Actions**
2. 点击 **New repository secret** 添加以下密钥：

   **TAURI_PRIVATE_KEY**
   ```bash
   # 复制私钥内容
   cat match_download.key
   # 将内容粘贴到 Secret 值中
   ```

   **TAURI_KEY_PASSWORD**
   ```
   # 如果生成密钥时设置了密码，填写密码
   # 如果没有密码，留空或填写空字符串
   ```

### 3. 更新公钥到配置文件

将 `match_download.pub` 的内容更新到 `src-tauri/tauri.conf.json` 的 `pubkey` 字段：

```json
{
  "plugins": {
    "updater": {
      "pubkey": "这里粘贴公钥内容"
    }
  }
}
```

## 📦 构建产物说明

构建完成后，在 Release 页面可以下载到以下文件：

### Windows
- `位来足迹-校赛作品下载器_x.x.x_x64_en-US.msi` - MSI 安装包（推荐）
- `位来足迹-校赛作品下载器_x.x.x_x64-setup.exe` - EXE 安装包

### macOS ARM64 (M1/M2/M3/M4)
- `位来足迹-校赛作品下载器_x.x.x_aarch64.dmg` - DMG 镜像文件

### macOS x64 (Intel)
- `位来足迹-校赛作品下载器_x.x.x_x64.dmg` - DMG 镜像文件

### Linux
- `位来足迹-校赛作品下载器_x.x.x_amd64.AppImage` - AppImage 格式
- `位来足迹-校赛作品下载器_x.x.x_amd64.deb` - Debian 包

## 🔍 查看构建状态

1. 打开 GitHub 仓库的 **Actions** 标签
2. 查看最新的工作流运行状态
3. 点击具体的运行记录可以查看详细日志
4. 构建时间大约 10-20 分钟（取决于 GitHub 服务器负载）

## ⚠️ 注意事项

1. **首次使用前**：确保已将代码推送到 GitHub
2. **版本号格式**：必须以 `v` 开头，例如 `v1.0.0`
3. **构建时间**：完整构建所有平台大约需要 15-25 分钟
4. **私钥安全**：永远不要将 `.key` 私钥文件提交到 Git 仓库
5. **Release 权限**：确保 GitHub Actions 有创建 Release 的权限

## 🐛 常见问题

### Q: 构建失败怎么办？
A: 
1. 检查 Actions 日志中的错误信息
2. 确保 `package.json` 和 `Cargo.toml` 配置正确
3. 确保所有依赖都已正确声明

### Q: 如何只构建 Windows 版本？
A: 修改 `.github/workflows/build.yml`，删除或注释掉其他平台的配置

### Q: 能在本地测试构建吗？
A: 可以使用 `pnpm tauri build` 在本地构建当前平台的版本

### Q: 如何删除错误的 Release？
A: 在 GitHub 仓库的 Releases 页面，点击对应 Release 的删除按钮

## 📝 版本发布流程示例

```bash
# 1. 更新版本号
# 编辑 src-tauri/tauri.conf.json 中的 version 字段
# 编辑 src-tauri/Cargo.toml 中的 version 字段

# 2. 提交更改
git add .
git commit -m "chore: bump version to v1.0.0"

# 3. 创建并推送 tag
git tag v1.0.0
git push origin main
git push origin v1.0.0

# 4. 等待 GitHub Actions 自动构建
# 5. 在 Releases 页面查看并下载构建产物
```

## 🎯 下一步

1. ✅ 将代码推送到 GitHub
2. ✅ 配置签名密钥（可选）
3. ✅ 创建第一个 tag 触发构建
4. ✅ 在 Releases 页面下载安装包

祝你构建顺利！🎉
