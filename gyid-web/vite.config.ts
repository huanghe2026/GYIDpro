import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";

// Vite 配置：SolidJS + WASM 适配
// base: "./" 让产物可从任意路径加载（便于嵌入 verifier 静态目录）
export default defineConfig({
  base: "./",
  plugins: [solidPlugin()],
  server: {
    port: 5173,
  },
  define: {
    "import.meta.env.VITE_VERIFIER_URL": JSON.stringify(
      process.env.VITE_VERIFIER_URL ?? "http://localhost:8080",
    ),
  },
  // WASM 不能被预打包（依赖 import.meta.url 解析 .wasm 资源）
  optimizeDeps: {
    exclude: ["gyid-wasm"],
  },
  build: {
    // 需要 top-level await + 原生 bigint 支持
    target: "esnext",
  },
});
