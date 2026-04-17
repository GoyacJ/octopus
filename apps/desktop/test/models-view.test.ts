// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'
import type { ModelCatalogSnapshot, RuntimeConfigPatch, RuntimeConfigValidationResult } from '@octopus/schema'

import ModelsView from '@/views/workspace/ModelsView.vue'
import i18n from '@/plugins/i18n'
import { useNotificationStore } from '@/stores/notifications'
import { useShellStore } from '@/stores/shell'
import type { WorkspaceClient } from '@/tauri/workspace-client'
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

async function mountView() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp(ModelsView)
  app.use(pinia)
  app.use(i18n)
  app.mount(container)

  const shellStore = useShellStore()
  await shellStore.bootstrap('ws-local', 'proj-redesign')
  await nextTick()

  return {
    app,
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000, label = 'condition') {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for ${label}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

async function waitForText(container: HTMLElement, value: string, timeoutMs = 2000) {
  await waitFor(() => container.textContent?.includes(value) ?? false, timeoutMs, `text: ${value}`)
}

function modelRows(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>('[data-testid^="models-table-row-"]'))
}

function configureWorkspaceClient(
  transform: (client: WorkspaceClient) => WorkspaceClient,
) {
  const createWorkspaceClientMock = vi.mocked(tauriClient.createWorkspaceClient)
  const baseImplementation = createWorkspaceClientMock.getMockImplementation()
  expect(baseImplementation).toBeTypeOf('function')

  createWorkspaceClientMock.mockImplementation((context) =>
    transform(baseImplementation!(context) as WorkspaceClient) as ReturnType<typeof tauriClient.createWorkspaceClient>,
  )
}

function setSelectValue(container: HTMLElement, testId: string, value: string) {
  const element = container.querySelector<HTMLSelectElement>(`[data-testid="${testId}"]`)
  expect(element).not.toBeNull()
  element!.value = value
  element!.dispatchEvent(new Event('change', { bubbles: true }))
}

function setInputValue(container: HTMLElement, testId: string, value: string) {
  const element = container.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[data-testid="${testId}"]`)
  expect(element).not.toBeNull()
  element!.value = value
  element!.dispatchEvent(new Event('input', { bubbles: true }))
  element!.dispatchEvent(new Event('change', { bubbles: true }))
}

function overrideWorkspaceRuntimeConfig(
  documentPatch: Record<string, unknown>,
) {
  configureWorkspaceClient((client) => ({
    ...client,
    runtime: {
      ...client.runtime,
      async getConfig() {
        const config = await client.runtime.getConfig()
        return {
          ...config,
          sources: config.sources.map(source => source.scope === 'workspace'
            ? {
                ...source,
                document: {
                  ...(source.document ?? {}),
                  ...documentPatch,
                },
              }
            : source),
        }
      },
    },
  }))
}

describe('Models view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('shows only workspace-created model instances and hides legacy-only catalog entries', async () => {
    const mounted = await mountView()

    await waitForText(mounted.container, '工作区模型中心')
    await waitFor(() => modelRows(mounted.container).length === 0, 2000, 'empty model rows')

    expect(mounted.container.textContent).toContain('还没有创建模型')
    expect(mounted.container.textContent).not.toContain('工作区默认绑定')
    expect(mounted.container.textContent).not.toContain('注册表诊断')

    mounted.destroy()
  })

  it('renders explicit workspace models with pagination and structured filters', async () => {
    overrideWorkspaceRuntimeConfig({
      configuredModels: Object.fromEntries(Array.from({ length: 12 }, (_, index) => {
        const configuredModelId = `openai-gpt4o-${index + 1}`
        return [configuredModelId, {
          configuredModelId,
          name: `GPT-4o Workspace ${index + 1}`,
          providerId: 'openai',
          modelId: 'gpt-4o',
          credentialRef: `env:OPENAI_KEY_${index + 1}`,
          tokenQuota: {
            totalTokens: 1000 + index,
          },
          enabled: true,
          source: 'workspace',
        }]
      })),
    })

    const mounted = await mountView()

    await waitForText(mounted.container, 'GPT-4o Workspace 1')
    await waitFor(() => modelRows(mounted.container).length === 10, 2000, 'first page configured model rows')

    expect(mounted.container.querySelector('[data-testid="models-pagination"]')?.textContent).toContain('1 / 2')
    expect(mounted.container.textContent).toContain('已消耗 Token')
    expect(mounted.container.textContent).toContain('Token 总量')
    expect(mounted.container.textContent).toContain('0')
    expect(mounted.container.textContent).toContain('1,000')

    setInputValue(mounted.container, 'models-search-input', 'workspace 11')
    await waitFor(() => modelRows(mounted.container).length === 1, 2000, 'search filtered rows')
    expect(mounted.container.textContent).toContain('GPT-4o Workspace 11')

    setInputValue(mounted.container, 'models-search-input', '')
    setSelectValue(mounted.container, 'models-provider-filter', 'openai')
    await waitFor(() => modelRows(mounted.container).length === 10, 2000, 'provider filtered rows')

    const nextPageButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="models-pagination"] button:last-of-type')
    expect(nextPageButton).not.toBeNull()
    nextPageButton?.click()
    await waitForText(mounted.container, 'GPT-4o Workspace 11')

    mounted.destroy()
  })

  it('switches create dialog fields for standard, custom, and ollama providers', async () => {
    configureWorkspaceClient((client) => ({
      ...client,
      catalog: {
        ...client.catalog,
        async getSnapshot() {
          const snapshot = await client.catalog.getSnapshot()
          return {
            ...snapshot,
            providers: [
              ...snapshot.providers,
              {
                providerId: 'ollama',
                label: 'Ollama',
                enabled: true,
                surfaces: [
                  {
                    surface: 'conversation',
                    protocolFamily: 'openai_chat',
                    transport: ['request_response', 'sse'],
                    authStrategy: 'bearer',
                    baseUrl: 'http://127.0.0.1:11434/v1',
                    baseUrlPolicy: 'allow_override',
                    enabled: true,
                    capabilities: [],
                  },
                ],
                metadata: {},
              },
            ],
          } satisfies ModelCatalogSnapshot
        },
      },
    }))

    const mounted = await mountView()

    await waitForText(mounted.container, '工作区模型中心')

    const createButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="models-create-button"]')
    expect(createButton).not.toBeNull()
    createButton?.click()

    await waitFor(() => document.body.querySelector('[data-testid="models-create-dialog"]') !== null, 2000, 'create model dialog')
    expect(document.body.querySelector('[data-testid="models-create-provider-select"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="models-create-name-input"]')).not.toBeNull()

    setSelectValue(document.body, 'models-create-provider-select', 'custom')
    await nextTick()
    expect(document.body.querySelector('[data-testid="models-create-custom-provider-name-input"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="models-create-upstream-model-input"]')).not.toBeNull()

    setSelectValue(document.body, 'models-create-provider-select', 'ollama')
    await nextTick()
    expect(document.body.querySelector('[data-testid="models-create-custom-provider-name-input"]')).toBeNull()
    expect(document.body.querySelector('[data-testid="models-create-upstream-model-input"]')).not.toBeNull()

    mounted.destroy()
  })

  it('creates custom models by auto-saving configuredModels and modelRegistry patches', async () => {
    const validateSpy = vi.fn(async (patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> => {
      const providers = (patch.patch.modelRegistry as Record<string, any> | undefined)?.providers as Record<string, any> | undefined
      const customProvider = providers && Object.values(providers)[0]
      return {
        valid: true,
        errors: [],
        warnings: customProvider?.surfaces?.[0]?.baseUrl === 'https://api.example.com/v1'
          ? ['custom provider is still using the placeholder base URL']
          : [],
      }
    })

    const saveSpy = vi.fn()

    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async validateConfig(patch) {
          return await validateSpy(patch)
        },
        async saveConfig(patch) {
          saveSpy(patch)
          return await client.runtime.saveConfig(patch)
        },
      },
    }))

    const mounted = await mountView()

    await waitForText(mounted.container, '工作区模型中心')

    const createButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="models-create-button"]')
    expect(createButton).not.toBeNull()
    createButton?.click()

    await waitFor(() => document.body.querySelector('[data-testid="models-create-dialog"]') !== null, 2000, 'create model dialog')
    setInputValue(document.body, 'models-create-name-input', 'Local Gateway Model')
    setSelectValue(document.body, 'models-create-provider-select', 'custom')
    await nextTick()
    setInputValue(document.body, 'models-create-custom-provider-name-input', 'Local Gateway')
    setInputValue(document.body, 'models-create-upstream-model-input', 'gateway-chat')
    setInputValue(document.body, 'models-create-base-url-input', 'https://gateway.example.test/v1')
    setInputValue(document.body, 'models-create-total-tokens-input', '2048')

    const confirmButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button')).at(-1)
    expect(confirmButton).not.toBeUndefined()
    confirmButton?.click()

    await waitFor(() => saveSpy.mock.calls.length === 1, 2000, 'auto save call')
    await waitForText(mounted.container, 'Local Gateway Model')
    await waitFor(() => document.body.querySelector('[data-testid="models-detail-dialog"]') !== null, 2000, 'detail dialog')
    await waitFor(() => document.body.querySelector('[data-testid="models-detail-panel"]') !== null, 2000, 'detail panel')
    expect(validateSpy.mock.calls.length).toBeGreaterThanOrEqual(1)

    const savedPatch = saveSpy.mock.calls[0]?.[0]
    expect(savedPatch?.scope).toBe('workspace')

    const configuredModelsPatch = savedPatch?.patch.configuredModels as Record<string, any>
    const configuredModelEntry = Object.values(configuredModelsPatch)[0]
    expect(configuredModelEntry).toMatchObject({
      name: 'Local Gateway Model',
      providerId: expect.stringMatching(/^custom-/),
      modelId: expect.stringMatching(/^custom-.*::gateway-chat$/),
      baseUrl: 'https://gateway.example.test/v1',
      tokenQuota: {
        totalTokens: 2048,
      },
      source: 'workspace',
    })
    expect(configuredModelEntry.tokenUsage).toBeUndefined()

    const providerPatch = (savedPatch?.patch.modelRegistry as Record<string, any>)?.providers as Record<string, any>
    const providerEntry = Object.values(providerPatch)[0]
    expect(providerEntry).toMatchObject({
      label: 'Local Gateway',
    })

    const modelPatch = (savedPatch?.patch.modelRegistry as Record<string, any>)?.models as Record<string, any>
    const modelEntry = Object.values(modelPatch)[0]
    expect(modelEntry).toMatchObject({
      label: 'gateway-chat',
    })

    mounted.destroy()
  })

  it('runs a real configured model probe when clicking validate', async () => {
    overrideWorkspaceRuntimeConfig({
      configuredModels: {
        'anthropic-primary': {
          configuredModelId: 'anthropic-primary',
          name: 'Claude Primary',
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4-5',
          credentialRef: 'env:ANTHROPIC_API_KEY',
          baseUrl: 'https://anthropic.example.test',
          enabled: true,
          source: 'workspace',
        },
      },
    })

    const probeSpy = vi.fn(async (input: Record<string, unknown>) => ({
      valid: true,
      reachable: true,
      configuredModelId: 'anthropic-primary',
      configuredModelName: 'Claude Primary',
      requestId: 'probe-request-1',
      consumedTokens: 12,
      errors: [],
      warnings: [],
    }))

    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async validateConfiguredModel(input) {
          return await probeSpy(input as Record<string, unknown>)
        },
      },
    }))

    const mounted = await mountView()

    await waitForText(mounted.container, 'Claude Primary')
    modelRows(mounted.container)[0]?.click()
    await waitFor(() => document.body.querySelector('[data-testid="models-detail-dialog"]') !== null, 2000, 'detail dialog')

    setInputValue(document.body, 'models-detail-base-url', 'https://anthropic.alt.example.test')
    const validateButton = document.body.querySelector<HTMLButtonElement>('[data-testid="models-validate-button"]')
    expect(validateButton).not.toBeNull()
    validateButton?.click()

    await waitFor(() => probeSpy.mock.calls.length === 1, 2000, 'configured model probe call')
    expect(probeSpy.mock.calls[0]?.[0]).toMatchObject({
      scope: 'workspace',
      configuredModelId: 'anthropic-primary',
      patch: {
        configuredModels: {
          'anthropic-primary': {
            baseUrl: 'https://anthropic.alt.example.test',
          },
        },
      },
    })
    await waitFor(() => document.body.textContent?.includes('已完成真实请求校验') ?? false, 2000, 'probe success message')

    mounted.destroy()
  })

  it('shows ApiKey copy and masks configured credential values across the models surface', async () => {
    overrideWorkspaceRuntimeConfig({
      configuredModels: {
        'openai-primary': {
          configuredModelId: 'openai-primary',
          name: 'GPT-4o Primary',
          providerId: 'openai',
          modelId: 'gpt-4o',
          credentialRef: 'env:OPENAI_API_KEY',
          enabled: true,
          source: 'workspace',
        },
      },
    })

    const mounted = await mountView()

    await waitForText(mounted.container, 'GPT-4o Primary')
    expect(mounted.container.textContent).toContain('ApiKey')
    expect(mounted.container.textContent).not.toContain('凭据引用')
    expect(mounted.container.textContent).not.toContain('env:OPENAI_API_KEY')

    modelRows(mounted.container)[0]?.click()
    await waitFor(() => document.body.querySelector('[data-testid="models-detail-dialog"]') !== null, 2000, 'detail dialog')
    expect(document.body.textContent).toContain('ApiKey')
    expect(document.body.textContent).not.toContain('env:OPENAI_API_KEY')

    const createButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="models-create-button"]')
    expect(createButton).not.toBeNull()
    createButton?.click()
    await waitFor(() => document.body.querySelector('[data-testid="models-create-dialog"]') !== null, 2000, 'create dialog')
    expect(document.body.textContent).toContain('ApiKey')

    mounted.destroy()
  })

  it('routes validate success through workspace notifications instead of inline-only feedback', async () => {
    overrideWorkspaceRuntimeConfig({
      configuredModels: {
        'anthropic-primary': {
          configuredModelId: 'anthropic-primary',
          name: 'Claude Primary',
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4-5',
          credentialRef: 'env:ANTHROPIC_API_KEY',
          enabled: true,
          source: 'workspace',
        },
      },
    })

    const probeSpy = vi.fn(async () => ({
      valid: true,
      reachable: true,
      configuredModelId: 'anthropic-primary',
      configuredModelName: 'Claude Primary',
      requestId: 'probe-request-2',
      consumedTokens: 9,
      errors: [],
      warnings: [],
    }))

    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async validateConfiguredModel(input) {
          return await probeSpy(input as Record<string, unknown>)
        },
      },
    }))

    const mounted = await mountView()

    await waitForText(mounted.container, 'Claude Primary')
    modelRows(mounted.container)[0]?.click()
    await waitFor(() => document.body.querySelector('[data-testid="models-detail-dialog"]') !== null, 2000, 'detail dialog')

    const notificationStore = useNotificationStore()
    const validateButton = document.body.querySelector<HTMLButtonElement>('[data-testid="models-validate-button"]')
    expect(validateButton).not.toBeNull()
    validateButton?.click()

    await waitFor(() => probeSpy.mock.calls.length === 1, 2000, 'configured model probe call')
    await waitFor(() => notificationStore.notificationsState.length > 0, 2000, 'validation toast')
    expect(notificationStore.notificationsState.some(notification =>
      notification.scopeKind === 'workspace'
        && notification.title.includes('校验'),
    )).toBe(true)

    mounted.destroy()
  })

  it('stores a replacement ApiKey through managed secret upsert before saving and emits a save notification', async () => {
    overrideWorkspaceRuntimeConfig({
      configuredModels: {
        'anthropic-primary': {
          configuredModelId: 'anthropic-primary',
          name: 'Claude Primary',
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4-5',
          credentialRef: 'env:ANTHROPIC_API_KEY',
          enabled: true,
          source: 'workspace',
        },
      },
    })

    const upsertSpy = vi.fn(async () => ({
      configuredModelId: 'anthropic-primary',
      credentialRef: 'secret-ref:fixture:anthropic-primary',
      storageKind: 'os-keyring',
      status: 'configured',
    }))
    const saveSpy = vi.fn()

    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async upsertConfiguredModelCredential(configuredModelId, input) {
          return await upsertSpy(configuredModelId, input)
        },
        async saveConfig(patch) {
          saveSpy(patch)
          return await client.runtime.saveConfig(patch)
        },
      },
    }))

    const mounted = await mountView()

    await waitForText(mounted.container, 'Claude Primary')
    modelRows(mounted.container)[0]?.click()
    await waitFor(() => document.body.querySelector('[data-testid="models-detail-dialog"]') !== null, 2000, 'detail dialog')

    setInputValue(document.body, 'models-detail-credential-ref', 'sk-ant-replacement-secret')
    const saveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="models-save-button"]')
    expect(saveButton).not.toBeNull()
    saveButton?.click()

    await waitFor(() => upsertSpy.mock.calls.length === 1, 2000, 'credential upsert call')
    expect(upsertSpy.mock.calls[0]?.[0]).toBe('anthropic-primary')
    expect(upsertSpy.mock.calls[0]?.[1]).toMatchObject({
      configuredModelId: 'anthropic-primary',
      providerId: 'anthropic',
      apiKey: 'sk-ant-replacement-secret',
    })

    await waitFor(() => saveSpy.mock.calls.length === 1, 2000, 'save config call')
    expect(saveSpy.mock.calls[0]?.[0]).toMatchObject({
      scope: 'workspace',
      patch: {
        configuredModels: {
          'anthropic-primary': {
            credentialRef: 'secret-ref:fixture:anthropic-primary',
          },
        },
      },
    })

    const notificationStore = useNotificationStore()
    await waitFor(() => notificationStore.notificationsState.length > 0, 2000, 'save toast')
    expect(notificationStore.notificationsState.some(notification =>
      notification.scopeKind === 'workspace'
        && notification.title.includes('保存'),
    )).toBe(true)

    mounted.destroy()
  })

  it('stores ApiKey through managed secret upsert before creating a model and emits a create notification', async () => {
    const upsertSpy = vi.fn(async (configuredModelId: string) => ({
      configuredModelId,
      credentialRef: `secret-ref:fixture:${configuredModelId}`,
      storageKind: 'os-keyring',
      status: 'configured',
    }))
    const saveSpy = vi.fn()

    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async upsertConfiguredModelCredential(configuredModelId, input) {
          return await upsertSpy(configuredModelId, input)
        },
        async saveConfig(patch) {
          saveSpy(patch)
          return await client.runtime.saveConfig(patch)
        },
      },
    }))

    const mounted = await mountView()

    await waitForText(mounted.container, '工作区模型中心')

    const createButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="models-create-button"]')
    expect(createButton).not.toBeNull()
    createButton?.click()

    await waitFor(() => document.body.querySelector('[data-testid="models-create-dialog"]') !== null, 2000, 'create dialog')
    setInputValue(document.body, 'models-create-name-input', 'Managed GPT-4o')
    setSelectValue(document.body, 'models-create-provider-select', 'openai')
    await nextTick()
    setSelectValue(document.body, 'models-create-upstream-model-select', 'gpt-4o')
    setInputValue(document.body, 'models-create-credential-ref-input', 'sk-openai-managed-secret')

    const confirmButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button')).at(-1)
    expect(confirmButton).not.toBeUndefined()
    confirmButton?.click()

    await waitFor(() => upsertSpy.mock.calls.length === 1, 2000, 'create credential upsert')
    expect(upsertSpy.mock.calls[0]?.[1]).toMatchObject({
      providerId: 'openai',
      apiKey: 'sk-openai-managed-secret',
    })

    await waitFor(() => saveSpy.mock.calls.length === 1, 2000, 'create save config call')
    const configuredModelsPatch = saveSpy.mock.calls[0]?.[0]?.patch?.configuredModels as Record<string, Record<string, unknown>>
    const createdEntry = Object.values(configuredModelsPatch ?? {})[0]
    expect(createdEntry?.credentialRef).toMatch(/^secret-ref:fixture:/)
    expect(JSON.stringify(createdEntry)).not.toContain('sk-openai-managed-secret')

    const notificationStore = useNotificationStore()
    await waitFor(() => notificationStore.notificationsState.length > 0, 2000, 'create toast')
    expect(notificationStore.notificationsState.some(notification =>
      notification.scopeKind === 'workspace'
        && notification.title.includes('创建'),
    )).toBe(true)

    mounted.destroy()
  })

  it('deletes the configured model and best-effort managed secret through a workspace notification workflow', async () => {
    overrideWorkspaceRuntimeConfig({
      configuredModels: {
        'anthropic-primary': {
          configuredModelId: 'anthropic-primary',
          name: 'Claude Primary',
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4-5',
          credentialRef: 'secret-ref:fixture:anthropic-primary',
          enabled: true,
          source: 'workspace',
        },
      },
    })

    const deleteCredentialSpy = vi.fn(async () => {})

    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async deleteConfiguredModelCredential(configuredModelId) {
          deleteCredentialSpy(configuredModelId)
          await client.runtime.deleteConfiguredModelCredential(configuredModelId)
        },
      },
    }))

    const mounted = await mountView()

    await waitForText(mounted.container, 'Claude Primary')
    modelRows(mounted.container)[0]?.click()
    await waitFor(() => document.body.querySelector('[data-testid="models-detail-dialog"]') !== null, 2000, 'detail dialog')

    const deleteButton = document.body.querySelector<HTMLButtonElement>('[data-testid="models-delete-button"]')
    expect(deleteButton).not.toBeNull()
    deleteButton?.click()

    await waitFor(() => deleteCredentialSpy.mock.calls.length === 1, 2000, 'delete credential cleanup call')
    expect(deleteCredentialSpy).toHaveBeenCalledWith('anthropic-primary')

    const notificationStore = useNotificationStore()
    await waitFor(() => notificationStore.notificationsState.length > 0, 2000, 'delete toast')
    expect(notificationStore.notificationsState.some(notification =>
      notification.scopeKind === 'workspace'
        && notification.title.includes('删除'),
    )).toBe(true)

    mounted.destroy()
  })
})
