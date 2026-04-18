import { describe, expect, it } from 'vitest'

import { collectAdapterRoutes, collectServerRoutes } from '../../../scripts/openapi-parity-lib.mjs'

describe('OpenAPI parity collectors', () => {
  it('collects normalized server routes from the active router definitions', async () => {
    const routes = await collectServerRoutes()

    expect(routes.length).toBeGreaterThan(0)
    expect(routes).toContain('/api/v1/workspace')
    expect(routes).toContain('/api/v1/host/health')
    expect(routes).toContain('/api/v1/runtime/sessions')
    expect(routes).toContain('/api/v1/runtime/sessions/{param}/events')
  })

  it('collects normalized adapter routes from the active desktop API modules', async () => {
    const routes = await collectAdapterRoutes()

    expect(routes.length).toBeGreaterThan(0)
    expect(routes).toContain('/api/v1/runtime/sessions/{param}/events')
    expect(routes).toContain('/api/v1/workspace/catalog/skills/{param}/files/{param}')
    expect(routes).toContain('/api/v1/host/notifications/{param}/read')
    expect(routes).not.toContain('/api/v1/workspace/automations')
    expect(routes).not.toContain('/api/v1/workspace/automations/{param}')
  })
})
