import type {
  ResolveRuntimeApprovalInput,
  RuntimeConfigScope,
  RuntimeConfigValidationResult,
  RuntimeConfiguredModelProbeResult,
  RuntimeDecisionAction,
  RuntimeEffectiveConfig,
  RuntimeSessionDetail,
  SubmitRuntimeTurnInput,
} from '@octopus/schema'

import * as tauriClient from '@/tauri/client'

import {
  createRuntimeConfigDraftsFromConfig,
  createRuntimeConfigValidationState,
  parseRuntimeConfigDraft,
} from './runtime-config'
import {
  createOptimisticAssistantMessage,
  createOptimisticRuntimeMessage,
} from './runtime_messages'
import { isBusyStatus } from './runtime_sessions'

export const runtimeStoreActions = {
  async loadWorkspaceConfig(this: any, force = false): Promise<RuntimeEffectiveConfig | null> {
    return this.loadConfig(force)
  },
  setConfigDraft(this: any, scope: RuntimeConfigScope, value: string) {
    this.configDrafts = {
      ...this.configDrafts,
      [scope]: value,
    }
    this.saveActiveWorkspaceSnapshot()
  },
  clearConfiguredModelProbeResult(this: any) {
    this.configuredModelProbeResult = null
    this.saveActiveWorkspaceSnapshot()
  },
  async loadConfig(this: any, force = false): Promise<RuntimeEffectiveConfig | null> {
    this.syncWorkspaceScopeFromShell()
    if (this.config && !force) {
      return this.config
    }

    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { connectionId, client } = resolvedClient

    this.configLoading = true
    this.configError = ''

    try {
      const config = await client.runtime.getConfig()
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return null
      }

      this.config = config
      this.configDrafts = createRuntimeConfigDraftsFromConfig(config)
      this.configValidation = createRuntimeConfigValidationState()
      this.saveActiveWorkspaceSnapshot()
      return config
    } catch (error) {
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configError = error instanceof Error ? error.message : 'Failed to load runtime config'
      }
      return null
    } finally {
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configLoading = false
      }
    }
  },
  async validateConfig(this: any, scope: RuntimeConfigScope): Promise<RuntimeConfigValidationResult> {
    if (scope !== 'workspace') {
      return {
        valid: false,
        errors: ['Settings only supports workspace runtime configuration'],
        warnings: [],
      }
    }

    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return {
        valid: false,
        errors: ['No active workspace connection selected'],
        warnings: [],
      }
    }
    const { connectionId, client } = resolvedClient

    this.configValidating = true
    this.configError = ''

    let patch
    try {
      patch = parseRuntimeConfigDraft(scope, this.configDrafts[scope])
    } catch (error) {
      const result = {
        valid: false,
        errors: [error instanceof Error ? error.message : `Invalid ${scope} runtime config`],
        warnings: [],
      } satisfies RuntimeConfigValidationResult
      this.configValidation = {
        ...this.configValidation,
        [scope]: result,
      }
      this.configError = result.errors[0] ?? ''
      this.configValidating = false
      this.saveActiveWorkspaceSnapshot()
      return result
    }

    try {
      const result = await client.runtime.validateConfig(patch)
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return result
      }

      this.configValidation = {
        ...this.configValidation,
        [scope]: result,
      }
      this.saveActiveWorkspaceSnapshot()
      return result
    } catch (error) {
      const result = {
        valid: false,
        errors: [error instanceof Error ? error.message : 'Failed to validate runtime config'],
        warnings: [],
      } satisfies RuntimeConfigValidationResult
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configValidation = {
          ...this.configValidation,
          [scope]: result,
        }
        this.configError = result.errors[0] ?? ''
        this.saveActiveWorkspaceSnapshot()
      }
      return result
    } finally {
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configValidating = false
      }
    }
  },
  async probeConfiguredModel(
    this: any,
    scope: RuntimeConfigScope,
    configuredModelId: string,
  ): Promise<RuntimeConfiguredModelProbeResult> {
    if (scope !== 'workspace') {
      return {
        valid: false,
        reachable: false,
        configuredModelId,
        errors: ['Settings only supports workspace runtime configuration'],
        warnings: [],
      }
    }

    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return {
        valid: false,
        reachable: false,
        configuredModelId,
        errors: ['No active workspace connection selected'],
        warnings: [],
      }
    }
    const { connectionId, client } = resolvedClient

    let patch
    try {
      patch = parseRuntimeConfigDraft(scope, this.configDrafts[scope])
    } catch (error) {
      const result = {
        valid: false,
        reachable: false,
        configuredModelId,
        errors: [error instanceof Error ? error.message : `Invalid ${scope} runtime config`],
        warnings: [],
      } satisfies RuntimeConfiguredModelProbeResult
      this.configuredModelProbeResult = result
      this.configValidation = {
        ...this.configValidation,
        [scope]: {
          valid: false,
          errors: result.errors,
          warnings: result.warnings,
        },
      }
      this.configError = result.errors[0] ?? ''
      this.saveActiveWorkspaceSnapshot()
      return result
    }

    this.configuredModelProbing = true
    this.configError = ''
    try {
      const result = await client.runtime.validateConfiguredModel({
        scope,
        configuredModelId,
        patch: patch.patch,
      })
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return result
      }

      this.configuredModelProbeResult = result
      this.configValidation = {
        ...this.configValidation,
        [scope]: {
          valid: result.valid && result.reachable,
          errors: result.errors,
          warnings: result.warnings,
        },
      }
      if (result.errors.length > 0) {
        this.configError = result.errors[0] ?? ''
      }
      this.saveActiveWorkspaceSnapshot()
      return result
    } catch (error) {
      const result = {
        valid: false,
        reachable: false,
        configuredModelId,
        errors: [error instanceof Error ? error.message : 'Failed to validate configured model'],
        warnings: [],
      } satisfies RuntimeConfiguredModelProbeResult
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configuredModelProbeResult = result
        this.configValidation = {
          ...this.configValidation,
          [scope]: {
            valid: false,
            errors: result.errors,
            warnings: result.warnings,
          },
        }
        this.configError = result.errors[0] ?? ''
        this.saveActiveWorkspaceSnapshot()
      }
      return result
    } finally {
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configuredModelProbing = false
      }
    }
  },
  async saveConfig(this: any, scope: RuntimeConfigScope): Promise<RuntimeEffectiveConfig | null> {
    if (scope !== 'workspace') {
      this.configError = 'Settings only supports workspace runtime configuration'
      return null
    }

    const validation = await this.validateConfig(scope)
    if (!validation.valid) {
      return null
    }

    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      this.configError = 'No active workspace connection selected'
      return null
    }
    const { connectionId, client } = resolvedClient

    this.configSaving = true
    this.configError = ''

    try {
      const patch = parseRuntimeConfigDraft(scope, this.configDrafts[scope])
      const config = await client.runtime.saveConfig(patch)
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return null
      }

      this.config = config
      this.configDrafts = createRuntimeConfigDraftsFromConfig(config)
      this.configValidation = {
        ...createRuntimeConfigValidationState(),
        [scope]: config.validation,
      }
      this.saveActiveWorkspaceSnapshot()
      return config
    } catch (error) {
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configError = error instanceof Error ? error.message : 'Failed to save runtime config'
      }
      return null
    } finally {
      if (this.activeWorkspaceConnectionId === connectionId) {
        this.configSaving = false
      }
    }
  },
  addOptimisticUserMessage(this: any, input: SubmitRuntimeTurnInput) {
    if (!this.activeSession) {
      return
    }

    const optimisticUserMessage = createOptimisticRuntimeMessage(
      this.activeSessionId,
      this.activeConversationId,
      input,
    )
    const optimisticAssistantMessage = createOptimisticAssistantMessage(
      this.activeSessionId,
      this.activeConversationId,
      input,
      optimisticUserMessage.timestamp + 1,
    )
    const detail: RuntimeSessionDetail = {
      ...this.activeSession,
      summary: {
        ...this.activeSession.summary,
        updatedAt: optimisticAssistantMessage.timestamp,
        lastMessagePreview: optimisticUserMessage.content,
      },
      messages: [
        ...this.activeSession.messages,
        optimisticUserMessage,
        optimisticAssistantMessage,
      ],
    }
    this.cacheSessionDetail(detail)
    this.saveActiveWorkspaceSnapshot()
  },
  replaceOptimisticMessages(this: any, content: string, sessionId?: string) {
    const targetSessionId = sessionId ?? this.activeSessionId
    if (!targetSessionId) {
      return
    }
    const detail = this.sessionDetails[targetSessionId]
    if (!detail) {
      return
    }

    const nextMessages = detail.messages.filter((message: { id: string, senderType: string, content: string }) => !(
      (message.id.startsWith('optimistic-msg-')
        && message.senderType === 'user'
        && message.content === content)
      || (message.id.startsWith('optimistic-assistant-')
        && message.senderType === 'assistant')
    ))

    if (nextMessages.length === detail.messages.length) {
      return
    }

    this.cacheSessionDetail({
      ...detail,
      messages: nextMessages,
    })
    this.saveActiveWorkspaceSnapshot()
  },
  async bootstrap(this: any) {
    this.syncWorkspaceScopeFromShell()
    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { connectionId, client } = resolvedClient

    if (this.bootstrapped && this.activeWorkspaceConnectionId === connectionId) {
      return
    }

    this.loading = true
    this.error = ''

    try {
      const payload = await client.runtime.bootstrap()
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return
      }

      this.provider = payload.provider
      this.sessions = payload.sessions
      this.bootstrapped = true
      this.saveActiveWorkspaceSnapshot()
    } catch (error) {
      this.error = error instanceof Error ? error.message : 'Failed to bootstrap runtime'
    } finally {
      this.loading = false
    }
  },
  async submitTurn(this: any, input: SubmitRuntimeTurnInput): Promise<boolean> {
    if (!this.activeSessionId) {
      throw new Error('No active runtime session selected')
    }

    const trimmed = input.content.trim()
    if (!trimmed) {
      return false
    }

    if (this.isBusy) {
      this.enqueueTurn({
        ...input,
        content: trimmed,
      })
      return true
    }

    const normalizedInput = {
      ...input,
      content: trimmed,
    }

    this.error = ''
    this.addOptimisticUserMessage(normalizedInput)
    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      this.replaceOptimisticMessages(trimmed)
      throw new Error('No active workspace connection selected')
    }
    const { connectionId, client } = resolvedClient

    try {
      const run = await client.runtime.submitUserTurn(this.activeSessionId, {
        content: trimmed,
        modelId: input.modelId,
        configuredModelId: input.configuredModelId,
        permissionMode: input.permissionMode,
        actorKind: input.actorKind,
        actorId: input.actorId,
      }, tauriClient.createIdempotencyKey(`runtime-turn-${connectionId}-${this.activeSessionId}`))
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return false
      }

      const detail = await client.runtime.loadSession(this.activeSessionId)
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return false
      }

      this.setActiveSession(detail)
      if (isBusyStatus(run.status)) {
        await this.startEventTransport(this.activeSessionId)
      } else {
        await this.finishTransportCycle(this.activeSessionId, connectionId)
      }
      this.saveActiveWorkspaceSnapshot()
      return true
    } catch (error) {
      this.replaceOptimisticMessages(trimmed)
      this.error = error instanceof Error ? error.message : 'Failed to submit runtime turn'
      return false
    }
  },
  async flushQueuedTurn(this: any) {
    if (!this.activeSessionId || this.pendingApproval || this.isBusy) {
      return
    }

    const [nextQueuedTurn, ...rest] = this.activeQueue
    if (!nextQueuedTurn) {
      return
    }

    if (this.activeRun?.status === 'blocked' || this.activeRun?.status === 'failed') {
      return
    }

    this.queuedTurns = {
      ...this.queuedTurns,
      [this.activeSessionId]: rest,
    }
    this.saveActiveWorkspaceSnapshot()

    await this.submitTurn(nextQueuedTurn)
  },
  async resolveApproval(this: any, decision: RuntimeDecisionAction) {
    if (!this.activeSessionId || !this.pendingApproval) {
      return
    }

    this.error = ''
    const pendingApprovalId = this.pendingApproval.id
    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { connectionId, client } = resolvedClient

    try {
      const input: ResolveRuntimeApprovalInput = { decision }
      await client.runtime.resolveApproval(
        this.activeSessionId,
        pendingApprovalId,
        input,
        tauriClient.createIdempotencyKey(`runtime-approval-${connectionId}-${pendingApprovalId}`),
      )
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return
      }

      const detail = await client.runtime.loadSession(this.activeSessionId)
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return
      }

      this.setActiveSession(detail)
      if (isBusyStatus(detail.run.status)) {
        await this.startEventTransport(this.activeSessionId)
      } else {
        await this.finishTransportCycle(this.activeSessionId, connectionId)
      }
      this.saveActiveWorkspaceSnapshot()
    } catch (error) {
      this.error = error instanceof Error ? error.message : 'Failed to resolve runtime approval'
    }
  },
  dispose(this: any) {
    this.saveActiveWorkspaceSnapshot()
    this.stopRealtimeTransport()
  },
}
