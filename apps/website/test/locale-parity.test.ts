import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const appRoot = path.resolve(import.meta.dirname, '..')
const zhPath = path.join(appRoot, 'locales', 'zh-CN.json')
const enPath = path.join(appRoot, 'locales', 'en-US.json')

function collectLeafPaths(value: unknown, prefix = ''): string[] {
  if (Array.isArray(value)) {
    return value.flatMap((item, index) => collectLeafPaths(item, prefix ? `${prefix}.${index}` : `${index}`))
  }

  if (value && typeof value === 'object') {
    return Object.entries(value).flatMap(([key, nestedValue]) =>
      collectLeafPaths(nestedValue, prefix ? `${prefix}.${key}` : key),
    )
  }

  return prefix ? [prefix] : []
}

describe('website locale registry', () => {
  it('keeps zh-CN and en-US leaf keys in parity', () => {
    expect(existsSync(zhPath)).toBe(true)
    expect(existsSync(enPath)).toBe(true)

    if (!existsSync(zhPath) || !existsSync(enPath)) {
      return
    }

    const zhCN = JSON.parse(readFileSync(zhPath, 'utf8')) as unknown
    const enUS = JSON.parse(readFileSync(enPath, 'utf8')) as unknown

    expect(collectLeafPaths(zhCN).sort()).toEqual(collectLeafPaths(enUS).sort())
  })

  it('covers the required navigation, theme, seo, and page namespaces', () => {
    expect(existsSync(zhPath)).toBe(true)

    if (!existsSync(zhPath)) {
      return
    }

    const zhCN = JSON.parse(readFileSync(zhPath, 'utf8')) as unknown
    const keys = collectLeafPaths(zhCN)

    expect(keys).toContain('site.name')
    expect(keys).toContain('nav.home')
    expect(keys).toContain('nav.product')
    expect(keys).toContain('nav.scenarios')
    expect(keys).toContain('nav.about')
    expect(keys).toContain('nav.bookDemo')
    expect(keys).toContain('theme.system')
    expect(keys).toContain('theme.light')
    expect(keys).toContain('theme.dark')
    expect(keys).toContain('footer.tagline')
    expect(keys).toContain('seo.home.title')
    expect(keys).toContain('seo.product.title')
    expect(keys).toContain('seo.scenarios.title')
    expect(keys).toContain('seo.about.title')
    expect(keys).toContain('seo.bookDemo.title')
    expect(keys).toContain('home.hero.title')
    expect(keys).toContain('product.hero.title')
    expect(keys).toContain('scenarios.hero.title')
    expect(keys).toContain('about.hero.title')
    expect(keys).toContain('bookDemo.hero.title')
  })
})
