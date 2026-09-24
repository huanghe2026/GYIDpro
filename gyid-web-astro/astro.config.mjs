// @ts-check
import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';

// GyID 官网（gyid.geoyuan.com）
//
// 双语约定与现有 gyid.geoyuan.com 一致：**英文默认**（URL 不带前缀），中文在 `/zh/` 下。
// 主题（配色/字体/发光/玻璃面板）对齐 diliy.cn，见 tailwind.config.mjs + src/styles/global.css。
//
// 验证器走同源：`https://gyid.geoyuan.com/v1/*` 与 `wss://gyid.geoyuan.com/v1/challenge`
// 由 nginx 反代到本机 trip-server（127.0.0.1:8080），因此前端无需配置绝对后端地址。
// 本地开发/预览时把验证方请求代理到本机 trip-server。
// 生产环境由 nginx 完成同样的反代；此配置不影响构建产物。
const verifierProxy = {
  '/v1': { target: 'http://127.0.0.1:8080', changeOrigin: true, ws: true },
  '/.well-known': { target: 'http://127.0.0.1:8080', changeOrigin: true },
};

// 同一套源码支持两种部署形态（互不干扰，可同时上线）：
//   BASE_PATH 未设置 / `/`  → 站点根   https://gyid.geoyuan.com/
//   BASE_PATH=/verify/      → 子目录   https://gyid.geoyuan.com/verify/
// OUT_DIR 控制产物目录，便于一次构建两个变体而不互相覆盖。
const basePath = process.env.BASE_PATH?.trim() || '/';
const outDir = process.env.OUT_DIR?.trim() || './dist';

export default defineConfig({
  site: 'https://gyid.geoyuan.com',
  base: basePath,
  outDir,
  compressHTML: true,
  trailingSlash: 'ignore',
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'zh'],
    routing: {
      prefixDefaultLocale: false,
    },
  },
  integrations: [tailwind({ applyBaseStyles: false })],
  prefetch: {
    prefetchAll: true,
    defaultStrategy: 'hover',
  },
  build: {
    // 控制台用原生 ES module + WASM（top-level await），需要现代目标
    inlineStylesheets: 'auto',
  },
  vite: {
    envPrefix: ['PUBLIC_', 'VITE_'],
    // 仅 `astro dev` 生效（`astro preview` 不接受该键）。
    // 预览与生产都由反向代理承担相同的转发：生产为 nginx，见部署说明。
    server: { proxy: verifierProxy },
  },
});
