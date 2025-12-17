# 🔧 修复 "Resource not accessible by integration" 错误

## ❌ 错误信息
```
Error: Resource not accessible by integration
```

这个错误表示 GitHub Actions 的 `GITHUB_TOKEN` 没有足够的权限来创建 Release。

---

## ✅ 解决步骤

### 步骤 1：修改仓库的 Actions 权限设置

1. **访问仓库设置**
   - 打开你的 GitHub 仓库：`https://github.com/你的用户名/match-download`
   - 点击顶部的 **Settings** 标签

2. **进入 Actions 设置**
   - 在左侧菜单中，找到 **Actions** → **General**

3. **修改 Workflow 权限**
   - 滚动到页面底部，找到 **Workflow permissions** 部分
   - 选择 **"Read and write permissions"** （默认可能是 "Read repository contents and packages permissions"）
   - ✅ 勾选 **"Allow GitHub Actions to create and approve pull requests"**
   - 点击 **Save** 按钮

### 步骤 2：重新运行 Workflow

修改权限后，有两种方式重新运行：

**方式 A：重新触发现有的 workflow**
1. 进入 **Actions** 标签
2. 找到失败的 workflow run
3. 点击 **Re-run all jobs**

**方式 B：创建新的 tag**
```bash
# 删除旧 tag
git tag -d v1.0.1
git push origin :refs/tags/v1.0.1

# 创建新 tag
git tag v1.0.1
git push origin v1.0.1
```

---

## 📸 设置截图参考

在 **Settings** → **Actions** → **General** 页面底部，应该看到：

```
Workflow permissions
○ Read repository contents and packages permissions
● Read and write permissions  ← 选择这个

☑ Allow GitHub Actions to create and approve pull requests  ← 勾选这个
```

---

## 🎯 验证

设置完成后，workflow 应该能够：
1. ✅ 构建应用
2. ✅ 生成签名文件
3. ✅ 创建 GitHub Release
4. ✅ 上传所有构建产物到 Release

---

## 🔍 其他可能的原因

如果修改权限后仍然失败，检查：

1. **仓库是否是 Fork**
   - Fork 的仓库可能有额外的限制
   - 解决方案：在自己的仓库中重新创建项目

2. **组织仓库的权限**
   - 如果仓库属于组织，可能需要组织管理员修改权限
   - 进入组织的 **Settings** → **Actions** → **General**

3. **使用 Personal Access Token（高级）**
   - 创建一个 PAT with `repo` scope
   - 添加为仓库 Secret: `RELEASE_TOKEN`
   - 在 workflow 中使用：
     ```yaml
     env:
       GITHUB_TOKEN: ${{ secrets.RELEASE_TOKEN }}
     ```

---

## 📝 相关文档

- [GitHub Actions 权限文档](https://docs.github.com/en/actions/security-guides/automatic-token-authentication#permissions-for-the-github_token)
- [创建 Release 所需权限](https://docs.github.com/en/rest/releases/releases#create-a-release)
