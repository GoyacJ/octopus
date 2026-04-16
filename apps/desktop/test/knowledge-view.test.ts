// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import type { KnowledgeRecord } from '@octopus/schema'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
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

function createKnowledgeRecord(overrides: Partial<KnowledgeRecord> = {}): KnowledgeRecord {
  return {
    id: 'knowledge-default',
    workspaceId: 'ws-local',
    title: 'Knowledge Default',
    summary: 'Knowledge default summary.',
    kind: 'shared',
    status: 'reviewed',
    sourceType: 'artifact',
    sourceRef: 'artifact-default',
    updatedAt: 100,
    scope: 'workspace',
    visibility: 'public',
    ...overrides,
  }
}

describe('Knowledge view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.projectKnowledge['proj-redesign'] = [
          {
            id: 'proj-redesign-knowledge-artifact',
            workspaceId: state.workspace.id,
            projectId: 'proj-redesign',
            title: 'Promoted Runtime Summary',
            summary: 'Promoted from Runtime Delivery Summary.',
            kind: 'shared',
            status: 'shared',
            sourceType: 'artifact',
            sourceRef: 'artifact-run-conv-redesign',
            updatedAt: 105,
          },
          ...(state.projectKnowledge['proj-redesign'] ?? []),
        ]
      },
    })
    await router.push('/workspaces/ws-local/projects/proj-redesign/knowledge')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders project knowledge from the workspace API fixture', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Desktop Redesign Notes')

    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Desktop Redesign Notes')
    expect(mounted.container.textContent).toContain('Knowledge entries scoped to Desktop Redesign.')

    mounted.destroy()
  })

  it('shows the project knowledge empty state when search has no matches', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Desktop Redesign Notes')

    const searchInput = mounted.container.querySelector<HTMLInputElement>('input')
    expect(searchInput).not.toBeNull()

    searchInput!.value = '不存在的关键字'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain(String(i18n.global.t('knowledge.empty.projectTitle')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('knowledge.empty.projectDescription')))

    mounted.destroy()
  })

  it('shows a personal section on the workspace knowledge page and hides another user personal knowledge', async () => {
    installWorkspaceApiFixture({
      stateTransform(state) {
        state.workspaceKnowledge = [
          createKnowledgeRecord({
            id: 'knowledge-workspace-shared',
            title: 'Workspace Protocol Baseline',
            summary: 'Shared workspace operations.',
            sourceRef: 'workspace-handbook',
            scope: 'workspace',
            visibility: 'public',
          }),
          createKnowledgeRecord({
            id: 'knowledge-personal-owner',
            title: 'Owner Personal Playbook',
            summary: 'Private owner guidance.',
            sourceRef: 'owner-playbook',
            scope: 'personal',
            visibility: 'private',
            ownerUserId: 'user-owner',
          }),
          createKnowledgeRecord({
            id: 'knowledge-personal-other',
            title: 'Operator Personal Notes',
            summary: 'Should not be visible to the owner.',
            sourceRef: 'operator-notes',
            scope: 'personal',
            visibility: 'private',
            ownerUserId: 'user-operator',
          }),
          createKnowledgeRecord({
            id: 'knowledge-project-redesign',
            title: 'Desktop Redesign Notes',
            summary: 'Project scoped notes.',
            sourceRef: 'proj-redesign-notes',
            scope: 'project',
            projectId: 'proj-redesign',
            visibility: 'public',
          }),
        ]
      },
    })
    await router.push('/workspaces/ws-local/console/knowledge')
    await router.isReady()

    const mounted = mountApp()

    await waitForText(mounted.container, 'Owner Personal Playbook')

    expect(mounted.container.textContent).toContain(String(i18n.global.t('knowledge.workspaceSections.personal')))
    expect(mounted.container.textContent).toContain('Owner Personal Playbook')
    expect(mounted.container.textContent).not.toContain('Operator Personal Notes')

    mounted.destroy()
  })

  it('supports personal, project, and workspace scope filtering on the project knowledge page', async () => {
    installWorkspaceApiFixture({
      stateTransform(state) {
        state.workspaceKnowledge = [
          createKnowledgeRecord({
            id: 'knowledge-workspace-shared',
            title: 'Workspace Protocol Baseline',
            summary: 'Shared workspace operations.',
            sourceRef: 'workspace-handbook',
            scope: 'workspace',
            visibility: 'public',
          }),
          createKnowledgeRecord({
            id: 'knowledge-personal-owner',
            title: 'Owner Personal Playbook',
            summary: 'Private owner guidance.',
            sourceRef: 'owner-playbook',
            scope: 'personal',
            visibility: 'private',
            ownerUserId: 'user-owner',
          }),
        ]
        state.projectKnowledge['proj-redesign'] = [
          createKnowledgeRecord({
            id: 'knowledge-project-redesign',
            title: 'Desktop Redesign Notes',
            summary: 'Project scoped notes.',
            sourceRef: 'proj-redesign-notes',
            scope: 'project',
            projectId: 'proj-redesign',
            visibility: 'public',
          }),
        ]
      },
    })
    await router.push('/workspaces/ws-local/projects/proj-redesign/knowledge')
    await router.isReady()

    const mounted = mountApp()

    await waitForText(mounted.container, 'Workspace Protocol Baseline')
    await waitForText(mounted.container, 'Owner Personal Playbook')
    await waitForText(mounted.container, 'Desktop Redesign Notes')

    const scopeFilter = mounted.container.querySelector<HTMLSelectElement>('[data-testid="project-knowledge-scope-filter"]')
    expect(scopeFilter).not.toBeNull()

    scopeFilter!.value = 'personal'
    scopeFilter!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('Owner Personal Playbook')
    expect(mounted.container.textContent).not.toContain('Workspace Protocol Baseline')
    expect(mounted.container.textContent).not.toContain('Desktop Redesign Notes')

    scopeFilter!.value = 'workspace'
    scopeFilter!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('Workspace Protocol Baseline')
    expect(mounted.container.textContent).not.toContain('Owner Personal Playbook')
    expect(mounted.container.textContent).not.toContain('Desktop Redesign Notes')

    scopeFilter!.value = 'project'
    scopeFilter!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('Desktop Redesign Notes')
    expect(mounted.container.textContent).not.toContain('Workspace Protocol Baseline')
    expect(mounted.container.textContent).not.toContain('Owner Personal Playbook')

    mounted.destroy()
  })

  it('links artifact-sourced knowledge entries back to the project deliverables surface', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Promoted Runtime Summary')

    const sourceLink = mounted.container.querySelector<HTMLAnchorElement>('[data-testid="knowledge-source-link-proj-redesign-knowledge-artifact"]')
    expect(sourceLink).not.toBeNull()
    expect(sourceLink?.getAttribute('href')).toContain('/workspaces/ws-local/projects/proj-redesign/deliverables')
    expect(sourceLink?.getAttribute('href')).toContain('deliverable=artifact-run-conv-redesign')

    mounted.destroy()
  })
})
