export type NavigationId =
  | 'overview'
  | 'workspaces'
  | 'agents'
  | 'runs'
  | 'inbox'
  | 'triggers'
  | 'artifacts'
  | 'extensions'
  | 'nodes'
  | 'audit'
  | 'settings'

export interface NavigationDefinition {
  id: NavigationId
  to: string
  labelKey: string
  phaseZeroEnabled: boolean
}

export const navigationDefinitions: NavigationDefinition[] = [
  { id: 'overview', to: '/', labelKey: 'navigation.overview', phaseZeroEnabled: true },
  { id: 'workspaces', to: '/workspaces', labelKey: 'navigation.workspaces', phaseZeroEnabled: true },
  { id: 'agents', to: '/agents', labelKey: 'navigation.agents', phaseZeroEnabled: true },
  { id: 'runs', to: '/runs', labelKey: 'navigation.runs', phaseZeroEnabled: true },
  { id: 'inbox', to: '/inbox', labelKey: 'navigation.inbox', phaseZeroEnabled: true },
  { id: 'triggers', to: '/triggers', labelKey: 'navigation.triggers', phaseZeroEnabled: false },
  { id: 'artifacts', to: '/artifacts', labelKey: 'navigation.artifacts', phaseZeroEnabled: false },
  { id: 'extensions', to: '/extensions', labelKey: 'navigation.extensions', phaseZeroEnabled: false },
  { id: 'nodes', to: '/nodes', labelKey: 'navigation.nodes', phaseZeroEnabled: false },
  { id: 'audit', to: '/audit', labelKey: 'navigation.audit', phaseZeroEnabled: true },
  { id: 'settings', to: '/settings', labelKey: 'navigation.settings', phaseZeroEnabled: false },
]
