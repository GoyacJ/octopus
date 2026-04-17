import { describe, expect, it } from 'vitest'

import { MENU_DEFINITIONS } from '@/navigation/menuRegistry'
import { collectAdapterRoutes, collectServerRoutes } from '../../../scripts/openapi-parity-lib.mjs'

describe('workspace automations removal', () => {
  it('removes the workspace automations menu definition', () => {
    expect(MENU_DEFINITIONS.some(item => item.id === 'menu-workspace-automations')).toBe(false)
    expect(MENU_DEFINITIONS.some(item => item.routeName === 'workspace-automations')).toBe(false)
  })

  it('removes workspace automations from adapter and server route collectors', async () => {
    const [adapterRoutes, serverRoutes] = await Promise.all([
      collectAdapterRoutes(),
      collectServerRoutes(),
    ])

    expect(adapterRoutes).not.toContain('/api/v1/workspace/automations')
    expect(adapterRoutes).not.toContain('/api/v1/workspace/automations/{param}')
    expect(serverRoutes).not.toContain('/api/v1/workspace/automations')
    expect(serverRoutes).not.toContain('/api/v1/workspace/automations/{param}')
  })
})
