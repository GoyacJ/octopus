import { useColorMode } from '@vueuse/core'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

import type { AppLocale } from '@/i18n'

export const useShellStore = defineStore('shell', () => {
  const locale = ref<AppLocale>('zh-CN')
  const colorMode = useColorMode({
    attribute: 'data-theme',
    initialValue: 'light',
    modes: {
      light: 'light',
      dark: 'dark',
    },
  })
  const selectedSurface = ref('Chat')

  const isDark = computed(() => colorMode.value === 'dark')

  const setLocale = (value: AppLocale) => {
    locale.value = value
  }

  const toggleTheme = () => {
    colorMode.value = colorMode.value === 'dark' ? 'light' : 'dark'
  }

  return {
    colorMode,
    isDark,
    locale,
    selectedSurface,
    setLocale,
    toggleTheme,
  }
})

