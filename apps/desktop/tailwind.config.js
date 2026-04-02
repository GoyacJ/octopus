import rootConfig from '../../tailwind.config.js'

export default {
  ...rootConfig,
  content: [
    './index.html',
    './src/**/*.{vue,js,ts,jsx,tsx}',
    '../../packages/ui/src/**/*.{vue,js,ts,jsx,tsx}',
  ],
}
