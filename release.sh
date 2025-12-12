#!/bin/bash

# GitHub Actions 快速发布脚本
# 用法: ./release.sh v1.0.0

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# 检查参数
if [ -z "$1" ]; then
    print_error "请提供版本号！"
    echo "用法: ./release.sh v1.0.0"
    exit 1
fi

VERSION=$1

# 验证版本号格式
if [[ ! $VERSION =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    print_error "版本号格式错误！应该是 vX.Y.Z 格式，例如: v1.0.0"
    exit 1
fi

print_info "准备发布版本: $VERSION"
echo ""

# 检查是否有未提交的更改
if [[ -n $(git status -s) ]]; then
    print_warning "检测到未提交的更改："
    git status -s
    echo ""
    read -p "是否继续？(y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "已取消发布"
        exit 0
    fi
fi

# 更新版本号
print_info "更新版本号到配置文件..."

# 更新 tauri.conf.json
VERSION_NUMBER=${VERSION#v}  # 移除 v 前缀
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION_NUMBER\"/" src-tauri/tauri.conf.json
else
    # Linux
    sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION_NUMBER\"/" src-tauri/tauri.conf.json
fi

# 更新 Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "s/^version = \".*\"/version = \"$VERSION_NUMBER\"/" src-tauri/Cargo.toml
else
    sed -i "s/^version = \".*\"/version = \"$VERSION_NUMBER\"/" src-tauri/Cargo.toml
fi

# 更新 package.json
if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION_NUMBER\",/" package.json
else
    sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION_NUMBER\",/" package.json
fi

print_success "版本号已更新"

# 提交更改
print_info "提交版本更新..."
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml package.json
git commit -m "chore: bump version to $VERSION" || print_warning "没有需要提交的更改"

# 创建 tag
print_info "创建 Git tag: $VERSION"
if git tag -a "$VERSION" -m "Release $VERSION"; then
    print_success "Tag 创建成功"
else
    print_error "Tag 创建失败，可能已存在"
    exit 1
fi

# 推送到远程
print_info "推送到 GitHub..."
echo ""
print_warning "即将执行以下操作："
echo "  1. 推送代码到 origin/main"
echo "  2. 推送 tag $VERSION"
echo "  3. 触发 GitHub Actions 自动构建"
echo ""
read -p "确认推送？(y/N) " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    # 获取当前分支
    CURRENT_BRANCH=$(git branch --show-current)
    
    # 推送代码
    git push origin "$CURRENT_BRANCH"
    print_success "代码已推送"
    
    # 推送 tag
    git push origin "$VERSION"
    print_success "Tag 已推送"
    
    echo ""
    print_success "🎉 发布流程已启动！"
    echo ""
    print_info "GitHub Actions 正在构建以下平台："
    echo "  • Windows x64"
    echo "  • macOS ARM64 (M1/M2/M3/M4)"
    echo "  • macOS x64 (Intel)"
    echo "  • Linux x64"
    echo ""
    print_info "查看构建进度："
    
    # 尝试获取 GitHub 仓库 URL
    REPO_URL=$(git config --get remote.origin.url | sed 's/\.git$//')
    if [[ $REPO_URL == git@github.com:* ]]; then
        REPO_URL="https://github.com/${REPO_URL#git@github.com:}"
    fi
    
    echo "  $REPO_URL/actions"
    echo ""
    print_info "构建完成后，在这里下载安装包："
    echo "  $REPO_URL/releases/tag/$VERSION"
    echo ""
    print_warning "预计构建时间: 15-25 分钟"
else
    print_info "已取消推送"
    print_warning "如需删除本地 tag，运行: git tag -d $VERSION"
    exit 0
fi
