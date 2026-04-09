import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

const hostRuntime = process.env.VITE_HOST_RUNTIME
const defaultPort = hostRuntime === 'browser' ? 15421 : 15420
const resolvedPort = Number(process.env.VITE_UI_PORT ?? defaultPort)

export default defineConfig({
  plugins: [vue()],
  server: {
    host: '127.0.0.1',
    port: resolvedPort,
    strictPort: true,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@octopus/assets/pets': path.resolve(__dirname, '../../packages/assets/pets/index.ts'),
      '@octopus/schema': path.resolve(__dirname, '../../packages/schema/src'),
      '@octopus/ui': path.resolve(__dirname, '../../packages/ui/src'),
    },
  },
})
