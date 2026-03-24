import tokens from './tokens.json'

export type ThemeMode = 'system' | 'light' | 'dark'

export const tokenBundle = tokens
export const supportedThemeModes: ThemeMode[] = ['system', 'light', 'dark']
