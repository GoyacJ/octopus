import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const appRoot = path.resolve(import.meta.dirname, '..')
const repoRoot = path.resolve(appRoot, '../..')

describe('website workspace governance', () => {
  it('ships the public assets referenced by the marketing pages', () => {
    const expectedAssets = [
      'public/brand/logo.png',
      'public/brand/og-cover.png',
      'public/screenshots/dashboard.png',
      'public/screenshots/conversation.png',
      'public/screenshots/knowledge.png',
      'public/screenshots/trace.png',
      'public/screenshots/settings-governance.png',
      'public/graphics/value-loop.png',
      'public/graphics/platform-layers.png',
      'public/graphics/governance-flow.png',
    ]

    for (const relativePath of expectedAssets) {
      expect(existsSync(path.join(appRoot, relativePath)), relativePath).toBe(true)
    }
  })

  it('participates in version governance with the other workspace packages', () => {
    const governanceSource = readFileSync(path.join(repoRoot, 'scripts', 'governance-lib.mjs'), 'utf8')

    expect(governanceSource).toContain("'apps/website/package.json'")
  })
})
