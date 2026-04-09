import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import path from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@octopus/assets/pets': path.resolve(__dirname, '../../packages/assets/pets/index.ts'),
      '@octopus/schema': path.resolve(__dirname, '../../packages/schema/src'),
      '@octopus/ui': path.resolve(__dirname, '../../packages/ui/src'),
    },
  },
  test: {
    environment: 'node',
    include: ['test/**/*.test.ts'],
  },
})
