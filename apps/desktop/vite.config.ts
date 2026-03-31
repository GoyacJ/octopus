import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

export default defineConfig({
  plugins: [vue()],
  server: {
    host: '127.0.0.1',
    port: 15420,
    strictPort: true,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@octopus/schema': path.resolve(__dirname, '../../packages/schema/src'),
      '@octopus/ui': path.resolve(__dirname, '../../packages/ui/src'),
    },
  },
})
