# GYID WebButton

基于 GEOYUAN 网站弹窗设计规范的可嵌入 GYID 生成工具。

## 功能特性

- **硬件指纹采集**: 自动采集浏览器指纹信息
- **地理位置定位**: 支持 L1/L2/L3 三种精度等级
- **头像上传**: 支持头像上传和预览
- **链上锚定**: 支持 Polygon、Aptos 或仅本地存储
- **二维码生成**: 生成的 GYID 可导出为二维码
- **权重验证**: 实时显示各维度的权重占比

## 设计规范

参考 `http://daixie.uno/2026fakao/` 页面的 GEOYUAN 按钮弹窗设计：

| 参数 | 值 |
|------|-----|
| 位置 | 固定右下角 (bottom: 24px, right: 24px) |
| z-index | 99999 |
| 面板宽度 | 340px |
| 背景 | #FFFFFF |
| 圆角 | 16px |
| 阴影 | rgba(13, 27, 62, 0.2) |

## 快速开始

### 方式一: CDN 引入

```html
<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="https://geoyuan.com/tools/webbutton/z/css/gyid-webbutton.css">
</head>
<body>
  <div id="gyid-widget-root"></div>
  
  <script src="https://geoyuan.com/tools/webbutton/z/js/qrcode.min.js"></script>
  <script src="https://geoyuan.com/tools/webbutton/z/js/gyid-webbutton.js"></script>
  <script>
    document.addEventListener('DOMContentLoaded', () => {
      GyIDWebButton.init({
        position: 'bottom-right',
        onGyIdGenerated: (result) => {
          console.log('GYID:', result.gyid);
        }
      });
    });
  </script>
</body>
</html>
```

### 方式二: iframe 嵌入

```html
<iframe
  src="https://geoyuan.com/tools/webbutton/z/embed.html"
  width="340"
  height="500"
  style="border: none; border-radius: 16px; box-shadow: 0 12px 48px rgba(13, 27, 62, 0.2);"
  allow="geolocation"
></iframe>
```

### 方式三: NPM 包

```bash
npm install @geoyuan/webbutton
```

```javascript
import { GyIDWebButton } from '@geoyuan/webbutton';

GyIDWebButton.init({
  position: 'bottom-right'
});
```

## 配置选项

```javascript
GyIDWebButton.init({
  // 位置
  position: 'bottom-right',  // 'bottom-right' | 'bottom-left'

  // 主题
  theme: 'auto',  // 'auto' | 'light' | 'dark'

  // 语言
  locale: 'zh-CN',

  // 功能开关
  features: {
    fingerprint: true,   // 硬件指纹
    geo: true,           // 地理位置
    avatar: true,        // 头像上传
    chain: true,         // 链上锚定
    qrExport: true       // 二维码导出
  },

  // 默认链上锚定
  defaultChain: 'local',  // 'local' | 'polygon' | 'aptos'

  // 回调函数
  onGyIdGenerated: (result) => {
    console.log('GYID:', result.gyid);
    console.log('权重:', result.weights);
  }
});
```

## iframe 通信

当使用 iframe 嵌入时，可以通过 postMessage 进行通信：

```javascript
// 监听 GYID 生成事件
window.addEventListener('message', (event) => {
  if (event.data.type === 'GYID_GENERATED') {
    console.log('GYID:', event.data.data.gyid);
  }
});

// 向 iframe 发送消息
iframe.contentWindow.postMessage({
  type: 'SET_LOCATION',
  data: { city: '杭州市', country: '中国' }
}, '*');

iframe.contentWindow.postMessage({
  type: 'CLOSE'
}, '*');
```

## 浏览器兼容性

- Chrome 60+
- Firefox 55+
- Safari 11+
- Edge 79+

## 文件结构

```
tools/webbutton/z/
├── index.html           # 主入口页面
├── embed.html           # iframe 嵌入版本
├── css/
│   └── gyid-webbutton.css  # 样式文件
├── js/
│   ├── gyid-webbutton.js   # 主脚本
│   └── qrcode.min.js       # 二维码生成库
└── README.md
```

## 部署

### GitHub Pages

1. 将文件推送到 `gh-pages` 分支
2. 访问 `https://yourusername.github.io/repo/tools/webbutton/z/`

### 自定义域名

1. 将文件上传到 Web 服务器
2. 配置 nginx:

```nginx
location /tools/webbutton/z/ {
  alias /var/www/gyid-webbutton/;
  index index.html;
  try_files $uri $uri/ =404;
}
```

## License

MIT License
