import { ref } from 'vue'
import { ThemeMode } from '@octopus/schema'

export function useTheme(initialTheme: ThemeMode = 'system') {
  const theme = ref<ThemeMode>(initialTheme)

  const updateTheme = (newTheme: ThemeMode) => {
    theme.value = newTheme
  }

  return {
    theme,
    updateTheme,
  }
}
