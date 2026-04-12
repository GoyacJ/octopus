// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import { enumLabel } from '@/i18n/copy'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useNotificationStore } from '@/stores/notifications'
import { useResourceStore } from '@/stores/resource'
import * as tauriClient from '@/tauri/client'
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
      throw new Error('Timed out waiting for project resources state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Project resources view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/projects/proj-redesign/resources')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders project resources from the workspace API and filters them by search', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Desktop Redesign Brief')

    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Desktop Redesign Brief')
    expect(mounted.container.textContent).toContain('Desktop Redesign API')
    expect(mounted.container.textContent).toContain('data/projects/proj-redesign/resources')
    expect(mounted.container.querySelector('[data-testid="project-resources-view-list"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-resources-view-card"]')).toBeNull()

    const searchInput = mounted.container.querySelector<HTMLInputElement>('input')
    expect(searchInput).not.toBeNull()
    searchInput!.value = 'api'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('Desktop Redesign API')
    expect(mounted.container.textContent).not.toContain('Desktop Redesign Brief')

    mounted.destroy()
  })

  it('shows the real empty state when the search has no matches', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Desktop Redesign Brief')

    const searchInput = mounted.container.querySelector<HTMLInputElement>('input')
    expect(searchInput).not.toBeNull()
    searchInput!.value = 'not-found-resource'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain(String(i18n.global.t('resources.empty.title')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('resources.empty.description')))

    mounted.destroy()
  })

  it('deactivates and deletes project resources through the project resource workbench', async () => {
    const mounted = mountApp()
    const resourceStore = useResourceStore()
    const notificationStore = useNotificationStore()

    await waitForText(mounted.container, 'Desktop Redesign Brief')

    expect(mounted.container.querySelector('[data-testid="project-resource-actions-proj-redesign-res-2"]')).toBeNull()

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resource-status-toggle-proj-redesign-res-2"] button')
      ?.click()

    await waitFor(() =>
      resourceStore.activeProjectResources.find(resource => resource.id === 'proj-redesign-res-2')?.status === 'attention',
    )

    expect(mounted.container.textContent).toContain(enumLabel('resourceStatus', 'attention'))
    expect(mounted.container.textContent).not.toContain('已配置')
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title === String(i18n.global.t('resources.notifications.status.disabled.title')),
      ),
    )

    await router.push('/workspaces/ws-local/projects/proj-redesign/resources?resourceId=proj-redesign-res-1')
    await router.isReady()
    await nextTick()

    await waitFor(() =>
      mounted.container.querySelector('[data-testid="project-resource-detail-delete"]') !== null,
    )

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resource-detail-delete"]')
      ?.click()

    await waitFor(() =>
      document.body.querySelector('[data-testid="project-resource-delete-dialog"]') !== null,
    )

    document.body
      .querySelector<HTMLButtonElement>('[data-testid="project-resource-delete-confirm"]')
      ?.click()

    await waitFor(() =>
      !resourceStore.activeProjectResources.some(resource => resource.id === 'proj-redesign-res-1'),
    )

    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title === String(i18n.global.t('resources.notifications.delete.successTitle')),
      ),
    )

    mounted.destroy()
  })

  it('opens the resource detail panel from the route query and updates visibility from the detail pane', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/resources?resourceId=proj-redesign-res-1')
    await router.isReady()

    const mounted = mountApp()
    const resourceStore = useResourceStore()

    await waitFor(() =>
      mounted.container.querySelector('[data-testid="project-resource-detail"]') !== null,
    )

    expect(mounted.container.textContent).toContain('Desktop Redesign Brief')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('resources.preview.markdown')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('resources.detail.actions')))

    const visibilitySelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="project-resource-detail-visibility"]')
    expect(visibilitySelect).not.toBeNull()
    visibilitySelect!.value = 'private'
    visibilitySelect!.dispatchEvent(new Event('change', { bubbles: true }))

    await waitFor(() =>
      resourceStore.activeProjectResources.find(resource => resource.id === 'proj-redesign-res-1')?.visibility === 'private',
    )

    expect(mounted.container.textContent).toContain(enumLabel('resourceVisibility', 'private'))

    mounted.destroy()
  })

  it('supports pagination without page numbers and submits workspace promotion for review from the detail pane', async () => {
    const mounted = mountApp()
    const resourceStore = useResourceStore()
    const notificationStore = useNotificationStore()

    await waitForText(mounted.container, 'Desktop Redesign Brief')

    for (const index of [0, 1, 2, 3]) {
      await resourceStore.importProjectResource('proj-redesign', {
        name: `Imported Resource ${index}`,
        scope: 'project',
        visibility: 'public',
        files: [
          {
            fileName: `imported-${index}.md`,
            contentType: 'text/markdown',
            dataBase64: btoa(`# Imported ${index}`),
            byteSize: 14,
            relativePath: `imported-${index}.md`,
          },
        ],
      })
    }

    await waitFor(() =>
      mounted.container.textContent?.includes('Imported Resource 1') ?? false,
    )

    expect(mounted.container.querySelector('[data-testid="project-resources-pagination-next"]')).not.toBeNull()
    expect(mounted.container.textContent).not.toContain('第 1 / 2 页')
    expect(mounted.container.textContent).not.toContain('1 / 2')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resources-pagination-next"]')
      ?.click()
    await nextTick()

    await waitFor(() =>
      mounted.container.textContent?.includes('Imported Resource 3') ?? false,
    )

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resources-pagination-previous"]')
      ?.click()
    await nextTick()

    await router.push('/workspaces/ws-local/projects/proj-redesign/resources?resourceId=proj-redesign-res-3')
    await router.isReady()
    await nextTick()

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resource-detail-promote"]')
      ?.click()

    await waitFor(() =>
      resourceStore.activeProjectResources.find(resource => resource.id === 'proj-redesign-res-3')?.scope === 'project',
    )
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title === String(i18n.global.t('resources.notifications.promote.successTitle')),
      ),
    )

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resource-detail-promote"]')
      ?.click()

    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title === String(i18n.global.t('resources.notifications.promote.submittedTitle')),
      ),
    )
    expect(resourceStore.activeProjectResources.find(resource => resource.id === 'proj-redesign-res-3')?.scope).toBe('project')
    expect(notificationStore.notificationsState[0]?.body).toContain('Desktop Redesign Personal Notes')

    mounted.destroy()
  })

  it('hides the promote action when the current user is not the project owner', async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'
        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        ;(project as any).ownerUserId = 'user-owner'
        ;(project as any).memberUserIds = ['user-owner', 'user-operator']
      },
    })
    await router.push('/workspaces/ws-local/projects/proj-redesign/resources?resourceId=proj-redesign-res-3')
    await router.isReady()

    const mounted = mountApp()

    await waitFor(() =>
      mounted.container.querySelector('[data-testid="project-resource-detail"]') !== null,
    )

    expect(mounted.container.querySelector('[data-testid="project-resource-detail-promote"]')).toBeNull()

    mounted.destroy()
  })

  it('imports uploaded files and folders into the project resource directory and previews them', async () => {
    vi.spyOn(tauriClient, 'pickResourceFile').mockResolvedValue({
      fileName: 'launch-plan.md',
      contentType: 'text/markdown',
      dataBase64: btoa('# Launch Plan'),
      byteSize: 13,
    })
    vi.spyOn(tauriClient, 'pickResourceFolder').mockResolvedValue([
      {
        fileName: 'readme.md',
        contentType: 'text/markdown',
        dataBase64: btoa('# Folder Readme'),
        byteSize: 15,
        relativePath: 'design-assets/readme.md',
      },
      {
        fileName: 'spec.json',
        contentType: 'application/json',
        dataBase64: btoa('{"ok":true}'),
        byteSize: 11,
        relativePath: 'design-assets/nested/spec.json',
      },
    ])

    const mounted = mountApp()
    const resourceStore = useResourceStore()
    const notificationStore = useNotificationStore()

    await waitForText(mounted.container, 'Desktop Redesign Brief')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resources-upload-file"]')
      ?.click()

    await waitFor(() =>
      resourceStore.activeProjectResources.some(resource => resource.name === 'launch-plan.md'),
    )

    const uploadedFile = resourceStore.activeProjectResources.find(resource => resource.name === 'launch-plan.md')
    expect(uploadedFile?.storagePath).toBe('data/projects/proj-redesign/resources/launch-plan.md')
    await waitFor(() => mounted.container.textContent?.includes('# Launch Plan') ?? false)
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title === String(i18n.global.t('resources.notifications.upload.fileTitle')),
      ),
    )
    expect(notificationStore.notificationsState[0]?.body).toContain('data/projects/proj-redesign/resources/launch-plan.md')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resources-upload-folder"]')
      ?.click()

    await waitFor(() =>
      resourceStore.activeProjectResources.some(resource => resource.name === 'design-assets'),
    )

    const uploadedFolder = resourceStore.activeProjectResources.find(resource => resource.name === 'design-assets')
    expect(uploadedFolder?.storagePath).toBe('data/projects/proj-redesign/resources/design-assets')
    await waitFor(() => mounted.container.textContent?.includes('nested/spec.json') ?? false)
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title === String(i18n.global.t('resources.notifications.upload.folderTitle')),
      ),
    )
    expect(notificationStore.notificationsState[0]?.body).toContain('data/projects/proj-redesign/resources/design-assets')

    mounted.destroy()
  })

  it('renders imported image previews even when the uploaded content type is empty', async () => {
    vi.spyOn(tauriClient, 'pickResourceFile').mockResolvedValue({
      fileName: 'hero.png',
      contentType: '',
      dataBase64: 'iVBORw0KGgo=',
      byteSize: 8,
    })

    const mounted = mountApp()
    const resourceStore = useResourceStore()

    await waitForText(mounted.container, 'Desktop Redesign Brief')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-resources-upload-file"]')
      ?.click()

    await waitFor(() =>
      resourceStore.activeProjectResources.some(resource => resource.name === 'hero.png'),
    )

    await waitFor(() =>
      mounted.container.querySelector('[data-testid="project-resource-image-preview"]') !== null,
    )

    const preview = mounted.container.querySelector<HTMLImageElement>('[data-testid="project-resource-image-preview"]')
    expect(preview?.getAttribute('src')).toMatch(/^data:image\/png;base64,/)
    expect(mounted.container.querySelector('[data-testid="project-resource-preview-fallback"]')).toBeNull()

    mounted.destroy()
  })
})
