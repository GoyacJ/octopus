import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const websiteRoot = path.resolve(import.meta.dirname, '..')
const locales = {
  zh: JSON.parse(readFileSync(path.join(websiteRoot, 'locales', 'zh-CN.json'), 'utf8')),
  en: JSON.parse(readFileSync(path.join(websiteRoot, 'locales', 'en-US.json'), 'utf8')),
}

const readPage = (name: string) =>
  readFileSync(path.join(websiteRoot, 'pages', name), 'utf8')

describe('website page content integrity', () => {
  it('keeps structured website narrative sections available in every locale', () => {
    expect(locales.zh.pages.scenarios.segments).toHaveLength(3)
    expect(locales.en.pages.scenarios.segments).toHaveLength(3)
    expect(locales.zh.pages.product.governance.items).toHaveLength(3)
    expect(locales.en.pages.product.governance.items).toHaveLength(3)
    expect(locales.zh.pages.home.workflow.steps).toHaveLength(3)
    expect(locales.en.pages.home.workflow.steps).toHaveLength(3)
    expect(locales.zh.pages.home.comparison.items).toHaveLength(6)
    expect(locales.en.pages.home.comparison.items).toHaveLength(6)
    expect(locales.zh.pages.home.useCases.items).toHaveLength(3)
    expect(locales.en.pages.home.useCases.items).toHaveLength(3)
    expect(locales.zh.pages.home.faq.items).toHaveLength(7)
    expect(locales.en.pages.home.faq.items).toHaveLength(7)
    expect(locales.zh.pages.product.modules.items).toHaveLength(4)
    expect(locales.en.pages.product.modules.items).toHaveLength(4)
    expect(locales.zh.pages.scenarios.useCases.items).toHaveLength(6)
    expect(locales.en.pages.scenarios.useCases.items).toHaveLength(6)
    expect(locales.zh.pages.about.principles.items).toHaveLength(4)
    expect(locales.en.pages.about.principles.items).toHaveLength(4)
    expect(locales.zh.pages.bookDemo.valueProps.items).toHaveLength(4)
    expect(locales.en.pages.bookDemo.valueProps.items).toHaveLength(4)
  })

  it('uses tm() instead of t() when reading translated object arrays', () => {
    const homePage = readPage('index.vue')
    const scenariosPage = readPage('scenarios.vue')
    const productPage = readPage('product.vue')
    const aboutPage = readPage('about.vue')
    const bookDemoPage = readPage('book-demo.vue')

    expect(homePage).toContain("tm('pages.home.workflow.steps')")
    expect(homePage).toContain("tm('pages.home.comparison.items')")
    expect(homePage).toContain("tm('pages.home.useCases.items')")
    expect(homePage).toContain("tm('pages.home.faq.items')")
    expect(homePage).toContain('rt(')
    expect(scenariosPage).toContain("tm('pages.scenarios.segments')")
    expect(scenariosPage).toContain("tm('pages.scenarios.useCases.items')")
    expect(scenariosPage).toContain('rt(')
    expect(scenariosPage).not.toContain("t('pages.scenarios.segments')")
    expect(productPage).toContain("tm('pages.product.governance.items')")
    expect(productPage).toContain("tm('pages.product.modules.items')")
    expect(productPage).toContain('rt(')
    expect(productPage).not.toContain("t('pages.product.governance.items')")
    expect(aboutPage).toContain("tm('pages.about.highlights')")
    expect(aboutPage).toContain("tm('pages.about.principles.items')")
    expect(aboutPage).toContain('rt(')
    expect(bookDemoPage).toContain("tm('pages.bookDemo.valueProps.items')")
    expect(bookDemoPage).toContain('rt(')
  })

  it('keeps shared screenshots and touch icons available from public assets', () => {
    const assetPaths = [
      'public/apple-touch-icon.png',
      'public/apple-touch-icon-precomposed.png',
      'public/favicon.png',
      'public/logo.png',
      'public/screenshots/agent.png',
      'public/screenshots/builtin.png',
      'public/screenshots/conversation.png',
      'public/screenshots/dashboard.png',
      'public/screenshots/mcp.png',
      'public/screenshots/rbac.png',
      'public/screenshots/skill.png',
    ]

    assetPaths.forEach((assetPath) => {
      expect(existsSync(path.join(websiteRoot, assetPath))).toBe(true)
    })
  })
})
