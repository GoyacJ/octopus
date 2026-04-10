export default defineNuxtConfig({
  compatibilityDate: '2026-04-10',
  devtools: { enabled: false },
  ssr: true,
  modules: ['@nuxtjs/tailwindcss', '@nuxtjs/i18n', '@nuxtjs/color-mode'],
  css: ['~/assets/css/website.css'],
  build: {
    transpile: ['@octopus/ui', 'lucide-vue-next'],
  },
  colorMode: {
    classSuffix: '',
    preference: 'system',
    fallback: 'light',
  },
  app: {
    head: {
      title: 'Octopus',
      htmlAttrs: {
        lang: 'zh-CN',
      },
      meta: [
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { name: 'theme-color', content: '#f97316' },
      ],
      link: [
        { rel: 'icon', type: 'image/png', href: '/brand/logo.png' },
        { rel: 'apple-touch-icon', href: '/brand/logo.png' },
      ],
    },
  },
  nitro: {
    prerender: {
      crawlLinks: true,
      routes: [
        '/',
        '/product',
        '/scenarios',
        '/about',
        '/book-demo',
        '/en',
        '/en/product',
        '/en/scenarios',
        '/en/about',
        '/en/book-demo',
      ],
    },
  },
  routeRules: {
    '/': { prerender: true },
    '/product': { prerender: true },
    '/scenarios': { prerender: true },
    '/about': { prerender: true },
    '/book-demo': { prerender: true },
    '/en': { prerender: true },
    '/en/product': { prerender: true },
    '/en/scenarios': { prerender: true },
    '/en/about': { prerender: true },
    '/en/book-demo': { prerender: true },
  },
  runtimeConfig: {
    public: {
      demoUrl: process.env.NUXT_PUBLIC_DEMO_URL ?? 'mailto:hello@octopus.run?subject=Book%20an%20Octopus%20demo',
      siteUrl: process.env.NUXT_PUBLIC_SITE_URL ?? 'https://octopus.run',
      githubRepoOwner: process.env.NUXT_PUBLIC_GITHUB_REPO_OWNER ?? 'GoyacJ',
      githubRepoName: process.env.NUXT_PUBLIC_GITHUB_REPO_NAME ?? 'octopus',
      githubApiBase: process.env.NUXT_PUBLIC_GITHUB_API_BASE ?? 'https://api.github.com',
    },
  },
  i18n: {
    restructureDir: '.',
    strategy: 'prefix_except_default',
    defaultLocale: 'zh-CN',
    detectBrowserLanguage: false,
    langDir: 'locales',
    vueI18n: './i18n.config.ts',
    locales: [
      { code: 'zh-CN', language: 'zh-CN', name: '简体中文', file: 'zh-CN.json' },
      { code: 'en', language: 'en-US', name: 'English', file: 'en-US.json' },
    ],
    baseUrl: process.env.NUXT_PUBLIC_SITE_URL ?? 'https://octopus.run',
    customRoutes: 'config',
    pages: {
      index: {
        'zh-CN': '/',
        en: '/',
      },
      product: {
        'zh-CN': '/product',
        en: '/product',
      },
      scenarios: {
        'zh-CN': '/scenarios',
        en: '/scenarios',
      },
      about: {
        'zh-CN': '/about',
        en: '/about',
      },
      'book-demo': {
        'zh-CN': '/book-demo',
        en: '/book-demo',
      },
    },
  },
  tailwindcss: {
    configPath: 'tailwind.config.ts',
    viewer: false,
  },
})
