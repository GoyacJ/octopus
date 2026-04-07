// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
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

async function waitForTextToDisappear(container: HTMLElement, value: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (container.textContent?.includes(value) ?? false) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for text to disappear: ${value}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

function findButton(container: ParentNode, label: string) {
  return Array.from(container.querySelectorAll<HTMLButtonElement>('button'))
    .find(button => button.textContent?.includes(label))
}

function findTabButton(container: ParentNode, label: string) {
  return Array.from(container.querySelectorAll<HTMLButtonElement>('button'))
    .find(button => button.textContent?.includes(label))
}

function findSkillCopyInput(container: ParentNode, skillId: string) {
  return container
    .querySelector<HTMLElement>(`[data-testid="tools-skill-action-copy-item-${skillId}"]`)
    ?.querySelector<HTMLInputElement>('input') ?? null
}

describe('Workspace tools view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/tools')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders workspace tools from the real catalog store', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'bash')

    expect(mounted.container.textContent).toContain(String(i18n.global.t('sidebar.navigation.tools')))
    expect(mounted.container.textContent).toContain('bash')

    const skillTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.skill')))
    expect(skillTab).toBeDefined()
    skillTab!.click()
    await waitForText(mounted.container, 'help')

    const mcpTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.mcp')))
    expect(mcpTab).toBeDefined()
    mcpTab!.click()
    await waitForText(mounted.container, 'mcp__ops__tail_logs')

    expect(mounted.container.textContent).toContain('ops')
    expect(mounted.container.textContent).toContain('MCP handshake timed out')

    mounted.destroy()
  })

  it('filters runtime-backed entries and exposes builtin disable management without edit actions', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'bash')

    const searchInput = mounted.container.querySelector<HTMLInputElement>('input')
    expect(searchInput).not.toBeNull()
    searchInput!.value = 'bash'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('bash')
    expect(mounted.container.textContent).not.toContain('help')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('tools.actions.disable')))
    expect(findButton(mounted.container, String(i18n.global.t('common.save')))).toBeUndefined()
    expect(findButton(mounted.container, String(i18n.global.t('common.delete')))).toBeUndefined()

    mounted.destroy()
  })

  it('shows editable actions for workspace-owned skill entries', async () => {
    const mounted = mountApp()

    const skillTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.skill')))
    expect(skillTab).toBeDefined()
    skillTab!.click()
    await waitForText(mounted.container, 'help')

    expect(findButton(mounted.container, String(i18n.global.t('tools.actions.newSkill')))).toBeDefined()
    expect(findButton(mounted.container, String(i18n.global.t('tools.actions.importSkill')))).toBeDefined()
    await waitForText(mounted.container, 'notes/guide.md')
    await waitForText(mounted.container, 'assets/logo.png')
    expect(findButton(mounted.container, String(i18n.global.t('common.save')))).toBeDefined()
    expect(findButton(mounted.container, String(i18n.global.t('common.delete')))).toBeDefined()
    expect(mounted.container.textContent).toContain(String(i18n.global.t('tools.actions.disable')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('tools.states.managed')))
    const managedCard = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-skill-workspace-help"]')
    expect(managedCard?.textContent).not.toContain(String(i18n.global.t('tools.states.readonly')))

    mounted.destroy()
  })

  it('shows copy management for external skills without edit and delete actions', async () => {
    const mounted = mountApp()

    const skillTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.skill')))
    expect(skillTab).toBeDefined()
    skillTab!.click()
    await waitForText(mounted.container, 'external-help')

    const externalCard = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-skill-external-help"]')
    expect(externalCard).toBeDefined()
    expect(externalCard?.textContent).toContain(String(i18n.global.t('tools.states.readonly')))
    externalCard?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await waitForText(mounted.container, 'examples/prompt.txt')

    expect(findButton(mounted.container, String(i18n.global.t('tools.actions.copyToManaged')))).toBeDefined()
    expect(findButton(mounted.container, String(i18n.global.t('common.delete')))).toBeUndefined()
    expect(mounted.container.textContent).toContain('examples/prompt.txt')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('tools.states.readonly')))

    mounted.destroy()
  })

  it('opens a dialog copy flow for external skills and completes the managed copy', async () => {
    const mounted = mountApp()

    const skillTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.skill')))
    expect(skillTab).toBeDefined()
    skillTab!.click()
    await waitForText(mounted.container, 'external-help')

    const externalCard = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-skill-external-help"]')
    expect(externalCard).toBeDefined()
    externalCard?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await waitForText(mounted.container, 'examples/prompt.txt')

    const copyButton = findButton(mounted.container, String(i18n.global.t('tools.actions.copyToManaged')))
    expect(copyButton).toBeDefined()
    copyButton!.click()

    const copyDescription = String(i18n.global.t('tools.editor.copySkillDescription'))
    await waitForText(document.body, copyDescription)
    const dialog = document.body.querySelector<HTMLElement>('[data-testid="tools-skill-action-dialog"]')
    expect(dialog).toBeDefined()
    const nameInput = findSkillCopyInput(document.body, 'skill-external-help')
    expect(nameInput).not.toBeNull()
    expect(nameInput?.value).toBe('external-help')
    nameInput!.value = 'external-help-copy'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))

    const confirmButton = findButton(document.body, String(i18n.global.t('common.confirm')))
    expect(confirmButton).toBeDefined()
    confirmButton!.click()

    await waitForTextToDisappear(document.body, copyDescription)
    await waitForText(mounted.container, 'data/skills/external-help-copy/SKILL.md')
    await waitForText(mounted.container, String(i18n.global.t('tools.states.managed')))
    const copiedCard = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-skill-workspace-external-help-copy"]')
    expect(copiedCard?.textContent).not.toContain(String(i18n.global.t('tools.states.readonly')))

    mounted.destroy()
  })

  it('supports multi-select copy for external skills', async () => {
    const mounted = mountApp()

    const skillTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.skill')))
    expect(skillTab).toBeDefined()
    skillTab!.click()
    await waitForText(mounted.container, 'external-help')
    await waitForText(mounted.container, 'external-checks')

    const firstCheckbox = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-select-skill-external-help"]')
    expect(firstCheckbox).not.toBeNull()
    firstCheckbox?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    const secondCheckbox = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-select-skill-external-checks"]')
    expect(secondCheckbox).not.toBeNull()
    secondCheckbox?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    const batchCopyButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="tools-copy-selected-skills-button"]')
    expect(batchCopyButton).not.toBeNull()
    expect(batchCopyButton?.textContent).toContain('2')
    batchCopyButton!.click()

    const copyDescription = String(i18n.global.t('tools.editor.copySkillDescription'))
    await waitForText(document.body, copyDescription)
    const firstNameInput = findSkillCopyInput(document.body, 'skill-external-help')
    const secondNameInput = findSkillCopyInput(document.body, 'skill-external-checks')
    expect(firstNameInput).not.toBeNull()
    expect(secondNameInput).not.toBeNull()
    expect(firstNameInput?.value).toBe('external-help')
    expect(secondNameInput?.value).toBe('external-checks')

    firstNameInput!.value = 'external-help-batch'
    firstNameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    secondNameInput!.value = 'external-checks-batch'
    secondNameInput!.dispatchEvent(new Event('input', { bubbles: true }))

    const confirmButton = findButton(document.body, String(i18n.global.t('common.confirm')))
    expect(confirmButton).toBeDefined()
    confirmButton!.click()

    await waitForTextToDisappear(document.body, copyDescription)
    await waitForText(mounted.container, 'data/skills/external-help-batch/SKILL.md')
    await waitForText(mounted.container, 'data/skills/external-checks-batch/SKILL.md')

    const copiedHelpCard = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-skill-workspace-external-help-batch"]')
    const copiedChecksCard = mounted.container.querySelector<HTMLElement>('[data-testid="tool-entry-skill-workspace-external-checks-batch"]')
    expect(copiedHelpCard?.textContent).not.toContain(String(i18n.global.t('tools.states.readonly')))
    expect(copiedChecksCard?.textContent).not.toContain(String(i18n.global.t('tools.states.readonly')))

    mounted.destroy()
  })

  it('imports multiple skill archives with default names from filenames', async () => {
    vi.spyOn(tauriClient, 'pickSkillArchive').mockResolvedValue([
      {
        fileName: 'alpha-skill.zip',
        contentType: 'application/zip',
        dataBase64: 'UEsDBAoAAAAAA',
        byteSize: 12,
      },
      {
        fileName: 'beta-skill.zip',
        contentType: 'application/zip',
        dataBase64: 'UEsDBAoAAAAAB',
        byteSize: 12,
      },
    ])

    const mounted = mountApp()

    const skillTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.skill')))
    expect(skillTab).toBeDefined()
    skillTab!.click()
    await waitForText(mounted.container, 'help')

    const importButton = findButton(mounted.container, String(i18n.global.t('tools.actions.importSkill')))
    expect(importButton).toBeDefined()
    importButton!.click()

    await waitForText(document.body, String(i18n.global.t('tools.editor.importSkillDescription')))

    const importArchiveButton = findButton(document.body, String(i18n.global.t('tools.actions.importArchive')))
    expect(importArchiveButton).toBeDefined()
    importArchiveButton!.click()
    await waitForText(document.body, 'alpha-skill')
    await waitForText(document.body, 'beta-skill')

    const confirmButton = findButton(document.body, String(i18n.global.t('common.confirm')))
    expect(confirmButton).toBeDefined()
    confirmButton!.click()

    await waitForText(mounted.container, 'data/skills/alpha-skill/SKILL.md')
    await waitForText(mounted.container, 'data/skills/beta-skill/SKILL.md')
    expect(document.body.querySelector('[data-testid="tools-skill-action-slug-input"]')).toBeNull()

    mounted.destroy()
  })

  it('shows editable actions for MCP entries instead of linking to runtime settings', async () => {
    const mounted = mountApp()

    const mcpTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.mcp')))
    expect(mcpTab).toBeDefined()
    mcpTab!.click()
    await waitForText(mounted.container, 'mcp__ops__tail_logs')

    expect(findButton(mounted.container, String(i18n.global.t('tools.actions.newMcp')))).toBeDefined()
    expect(findButton(mounted.container, String(i18n.global.t('common.save')))).toBeDefined()
    expect(findButton(mounted.container, String(i18n.global.t('common.delete')))).toBeDefined()

    const settingsLink = Array.from(mounted.container.querySelectorAll<HTMLAnchorElement>('a'))
      .find(link => link.getAttribute('href') === '/settings')
    expect(settingsLink).toBeUndefined()

    mounted.destroy()
  })
})
