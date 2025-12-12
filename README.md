# 位来足迹 - 校赛作品下载器

一个基于 Tauri + Vue 3 开发的桌面应用，用于批量下载校赛作品。

## ✨ 功能特性

- 🚀 支持批量下载作品
- 🔐 OAuth 认证登录
- 📊 下载进度实时显示
- 🔄 自动更新功能
- 🎨 现代化 UI 设计
- 💻 跨平台支持（Windows、macOS、Linux）

## 🛠️ 技术栈

- **前端**: Vue 3 + Vite
- **后端**: Rust + Tauri 2.0
- **HTTP 客户端**: Reqwest
- **异步运行时**: Tokio

## 📦 下载安装

前往 [Releases](../../releases) 页面下载最新版本：

- **Windows**: 下载 `.msi` 或 `.exe` 安装包
- **macOS ARM64** (M1/M2/M3/M4): 下载 `aarch64.dmg`
- **macOS Intel**: 下载 `x64.dmg`
- **Linux**: 下载 `.AppImage` 或 `.deb`

## 🚀 开发指南

### 环境要求

- Node.js 18+
- pnpm 8+
- Rust 1.70+
- 系统依赖（根据平台不同）

### 安装依赖

```bash
# 安装前端依赖
pnpm install

# 安装 Rust 依赖（自动）
cd src-tauri
cargo build
```

### 开发运行

```bash
# 启动开发服务器
pnpm tauri dev
```

### 本地构建

```bash
# 构建当前平台的安装包
pnpm tauri build
```

## 📤 发布新版本

### 方法一：使用自动化脚本（推荐）

```bash
# 一键发布，自动更新版本号并推送
./release.sh v1.0.0
```

### 方法二：手动发布

```bash
# 1. 更新版本号
# 编辑 src-tauri/tauri.conf.json 和 src-tauri/Cargo.toml

# 2. 提交更改
git add .
git commit -m "chore: bump version to v1.0.0"

# 3. 创建并推送 tag
git tag v1.0.0
git push origin main
git push origin v1.0.0
```

### 方法三：GitHub 手动触发

1. 打开 GitHub 仓库的 **Actions** 标签
2. 选择 **Build and Release** 工作流
3. 点击 **Run workflow**
4. 输入版本号并运行

详细说明请查看 [GitHub Actions 使用说明](./GITHUB_ACTIONS_使用说明.md)

## 🔐 配置签名（可选）

为了启用自动更新功能，需要配置 Tauri 签名：

```bash
# 1. 生成密钥对
python3 gen_key.py

# 2. 在 GitHub 仓库设置中添加 Secrets：
#    - TAURI_PRIVATE_KEY: 私钥内容
#    - TAURI_KEY_PASSWORD: 密钥密码（如有）

# 3. 更新 src-tauri/tauri.conf.json 中的 pubkey
```

## 📁 项目结构

```
match_download/
├── src/                    # Vue 前端源码
├── src-tauri/             # Rust 后端源码
│   ├── src/
│   │   ├── main.rs        # 主入口
│   │   └── downloader.rs  # 下载器模块
│   ├── Cargo.toml         # Rust 依赖配置
│   └── tauri.conf.json    # Tauri 配置
├── .github/
│   └── workflows/
│       └── build.yml      # GitHub Actions 配置
├── release.sh             # 自动发布脚本
└── README.md
```

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 开源协议

MIT License

## 🔗 相关文档

- [Tauri 官方文档](https://tauri.app/)
- [Vue 3 文档](https://vuejs.org/)
- [GitHub Actions 使用说明](./GITHUB_ACTIONS_使用说明.md)

## 💡 推荐 IDE 配置

- [VS Code](https://code.visualstudio.com/)
- [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
