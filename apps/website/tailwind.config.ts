import sharedConfig from '../../tailwind.config.js'

const baseConfig = sharedConfig as {
  darkMode?: unknown
  theme?: Record<string, unknown>
  plugins?: unknown[]
}

export default {
  darkMode: baseConfig.darkMode,
  content: [
    './app.vue',
    './components/**/*.{vue,js,ts}',
    './composables/**/*.{js,ts}',
    './content-or-copy/**/*.{js,ts}',
    './layouts/**/*.{vue,js,ts}',
    './pages/**/*.{vue,js,ts}',
    '../../packages/ui/src/**/*.{vue,js,ts}',
  ],
  theme: baseConfig.theme,
  plugins: baseConfig.plugins,
}
