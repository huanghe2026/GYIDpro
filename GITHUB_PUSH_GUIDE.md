# GitHub 首次推送指南

## 当前仓库状态

- **分支**: main
- **提交历史**: 2 个 commit
  - `a7ec322` initial commit - GeoYuan project (闭源专有许可证)
  - `52f36d3` pre-push cleanup: remove temp scripts, parameterize server IP
- **追踪文件**: 208 个
- **仓库体积**: ~2.3MB
- **凭据助手**: Git Credential Manager (已配置)

## 推送步骤

### 1. 在 GitHub 上创建仓库

1. 登录 https://github.com
2. 点击右上角 **+** → **New repository**
3. 填写仓库名（如 `GYID` 或 `geoyuan`）
4. 选择 **Private**（闭源项目）
5. **不要**勾选 "Add a README" / ".gitignore" / "license"（项目已有这些文件）
6. 点击 **Create repository**

### 2. 添加远程地址

将 `<你的用户名>` 替换为你的 GitHub 用户名，`<仓库名>` 替换为第 1 步创建的仓库名：

```bash
git remote add origin https://github.com/<你的用户名>/<仓库名>.git
```

### 3. 推送

```bash
git push -u origin main
```

首次推送时，浏览器会自动弹出 GitHub 登录页面。登录后凭据会被 GCM 自动保存，后续推送无需重复登录。

## 推送后验证

```bash
# 查看远程状态
git remote -v

# 查看远程分支
git branch -r
```

## 注意事项

- 仓库为 **Private**，只有你能看到代码
- `.env` 文件（含高德 API Key）未被追踪，不会上传
- 服务器 IP 已从所有追踪文件中移除，替换为 `<SERVER_IP>` 占位符
- 部署脚本改为通过 `DEPLOY_SERVER` 环境变量或 `--server` 参数传入服务器地址
- 9 个临时 Python 脚本已从 Git 中移除（本地文件仍保留）
