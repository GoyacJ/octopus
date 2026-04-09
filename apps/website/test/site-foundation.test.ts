import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const appRoot = path.resolve(import.meta.dirname, '..')
const repoRoot = path.resolve(appRoot, '../..')

function readJson(filePath: string) {
  return JSON.parse(readFileSync(filePath, 'utf8')) as {
    scripts?: Record<string, string>
    dependencies?: Record<string, string>
    devDependencies?: Record<string, string>
  }
}

describe('website app foundation', () => {
  it('defines the expected Nuxt website app skeleton', () => {
    expect(existsSync(path.join(appRoot, 'nuxt.config.ts'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'app.vue'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'pages', 'index.vue'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'pages', 'product.vue'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'pages', 'scenarios.vue'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'pages', 'about.vue'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'pages', 'book-demo.vue'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'components', 'brand'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'components', 'shared'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'composables'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'locales', 'zh-CN.json'))).toBe(true)
    expect(existsSync(path.join(appRoot, 'locales', 'en-US.json'))).toBe(true)
  })

  it('registers the expected Nuxt modules and website scripts', () => {
    const appPackage = readJson(path.join(appRoot, 'package.json'))
    const rootPackage = readJson(path.join(repoRoot, 'package.json'))

    expect(appPackage.dependencies?.nuxt).toBeTruthy()
    expect(appPackage.dependencies?.['@nuxtjs/i18n']).toBeTruthy()
    expect(appPackage.dependencies?.['@nuxtjs/tailwindcss']).toBeTruthy()
    expect(appPackage.devDependencies?.['vue-tsc']).toBeTruthy()
    expect(appPackage.scripts?.dev).toBe('nuxt dev')
    expect(appPackage.scripts?.generate).toBe('nuxt generate')
    expect(appPackage.scripts?.typecheck).toBe('nuxt typecheck')

    expect(rootPackage.scripts?.['dev:website']).toBe('pnpm -C apps/website dev')
    expect(rootPackage.scripts?.['build:website']).toBe('pnpm -C apps/website generate')
    expect(rootPackage.scripts?.['check:website']).toBe('pnpm -C apps/website typecheck && pnpm -C apps/website test && pnpm -C apps/website generate')
    expect(rootPackage.scripts?.['check:frontend']).toBe('pnpm check:desktop && pnpm check:website')
    expect(rootPackage.scripts?.['check:desktop-release']).not.toContain('pnpm check:website')
  })
})
