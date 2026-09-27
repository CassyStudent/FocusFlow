import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    strictPort: true,
  },
  test: {
    environment: 'jsdom',
    globals: true,
  },
})
