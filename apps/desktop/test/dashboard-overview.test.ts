// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useWorkbenchStore } from '@/stores/workbench'

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

describe('Dashboard and overview surfaces', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
  })

  it('renders the shared overview hero and action entry points', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="workspace-overview-hero"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-overview-action-dashboard"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-overview-action-conversation"]')).not.toBeNull()

    mounted.destroy()
  })

  it('renders the dashboard hero and keeps the project edit flow working with RBAC controls', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/dashboard')
    await router.isReady()

    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="project-dashboard-hero"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-dashboard-action-conversation"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-dashboard-action-knowledge"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-dashboard-action-trace"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-dashboard-edit"]')?.click()
    await nextTick()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="project-dashboard-edit-name"]')
    const summaryInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="project-dashboard-edit-summary"]')

    expect(nameInput).not.toBeNull()
    expect(summaryInput).not.toBeNull()

    nameInput!.value = 'Refined Dashboard Project'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    summaryInput!.value = 'Shared UI baseline refactor remains editable.'
    summaryInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-dashboard-save"]')?.click()
    await nextTick()

    expect(workbench.projectDashboard.project.name).toBe('Refined Dashboard Project')
    expect(workbench.projectDashboard.project.summary).toContain('Shared UI baseline refactor')

    mounted.destroy()
  })
})
