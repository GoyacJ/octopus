// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})

function mountApp() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp(App)
  app.use(pinia)
  app.use(i18n)
  app.use(router)
  app.mount(container)

  return {
    app,
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

describe('Knowledge view', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    await router.push('/workspaces/ws-local/projects/proj-redesign/knowledge')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders the reading-first hero and selected knowledge detail', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="knowledge-hero-summary"]')?.textContent).toContain('偏浏览')
    expect(mounted.container.querySelector('[data-testid="knowledge-stat-total"]')?.textContent).toContain('1')
    expect(mounted.container.querySelector('[data-testid="knowledge-detail-title"]')?.textContent).toContain('Architect')
    expect(mounted.container.querySelector('[data-testid="knowledge-source-card"]')?.textContent).toContain('conv-redesign')
    expect(mounted.container.querySelector('[data-testid="knowledge-related-conversation-link"]')?.textContent).toContain('Desktop shell GA 重构')
    expect(mounted.container.querySelector('[data-testid="knowledge-lineage-list"]')?.textContent).toContain('会话')

    mounted.destroy()
  })

  it('supports kind tabs and search-driven empty states', async () => {
    const mounted = mountApp()

    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-filter-chip-shared"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="knowledge-empty-state"]')?.textContent).toContain('没有匹配的知识')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-filter-chip-all"]')?.click()
    await nextTick()

    const searchInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="knowledge-search-input"]')
    expect(searchInput).not.toBeNull()

    searchInput!.value = '不存在的关键字'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="knowledge-empty-state"]')?.textContent).toContain('没有匹配的知识')

    mounted.destroy()
  })
})
