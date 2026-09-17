# GYID WebButton 部署检查清单

## 部署前检查 ✅

### 1. 文件完整性
- [ ] `index.html` - 主入口页面
- [ ] `embed.html` - iframe 嵌入版本
- [ ] `README.md` - 使用文档
- [ ] `css/gyid-webbutton.css` - 样式文件
- [ ] `js/gyid-webbutton.js` - 核心脚本
- [ ] `js/qrcode.min.js` - 二维码库

### 2. 服务器配置
- [ ] Nginx 配置已更新
- [ ] HTTPS 证书有效
- [ ] CORS 配置正确（如需要）
- [ ] MIME 类型已配置

### 3. CDN 配置（如使用）
- [ ] Cloudflare /阿里云 CDN 已绑定
- [ ] 缓存规则已设置
- [ ] 压缩已启用 (gzip/brotli)

## 部署命令

### 手动部署
```bash
# Linux/Mac
chmod +x deploy.sh
./deploy.sh production

# Windows
deploy.bat
```

### 自动部署 (CI/CD)
```bash
# GitHub Actions 已配置
# 推送到 main 分支自动触发
git push origin main
```

## 部署后验证

### 1. 基础验证
```bash
# 检查文件是否存在
curl -I https://geoyuan.com/tools/webbutton/z/index.html
curl -I https://geoyuan.com/tools/webbutton/z/js/gyid-webbutton.js
curl -I https://geoyuan.com/tools/webbutton/z/css/gyid-webbutton.css

# 检查响应状态
# 应返回 200 OK
```

### 2. 功能验证
- [ ] 打开 `https://geoyuan.com/tools/webbutton/z/index.html`
- [ ] 右下角显示 GEOYUAN 按钮
- [ ] 点击展开面板
- [ ] 地理位置显示正确
- [ ] 指纹采集正常
- [ ] 权重验证 ≥60%

### 3. 嵌入验证
```html
<!-- 测试 iframe 嵌入 -->
<iframe src="https://geoyuan.com/tools/webbutton/z/embed.html"
        width="340" height="480"
        frameborder="0"
        allow="geolocation">
</iframe>
```

## 回滚操作

如部署出现问题，执行以下命令回滚：

```bash
# Linux
cd /var/www/geoyuan.com/tools/webbutton
rm -rf z && mv z.backup z

# 清除 CDN 缓存
curl -X POST "https://api.cloudflare.com/..." \
  -d '{"purge_everything":true}'
```

## Nginx 配置参考

```nginx
location /tools/webbutton/z {
    alias /var/www/geoyuan.com/tools/webbutton/z;
    
    # 启用 gzip
    gzip on;
    gzip_types text/css application/javascript;
    
    # 缓存策略
    location ~* \.(js|css)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
    
    location ~* \.(html|json)$ {
        expires -1;
        add_header Cache-Control "no-cache, no-store";
    }
}
```

## 联系人

- 技术支持: support@geoyuan.com
- 问题反馈: https://github.com/geoyuan/gyid/issues
