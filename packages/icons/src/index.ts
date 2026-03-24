export const navigationIcons = {
  overview: 'i-lucide-layout-dashboard',
  workspaces: 'i-lucide-folders',
  agents: 'i-lucide-bot',
  runs: 'i-lucide-activity',
  inbox: 'i-lucide-inbox',
  triggers: 'i-lucide-alarm-clock',
  artifacts: 'i-lucide-box',
  extensions: 'i-lucide-puzzle',
  nodes: 'i-lucide-server',
  audit: 'i-lucide-shield-check',
  settings: 'i-lucide-settings-2',
} as const

export type NavigationIconKey = keyof typeof navigationIcons
