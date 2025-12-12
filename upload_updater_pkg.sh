#!/bin/bash
# 上传 Updater 包到服务器

LOCAL_FILE="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/wlzj-match-downloader_0.1.0_aarch64.app.tar.gz"
FILENAME=$(basename "$LOCAL_FILE")
REMOTE_DIR="/www/wwwroot/job3.posedu.cn/downloads"

echo "📦 准备上传: $FILENAME"

upload_to() {
    IP=$1
    USER="yundai"
    PASS="pwd.135246.weiLai.0727"
    ROOT_PASS="=8nheyJ):LLfjLdn49iE" # using server 1 pass as example
    if [ "$IP" == "39.100.97.206" ]; then
        ROOT_PASS="bT%sFN,L>gL3?-rk#e!>"
    fi
    
    echo "➡️ 上传到 $IP ..."
    
    # 创建目录 (尝试 sudo/su)
    sshpass -p "$PASS" ssh -o StrictHostKeyChecking=no $USER@$IP "test -d $REMOTE_DIR || echo \"$ROOT_PASS\" | su -c 'mkdir -p $REMOTE_DIR && chmod 777 $REMOTE_DIR'"

    # 上传
    sshpass -p "$PASS" scp -o StrictHostKeyChecking=no "$LOCAL_FILE" "$USER@$IP:$REMOTE_DIR/$FILENAME"
    
    if [ $? -eq 0 ]; then
        echo "✅ 上传成功"
    else
        echo "❌ 上传失败"
    fi
}

upload_to "39.99.222.212"
upload_to "39.100.97.206"
