export const navigationItems = [
  { key: 'home', path: '/' },
  { key: 'product', path: '/product' },
  { key: 'scenarios', path: '/scenarios' },
  { key: 'about', path: '/about' },
] as const

export const proofMetricIds = ['conversations', 'knowledge', 'trace'] as const
export const homeNarrativeIds = ['think', 'execute', 'retain'] as const
export const scenarioIds = ['individual', 'team', 'enterprise'] as const
export const aboutPrincipleIds = ['calm', 'systems', 'memory', 'trust'] as const
export const bookDemoOptionIds = ['call', 'email', 'product'] as const

export const interfaceProofs = [
  { id: 'dashboard', src: '/screenshots/dashboard.png', aspect: 'landscape' },
  { id: 'conversation', src: '/screenshots/conversation.png', aspect: 'landscape' },
  { id: 'knowledge', src: '/screenshots/knowledge.png', aspect: 'landscape' },
  { id: 'trace', src: '/screenshots/trace.png', aspect: 'landscape' },
  { id: 'governance', src: '/screenshots/settings-governance.png', aspect: 'landscape' },
] as const

export const productFeatureCards = [
  { id: 'conversations', mediaId: 'conversation' },
  { id: 'agents', mediaId: 'dashboard' },
  { id: 'knowledge', mediaId: 'knowledge' },
  { id: 'trace', mediaId: 'trace' },
  { id: 'automations', mediaId: 'governance' },
  { id: 'runtime', mediaId: 'dashboard' },
] as const

export const editorialGraphics = {
  valueLoop: '/graphics/value-loop.png',
  platformLayers: '/graphics/platform-layers.png',
  governanceFlow: '/graphics/governance-flow.png',
} as const
