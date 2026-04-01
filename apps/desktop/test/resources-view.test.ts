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

describe('Project resources view', () => {
  beforeEach(async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/resources')
    await router.isReady()
    document.body.innerHTML = ''
    window.confirm = () => true
  })

  it('supports creating url resources, filtering, and switching between list and grid views', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.textContent).not.toContain('统一查看文件、文件夹、Artifact 引用和上传资源。')
    expect(mounted.container.textContent).not.toContain('资源列表')

    const addTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-add-trigger"]')
    expect(addTrigger).not.toBeNull()
    addTrigger?.click()
    await nextTick()

    const addUrl = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-add-url"]')
    expect(addUrl).not.toBeNull()
    addUrl?.click()
    await nextTick()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="resource-url-name-input"]')
    const locationInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="resource-url-location-input"]')
    const confirmButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resource-url-confirm"]')

    expect(nameInput).not.toBeNull()
    expect(locationInput).not.toBeNull()
    expect(confirmButton).not.toBeNull()

    nameInput!.value = 'API docs'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    locationInput!.value = 'https://example.com/docs'
    locationInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    confirmButton?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('API docs')

    for (let index = 0; index < 12; index += 1) {
      const addTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-add-trigger"]')
      addTrigger?.click()
      await nextTick()
      mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-add-file"]')?.click()
      await nextTick()
    }

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-pagination-next"]')?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('第 2 / 4 页')

    const searchInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="resources-search-input"]')
    expect(searchInput).not.toBeNull()
    searchInput!.value = 'Shell Layout'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const visibleItems = mounted.container.querySelectorAll('[data-testid^="resource-item-"]')
    expect(visibleItems.length).toBe(1)
    expect(mounted.container.textContent).toContain('Shell Layout Notes')
    expect(mounted.container.textContent).toContain('第 1 / 1 页')

    const gridToggle = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-view-grid"]')
    expect(gridToggle).not.toBeNull()
    gridToggle?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="resources-grid"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('第 1 / 1 页')

    mounted.destroy()
  })

  it('supports previewing, editing, deleting, and paginating resources', async () => {
    const mounted = mountApp()

    await nextTick()

    for (let index = 0; index < 12; index += 1) {
      const addTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-add-trigger"]')
      addTrigger?.click()
      await nextTick()
      mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-add-file"]')?.click()
      await nextTick()
    }

    const nextPage = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-pagination-next"]')
    expect(nextPage).not.toBeNull()
    expect(nextPage?.disabled).toBe(false)
    nextPage?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Mock File')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="resources-pagination-prev"]')?.click()
    await nextTick()

    const previewButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid^="resource-preview-res-file-mock-"]')
    expect(previewButton).not.toBeNull()
    const targetResourceId = previewButton?.getAttribute('data-testid')?.replace('resource-preview-', '')
    expect(targetResourceId).toBeTruthy()
    previewButton?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="resource-preview-modal"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="resource-preview-close"]')?.click()
    await nextTick()

    const editButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="resource-edit-${targetResourceId}"]`)
    expect(editButton).not.toBeNull()
    editButton?.click()
    await nextTick()

    const renameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="resource-edit-name-input"]')
    const saveEdit = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resource-edit-confirm"]')
    expect(renameInput).not.toBeNull()
    expect(saveEdit).not.toBeNull()

    renameInput!.value = 'Renamed layout notes.md'
    renameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    saveEdit?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Renamed layout notes')

    const deleteButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="resource-delete-${targetResourceId}"]`)
    expect(deleteButton).not.toBeNull()
    deleteButton?.click()
    await nextTick()

    const confirmDelete = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resource-delete-confirm"]')
    expect(confirmDelete).not.toBeNull()
    confirmDelete?.click()
    await nextTick()

    expect(mounted.container.textContent).not.toContain('Renamed layout notes')

    mounted.destroy()
  })
})
