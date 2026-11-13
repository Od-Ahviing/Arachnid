import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import tailwind from '@tailwindcss/vite'

export default defineConfig({
  plugins: [vue(), tailwind()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: '127.0.0.1',
    watch: { ignored: ['**/target/**', '**/src-tauri/**', '**/crates/**'] },
  },
  build: { target: 'chrome111' },
  test: { environment: 'jsdom', include: ['tests/frontend/**/*.test.ts'] },
})
