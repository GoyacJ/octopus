import { computed } from 'vue'

import {
  applyThemeToDocument,
  parseThemePreference,
  resolveAppliedTheme,
  themeStorageKey,
  type ThemePreference,
} from '../lib/theme'

export function useWebsiteTheme() {
  const preference = useState<ThemePreference>('website-theme-preference', () => 'system')
  const systemPrefersDark = useState<boolean>('website-system-prefers-dark', () => false)

  const appliedTheme = computed(() => resolveAppliedTheme(preference.value, systemPrefersDark.value))

  function setPreference(nextPreference: ThemePreference) {
    preference.value = nextPreference

    if (import.meta.client) {
      window.localStorage.setItem(themeStorageKey, nextPreference)
      applyThemeToDocument(appliedTheme.value)
    }
  }

  function hydrateTheme(nextPreference: string | null | undefined, prefersDark: boolean) {
    preference.value = parseThemePreference(nextPreference)
    systemPrefersDark.value = prefersDark

    if (import.meta.client) {
      applyThemeToDocument(appliedTheme.value)
    }
  }

  return {
    appliedTheme,
    preference,
    options: ['system', 'light', 'dark'] as ThemePreference[],
    setPreference,
    hydrateTheme,
  }
}
