// @vitest-environment node

import { describe, expect, it } from 'vitest'

import viteConfig from '../../vite.config.ts'

describe('vite local runtime wiring', () => {
  it('proxies local /api requests to the octopus server during development', () => {
    expect(viteConfig.server?.proxy?.['/api']).toMatchObject({
      target: 'http://127.0.0.1:3000',
      changeOrigin: true,
    })
  })
})
