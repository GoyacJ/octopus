const themeBootScript = `
(() => {
  const storageKey = 'octopus-website-theme'
  const root = document.documentElement
  const preference = localStorage.getItem(storageKey) || 'system'
  const resolved = preference === 'system'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : preference
  root.dataset.theme = resolved
  root.style.colorScheme = resolved
  root.classList.toggle('dark', resolved === 'dark')
})()
`.trim()

export default defineNuxtConfig({
  devtools: { enabled: false },
  ssr: true,
  modules: ['@nuxtjs/tailwindcss', '@nuxtjs/i18n'],
  css: ['~/assets/css/website.css'],
  build: {
    transpile: ['@octopus/ui'],
  },
  app: {
    head: {
      title: 'Octopus',
      htmlAttrs: {
        lang: 'zh-CN',
        'data-theme': 'light',
      },
      meta: [
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { name: 'theme-color', content: '#fbfaf8' },
      ],
      link: [
        { rel: 'icon', type: 'image/png', href: '/brand/logo.png' },
        { rel: 'apple-touch-icon', href: '/brand/logo.png' },
      ],
      script: [
        {
          id: 'octopus-theme-boot',
          innerHTML: themeBootScript,
          tagPosition: 'head',
        },
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
