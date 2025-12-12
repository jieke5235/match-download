#!/bin/bash
# 构建 ARM64 版本并生成更新包

echo "🚀 开始构建 macOS ARM64 版本..."

# 读取私钥
if [ -f "$HOME/.tauri/wlzj-match-downloader.key" ]; then
    export TAURI_SIGNING_PRIVATE_KEY=$(cat "$HOME/.tauri/wlzj-match-downloader.key")
    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="121212" # 如果有密码请在此设置
    echo "🔑 已加载私钥"
else
    echo "❌ 未找到私钥文件: $HOME/.tauri/wlzj-match-downloader.key"
    echo "请先生成密钥：npm run tauri signer generate -- -w $HOME/.tauri/wlzj-match-downloader.key"
    exit 1
fi

# 构建
npm run tauri build -- --target aarch64-apple-darwin

if [ $? -eq 0 ]; then
    echo "✅ 构建成功！"
    
    VERSION=$(grep '"version":' package.json | cut -d '"' -f 4)
    # 查找生成的文件（可能有空格，使用通配符匹配）
    BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle/macos"
    APP_NAME="位来足迹-校赛作品下载器.app"
    
    # 查找 .APP.TAR.GZ 文件 (忽略大小写)
    UPDATE_FILE=$(find "$BUNDLE_DIR" -name "*.app.tar.gz" | head -n 1)
    
    # 如果没找到自动生成的包，但找到了 .app，则手动打包
    if [ -z "$UPDATE_FILE" ] && [ -d "$BUNDLE_DIR/$APP_NAME" ]; then
        echo "⚠️ 未自动生成 updater 包，开始手动打包..."
        
        # 切换到目录进行打包，避免包含长路径
        cd "$BUNDLE_DIR"
        TAR_NAME="wlzj-match-downloader_${VERSION}_aarch64.app.tar.gz"
        tar -czf "$TAR_NAME" "$APP_NAME"
        
        UPDATE_FILE="$BUNDLE_DIR/$TAR_NAME"
        echo "📦 手动打包完成: $UPDATE_FILE"
        
        # 使用 Tauri CLI 签名
        echo "✍️ 正在签名..."
        # 显式指定私钥文件路径，并提供密码
        npm run tauri signer sign -k "$HOME/.tauri/wlzj-match-downloader.key" --password "121212" "$TAR_NAME"
        
        # 回到原目录
        cd - > /dev/null
    fi

    SIG_FILE="${UPDATE_FILE}.sig"
    
    if [ -n "$UPDATE_FILE" ] && [ -f "$UPDATE_FILE" ] && [ -f "$SIG_FILE" ]; then
        echo "📦 更新包路径: $UPDATE_FILE"
        echo "🔑 签名路径: $SIG_FILE"
        
        echo ""
        echo "📝 请执行以下 SQL 更新数据库 (ARM64):"
        echo "----------------------------------------"
        SIG=$(cat "$SIG_FILE")
        # 提取文件名
        FILENAME=$(basename "$UPDATE_FILE")
        echo "INSERT INTO fa_app_versions (app_name, version, platform, download_url, signature, release_notes, publish_date, status) VALUES ('wlzj-match-downloader', '$VERSION', 'darwin-aarch64', 'https://job3.posedu.cn/downloads/$FILENAME', '$SIG', '更新说明...', NOW(), 1);"
        echo "----------------------------------------"
    else
        echo "❌ 未找到更新包或签名文件"
        echo "目录内容: $BUNDLE_DIR"
        ls -la "$BUNDLE_DIR"
    fi
else
    echo "❌ 构建失败"
fi
