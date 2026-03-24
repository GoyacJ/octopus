import { computed } from 'vue'
import { usePreferredDark, useStorage } from '@vueuse/core'

export type ThemeMode = 'system' | 'light' | 'dark'
export type LocaleMode = 'zh-CN' | 'en-US'

export function useThemeMode() {
  const preferredDark = usePreferredDark()
  const mode = useStorage<ThemeMode>('octopus.theme.mode', 'system')
  const resolved = computed<'light' | 'dark'>(() => {
    if (mode.value === 'system') {
      return preferredDark.value ? 'dark' : 'light'
    }

    return mode.value
  })

  return {
    mode,
    resolved,
  }
}

export function useLocaleMode() {
  const locale = useStorage<LocaleMode>('octopus.locale', 'zh-CN')

  return {
    locale,
  }
}
