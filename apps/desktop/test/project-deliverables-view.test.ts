// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useKnowledgeStore } from '@/stores/knowledge'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

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

async function waitForText(container: HTMLElement, value: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!(container.textContent?.includes(value) ?? false)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for text: ${value}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for project deliverables state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Project deliverables view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/projects/proj-redesign/deliverables')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders project deliverables and opens the selected preview', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Runtime Delivery Summary')

    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Workspace Protocol Baseline')
    expect(mounted.container.querySelector('[data-testid="project-deliverable-detail"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Version 3 content for artifact-run-conv-redesign.')

    mounted.destroy()
  })

  it('promotes the selected deliverable to knowledge and forks it into a new conversation', async () => {
    const mounted = mountApp()
    const knowledgeStore = useKnowledgeStore()

    await waitForText(mounted.container, 'Runtime Delivery Summary')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-deliverable-promote"]')
      ?.click()

    await waitFor(() =>
      knowledgeStore.activeProjectKnowledge.some(entry => entry.sourceRef === 'artifact-run-conv-redesign'),
    )

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-deliverable-fork"]')
      ?.click()

    await waitFor(() => router.currentRoute.value.name === 'project-conversation')
    expect(router.currentRoute.value.fullPath).toContain('/conversations/conv-fork-artifact-run-conv-redesign-')

    mounted.destroy()
  })
})
