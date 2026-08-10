import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri 用系统 WebView2 渲染；dev 走 5173，build 输出到 dist 供 Tauri 打包
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: 'es2021',
    outDir: 'dist',
    sourcemap: false,
    assetsInlineLimit: 0,
  },
})
