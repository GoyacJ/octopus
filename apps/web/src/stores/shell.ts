import { computed } from 'vue'
import { defineStore } from 'pinia'
import { useLocaleMode, useThemeMode } from '@octopus/composables'
import type { SupportedLocale } from '@octopus/i18n'

const themeOrder = ['system', 'light', 'dark'] as const
const localeOrder: SupportedLocale[] = ['zh-CN', 'en-US']

export const useShellStore = defineStore('shell', () => {
  const { mode, resolved } = useThemeMode()
  const { locale } = useLocaleMode()

  const localeValue = computed<SupportedLocale>({
    get: () => locale.value,
    set: (value) => {
      locale.value = value
    },
  })

  function cycleTheme() {
    const currentIndex = themeOrder.indexOf(mode.value)
    const nextIndex = (currentIndex + 1) % themeOrder.length

    mode.value = themeOrder[nextIndex]
  }

  function toggleLocale() {
    const currentIndex = localeOrder.indexOf(localeValue.value)
    const nextIndex = (currentIndex + 1) % localeOrder.length

    localeValue.value = localeOrder[nextIndex]
  }

  return {
    locale: localeValue,
    mode,
    resolved,
    cycleTheme,
    toggleLocale,
  }
})
