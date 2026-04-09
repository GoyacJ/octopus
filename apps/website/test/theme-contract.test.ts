import { describe, expect, it } from 'vitest'

import {
  parseThemePreference,
  resolveAppliedTheme,
  themeStorageKey,
} from '../lib/theme'

describe('website theme contract', () => {
  it('parses only supported theme preferences', () => {
    expect(parseThemePreference('system')).toBe('system')
    expect(parseThemePreference('light')).toBe('light')
    expect(parseThemePreference('dark')).toBe('dark')
    expect(parseThemePreference('sepia')).toBe('system')
    expect(parseThemePreference(null)).toBe('system')
    expect(parseThemePreference(undefined)).toBe('system')
  })

  it('resolves system theme using the OS preference', () => {
    expect(resolveAppliedTheme('system', true)).toBe('dark')
    expect(resolveAppliedTheme('system', false)).toBe('light')
    expect(resolveAppliedTheme('light', true)).toBe('light')
    expect(resolveAppliedTheme('dark', false)).toBe('dark')
  })

  it('uses the agreed local storage key', () => {
    expect(themeStorageKey).toBe('octopus-website-theme')
  })
})
