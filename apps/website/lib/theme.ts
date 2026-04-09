export type ThemePreference = 'system' | 'light' | 'dark'
export type AppliedTheme = 'light' | 'dark'

export const themeStorageKey = 'octopus-website-theme'

export function parseThemePreference(value: string | null | undefined): ThemePreference {
  if (value === 'light' || value === 'dark' || value === 'system') {
    return value
  }

  return 'system'
}

export function resolveAppliedTheme(preference: ThemePreference, systemPrefersDark: boolean): AppliedTheme {
  if (preference === 'light' || preference === 'dark') {
    return preference
  }

  return systemPrefersDark ? 'dark' : 'light'
}

export function applyThemeToDocument(theme: AppliedTheme) {
  if (!import.meta.client) {
    return
  }

  const root = document.documentElement
  root.dataset.theme = theme
  root.style.colorScheme = theme
  root.classList.toggle('dark', theme === 'dark')
}
