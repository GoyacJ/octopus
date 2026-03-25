import { createI18n } from 'vue-i18n'

export const messages = {
  'zh-CN': {
    app: {
      subtitle: 'Phase 1 控制面壳，用于承接统一对象模型、主题与多语言基线。',
      locale: '中文',
      theme: '主题',
      light: '浅色',
      dark: '深色',
    },
  },
  'en-US': {
    app: {
      subtitle: 'Phase 1 control shell for the unified object model, theme, and i18n baseline.',
      locale: 'English',
      theme: 'Theme',
      light: 'Light',
      dark: 'Dark',
    },
  },
} as const

export type AppLocale = keyof typeof messages

export const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'en-US',
  messages,
})

