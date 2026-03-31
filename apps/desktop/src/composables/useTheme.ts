import { ref, onMounted } from 'vue'
import { ThemeMode } from '@octopus/schema'

export function useTheme() {
  const theme = ref<ThemeMode>(
    (localStorage.getItem('octopus-theme') as ThemeMode) || 'system'
  )

  const updateTheme = (newTheme: ThemeMode) => {
    const html = document.documentElement
    let effectiveTheme = newTheme

    if (newTheme === 'system') {
      effectiveTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light'
    }

    html.setAttribute('data-theme', effectiveTheme)
    localStorage.setItem('octopus-theme', newTheme)
    theme.value = newTheme
  }

  onMounted(() => {
    updateTheme(theme.value)
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      if (theme.value === 'system') {
        updateTheme('system')
      }
    })
  })

  return {
    theme,
    updateTheme
  }
}
