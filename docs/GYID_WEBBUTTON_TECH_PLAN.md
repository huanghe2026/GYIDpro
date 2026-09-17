# GYID WebButton 工具 - 技术方案

## 1. 项目概述

### 1.1 目标
基于 GEOYUAN 网站弹窗设计规范，开发一个可嵌入任意网页的 GYID 生成工具（WebButton）。

### 1.2 设计规范来源
参考 `http://daixie.uno/2026fakao/` 页面的 GEOYUAN 按钮弹窗设计。

---

## 2. 设计规格分析

### 2.1 GEOYUAN 弹窗设计参数

| 参数 | 值 |
|------|-----|
| **位置** | 固定右下角 (bottom: 24px, right: 24px) |
| **z-index** | 99999 |
| **字体** | "Noto Sans SC", -apple-system, BlinkMacSystemFont |
| **面板宽度** | 340px |
| **背景** | #FFFFFF (白色) |
| **圆角** | 16px |
| **阴影** | rgba(13, 27, 62, 0.2) 0px 12px 48px |
| **动画时长** | 0.4s cubic-bezier(0.22, 1, 0.36, 1) |

### 2.2 Tab 样式
- 深蓝色渐变背景: `linear-gradient(135deg, rgb(13, 27, 62) → rgb(26, 58, 107))`
- 圆角: 24px
- 绿色脉冲动画点
- GEOYUAN 徽章: 金色渐变背景

### 2.3 Header 样式
- 分隔线: 1px solid #E5E7EB
- 标题: 14px, font-weight: 600
- 关闭按钮: 灰色，带 hover 效果

### 2.4 Body 样式
- 描述文字: 13px, #6B7280
- iframe 内容区: 100% 宽, 200px 高, 8px 圆角

---

## 3. GYID 生成页面功能设计

### 3.1 必要的控件清单

#### 3.1.1 硬件指纹采集
- **自动采集**: 通过 Web API 采集浏览器指纹
  - User Agent
  - 屏幕分辨率
  - 颜色深度
  - 时区
  - 语言
  - 插件列表
  - Canvas 指纹
  - WebGL 指纹

#### 3.1.2 地理位置采集
- **GPS 定位** (需用户授权)
  - 精度等级选择: L1(城市) / L2(区域) / L3(GPS+WiFi)
- **IP 定位** (降级方案)
- **手动输入** (GeoPrecisionLevel::Manual)

#### 3.1.3 头像采集
- **头像上传**: 支持拖拽/点击上传
- **预览**: 128x128 缩略图
- **验证**: Base64 格式，支持 PNG/JPG/GIF/WebP

#### 3.1.4 设备关联 (可选)
- **多设备模式**: 主设备/从设备切换
- **设备树形结构显示**

#### 3.1.5 链上锚定选项
- **区块链选择**:
  - Polygon PoS
  - Aptos
  - 仅本地 (离线模式)
- **签名方式**:
  - EIP-155 (Polygon)
  - ED25519 (Aptos)

#### 3.1.6 高级选项
- **H3 六边形网格**:
  - 分辨率选择: 0-15
  - 自动计算
- **时间戳精度**:
  - UTC毫秒 + 16bit随机数

---

## 4. 页面布局设计

### 4.1 弹窗结构

```
┌──────────────────────────────────────┐
│  📍 您的位置：杭州市 电信  [×]      │ ← Header
├──────────────────────────────────────┤
│  探索去中心化身份（DID），            │ ← Description
│  掌握您的数字主权。                   │
├──────────────────────────────────────┤
│  ┌────────────────────────────────┐  │
│  │                                │  │
│  │     GYID 生成器界面           │  │ ← Main Content
│  │                                │  │
│  └────────────────────────────────┘  │
├──────────────────────────────────────┤
│  [生成 GYID]                         │ ← Action Button
└──────────────────────────────────────┘
```

### 4.2 GYID 生成器界面

```
┌──────────────────────────────────────┐
│  🔐 硬件指纹                        │
│  ├─ UA: Windows NT 10.0...     [✓] │
│  ├─ 屏幕: 1920x1080             [✓] │
│  ├─ 时区: Asia/Shanghai         [✓] │
│  └─ Canvas 指纹: 已采集         [✓] │
├──────────────────────────────────────┤
│  📍 地理位置                        │
│  ○ 仅城市 (L1)                     │
│  ○ 区域精度 (L2)                   │
│  ● GPS定位 (L3)            [定位中] │
│  位置: 杭州市, 中国                 │
├──────────────────────────────────────┤
│  👤 头像                            │
│  ┌──────┐  [上传头像]              │
│  │ 预览 │                          │
│  └──────┘                          │
├──────────────────────────────────────┤
│  ⛓️ 链上锚定                        │
│  ○ Polygon PoS                     │
│  ○ Aptos                           │
│  ● 仅本地存储                       │
├──────────────────────────────────────┤
│  [━━━━━━━━ 生成 GYID ━━━━━━━━]     │
└──────────────────────────────────────┘
```

---

## 5. 技术实现方案

### 5.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                     宿主网页 (任意网站)                      │
│  <script src="gyid-webbutton.js"></script>                  │
│  <div id="gyid-widget-root"></div>                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    gyid-webbutton.js                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Widget Shell │  │ Fingerprint │  │ GeoService  │        │
│  │  (UI框架)    │  │  (指纹采集)  │  │ (位置服务)   │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │  GyIdCore   │  │ QRGenerator │  │  Storage    │        │
│  │ (核心生成)   │  │  (二维码)   │  │  (本地存储)  │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 核心模块

#### 5.2.1 Widget Shell (UI 框架)
- 纯原生 JavaScript 实现
- 无外部依赖
- CSS 变量支持主题定制
- 响应式设计

#### 5.2.2 Fingerprint Module (指纹采集)
- Web Fingerprinting API
- Canvas 指纹
- WebGL 指纹
- Audio 指纹

#### 5.2.3 Geo Module (地理位置)
- Geolocation API
- IP 定位降级
- WiFi MLS API (可选)

#### 5.2.4 GyIdCore (核心生成器)
- WASM 编译的 GyID Core
- Base58 编码
- 权重验证

#### 5.2.5 QRGenerator (二维码)
- QRCode.js 内联版
- SVG/Canvas 渲染

#### 5.2.6 Storage (存储)
- IndexedDB
- LocalStorage (降级)

### 5.3 部署方式

#### 方式一: CDN 引入 (推荐)
```html
<script src="https://geoyuan.com/tools/webbutton/z/gyid-webbutton.min.js"></script>
<link href="https://geoyuan.com/tools/webbutton/z/gyid-webbutton.min.css" rel="stylesheet">
<div id="gyid-widget-root"></div>
<script>
  GyIDWebButton.init({
    position: 'bottom-right', // 'bottom-right' | 'bottom-left'
    theme: 'auto', // 'auto' | 'light' | 'dark'
    apiEndpoint: 'https://api.geoyuan.com'
  });
</script>
```

#### 方式二: NPM 包引入
```bash
npm install @geoyuan/webbutton
```
```javascript
import { GyIDWebButton } from '@geoyuan/webbutton';
GyIDWebButton.init('#app');
```

#### 方式三: iframe 嵌入
```html
<iframe
  src="https://geoyuan.com/tools/webbutton/z/embed.html"
  width="340"
  height="500"
  style="border: none; border-radius: 16px; box-shadow: 0 12px 48px rgba(13, 27, 62, 0.2);"
  allow="geolocation"
></iframe>
```

---

## 6. 文件结构

```
/tools/webbutton/z/
├── index.html              # 主入口页面
├── embed.html              # iframe 嵌入版本
├── css/
│   ├── gyid-webbutton.css  # 主样式
│   ├── theme-light.css     # 浅色主题
│   └── theme-dark.css      # 深色主题
├── js/
│   ├── gyid-webbutton.js   # 主入口
│   ├── widget-shell.js     # UI 框架
│   ├── fingerprint.js      # 指纹采集
│   ├── geo-service.js      # 位置服务
│   ├── gyid-core.wasm      # 核心生成器 (WASM)
│   ├── qr-generator.js     # 二维码生成
│   └── storage.js          # 本地存储
├── assets/
│   ├── icons/              # 图标资源
│   └── images/             # 图片资源
├── manifest.json           # Web App Manifest
├── sw.js                   # Service Worker (离线支持)
└── README.md               # 使用文档
```

---

## 7. API 接口设计

### 7.1 生成 GYID

**请求**
```http
POST /api/v1/gyid/generate
Content-Type: application/json

{
  "fingerprint": {
    "userAgent": "...",
    "screen": "1920x1080",
    "timezone": "Asia/Shanghai",
    "canvasHash": "...",
    "webglHash": "..."
  },
  "geo": {
    "precision": "L3",
    "latitude": 30.2741,
    "longitude": 120.1551,
    "city": "杭州市",
    "country": "中国",
    "h3Cell": "8a2a1072c63fff"
  },
  "avatar": {
    "hash": "sha256:...",
    "base64": "..."
  },
  "chain": "polygon|aptos|local",
  "timestamp": 1743932400000
}
```

**响应**
```json
{
  "success": true,
  "data": {
    "gyid": "GY1A2B3C4D5E6F7G8H9J...",
    "createdAt": "2026-04-06T02:30:00.000Z",
    "qrCode": "data:image/png;base64,...",
    "chainTx": "0x...",
    "weights": {
      "fingerprint": 0.4,
      "geo": 0.35,
      "avatar": 0.15,
      "timestamp": 0.1
    }
  }
}
```

---

## 8. 安全考虑

### 8.1 CSP 策略
```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: blob:;
  connect-src 'self' https://api.geoyuan.com https://ip-api.com;
  frame-ancestors *;
```

### 8.2 隐私保护
- 指纹数据仅本地处理，不上传
- 地理位置需用户明确授权
- 头像数据哈希后存储
- 支持隐私模式

### 8.3 XSS 防护
- 所有用户输入转义
- CSP 头保护
- iframe sandbox 属性

---

## 9. 性能优化

### 9.1 加载策略
- 懒加载: 仅在用户首次交互时加载核心模块
- 代码分割: 按需加载 WASM 和 QR 模块
- 预加载: `<link rel="preload">` 关键资源

### 9.2 WASM 优化
- 首次加载后缓存到 IndexedDB
- 使用 SharedArrayBuffer (如支持)
- 流式编译

### 9.3 缓存策略
- Service Worker 缓存静态资源
- ETag 协商缓存
- 版本控制 URL

---

## 10. 浏览器兼容性

| 功能 | Chrome | Firefox | Safari | Edge |
|------|--------|---------|--------|------|
| WebAssembly | ✅ 57+ | ✅ 52+ | ✅ 11+ | ✅ 16+ |
| Geolocation | ✅ | ✅ | ✅ | ✅ |
| IndexedDB | ✅ | ✅ | ✅ | ✅ |
| Service Worker | ✅ 40+ | ✅ 44+ | ✅ 11.1+ | ✅ 17+ |
| WebGL | ✅ | ✅ | ✅ | ✅ |
| Canvas | ✅ | ✅ | ✅ | ✅ |

**最低支持**: Chrome 60, Firefox 55, Safari 11, Edge 79

---

## 11. 安装集成示例

### 11.1 WordPress
```php
// functions.php
function add_gyid_webbutton() {
    echo '<script src="https://geoyuan.com/tools/webbutton/z/gyid-webbutton.min.js"></script>';
    echo '<div id="gyid-widget-root"></div>';
    echo '<script>GyIDWebButton.init({position: "bottom-right"});</script>';
}
add_action('wp_footer', 'add_gyid_webbutton');
```

### 11.2 Shopify
```liquid
<!-- theme.liquid -->
{% section 'gyid-webbutton' %}
```

### 11.3 Wix
```javascript
// Wix Editor
import { GyIDWebButton } from '@geoyuan/webbutton';
$w.onReady(() => {
  GyIDWebButton.init('#footer');
});
```

---

## 12. 定制化选项

```javascript
GyIDWebButton.init({
  // 位置
  position: 'bottom-right',

  // 主题
  theme: 'auto',

  // 语言
  locale: 'zh-CN',

  // API 端点
  apiEndpoint: 'https://api.geoyuan.com',

  // 功能开关
  features: {
    fingerprint: true,
    geo: true,
    avatar: true,
    chain: true,
    qrExport: true
  },

  // 地理位置权限
  geoPermission: 'prompt', // 'prompt' | 'deny' | 'grant'

  // 链上锚定
  defaultChain: 'local',

  // 回调
  onGyIdGenerated: (gyid) => {
    console.log('GYID 生成成功:', gyid);
  },

  // 样式覆盖
  customStyles: {
    '--gyid-primary': '#1a3a6b',
    '--gyid-radius': '16px'
  }
});
```

---

## 13. 开发路线图

### Phase 1: MVP (1-2周)
- [ ] Widget Shell 开发
- [ ] 基础指纹采集
- [ ] 地理位置服务
- [ ] GYID 生成核心
- [ ] 基本 UI

### Phase 2: 完善 (2-3周)
- [ ] 头像上传
- [ ] 二维码生成
- [ ] 导出功能
- [ ] 主题定制

### Phase 3: 高级 (3-4周)
- [ ] 链上锚定
- [ ] 多语言支持
- [ ] 离线支持
- [ ] 性能优化

### Phase 4: 发布
- [ ] CDN 部署
- [ ] NPM 包发布
- [ ] 文档完善
- [ ] 监控告警

---

## 14. 联系方式

- **官方网站**: https://geoyuan.com
- **技术支持**: support@geoyuan.com
- **GitHub Issues**: https://github.com/geoyuan/gyid-webbutton
