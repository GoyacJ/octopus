import {
  applyThemeToDocument,
  parseThemePreference,
  resolveAppliedTheme,
  themeStorageKey,
} from '../lib/theme'

export default defineNuxtPlugin(() => {
  const preference = useState('website-theme-preference', () => 'system')
  const systemPrefersDark = useState('website-system-prefers-dark', () => false)
  const initialized = useState('website-theme-initialized', () => false)

  if (initialized.value) {
    return
  }

  const media = window.matchMedia('(prefers-color-scheme: dark)')

  const syncTheme = () => {
    const appliedTheme = resolveAppliedTheme(
      parseThemePreference(preference.value),
      systemPrefersDark.value,
    )
    applyThemeToDocument(appliedTheme)
  }

  const storedPreference = window.localStorage.getItem(themeStorageKey)
  preference.value = parseThemePreference(storedPreference)
  systemPrefersDark.value = media.matches
  syncTheme()

  media.addEventListener('change', (event) => {
    systemPrefersDark.value = event.matches
    syncTheme()
  })

  initialized.value = true
})
