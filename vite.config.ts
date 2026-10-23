import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri 在 dev 模式下把 devUrl 指向本机端口；端口必须与 tauri.conf.json 的
// build.devUrl 完全一致，否则前端产物加载不到。
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Rust 侧目录交给 cargo 自行处理，避免 Vite 重复 watch 造成 CPU 空转。
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // Tauri 使用系统 WebView2，支持现代语法；不做过度降级以保住体积。
    target: "es2021",
    sourcemap: false,
    outDir: "dist",
    emptyOutDir: true,
  },
});
