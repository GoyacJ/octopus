import { defineStore } from 'pinia'

import { enumLabel, resolveRunDisplayValue } from '@/i18n/copy'
import {
  createRuntimeConfigDrafts,
  createRuntimeConfigValidationState,
} from '@/stores/runtime-config'

import {
  type Message,
  type ProviderConfig,
  type RuntimeApprovalRequest,
  type RuntimeAuthChallengeSummary,
  type RuntimeConfiguredModelProbeResult,
  type RuntimeEffectiveConfig,
  type RuntimeMediationOutcome,
  type RuntimePendingMediation,
  type RuntimeRunSnapshot,
  type RuntimeSessionDetail,
  type RuntimeSessionSummary,
  type RuntimeTraceItem,
  type ToolCatalogKind,
} from '@octopus/schema'

import { toConversationMessage } from './runtime_messages'
import { buildToolStats, runtimeEventActions } from './runtime_events'
import {
  isBusyStatus,
  runtimeSessionActions,
  type RuntimeQueueItem,
  type RuntimeTransportMode,
  type RuntimeWorkspaceSnapshot,
} from './runtime_sessions'
import { runtimeStoreActions } from './runtime_actions'

export type { RuntimeQueueItem, RuntimeTransportMode, RuntimeWorkspaceSnapshot } from './runtime_sessions'

export const useRuntimeStore = defineStore('runtime', {
  state: () => ({
    provider: null as ProviderConfig | null,
    bootstrapped: false,
    loading: false,
    sessions: [] as RuntimeSessionSummary[],
    sessionDetails: {} as Record<string, RuntimeSessionDetail>,
    activeSessionId: '',
    activeConversationId: '',
    queuedTurns: {} as Record<string, RuntimeQueueItem[]>,
    lastEventIds: {} as Record<string, string>,
    activeWorkspaceConnectionId: '',
    workspaceStateSnapshots: {} as Record<string, RuntimeWorkspaceSnapshot>,
    transportMode: 'idle' as RuntimeTransportMode,
    streamSessionId: '',
    streamSubscription: null as { close: () => void } | null,
    pollingSessionId: '',
    pollingTimer: null as ReturnType<typeof setInterval> | null,
    config: null as RuntimeEffectiveConfig | null,
    configDrafts: createRuntimeConfigDrafts(),
    configValidation: createRuntimeConfigValidationState(),
    configuredModelProbeResult: null as RuntimeConfiguredModelProbeResult | null,
    configuredModelProbing: false,
    configLoading: false,
    configSaving: false,
    configValidating: false,
    configError: '',
    error: '',
  }),
  getters: {
    activeSession(state): RuntimeSessionDetail | null {
      return state.activeSessionId ? state.sessionDetails[state.activeSessionId] ?? null : null
    },
    activeRun(): RuntimeRunSnapshot | null {
      return this.activeSession?.run ?? null
    },
    activeTrace(): RuntimeTraceItem[] {
      return this.activeSession?.trace ?? []
    },
    activeMessages(): Message[] {
      return (this.activeSession?.messages ?? []).map((message) =>
        toConversationMessage(message, this.pendingApproval ?? undefined),
      )
    },
    pendingApproval(): RuntimeApprovalRequest | null {
      return this.activeRun?.approvalTarget ?? this.activeSession?.pendingApproval ?? null
    },
    pendingMediation(): RuntimePendingMediation | null {
      if (this.activeRun) {
        return this.activeRun.pendingMediation ?? null
      }
      return this.activeSession?.pendingMediation ?? this.activeSession?.summary.pendingMediation ?? null
    },
    authTarget(): RuntimeAuthChallengeSummary | null {
      return this.activeRun?.authTarget
        ?? this.activeRun?.checkpoint.pendingAuthChallenge
        ?? null
    },
    pendingMemoryProposal(): RuntimeRunSnapshot['pendingMemoryProposal'] | null {
      const proposal = this.activeRun?.pendingMemoryProposal ?? null
      return proposal?.proposalState === 'pending' ? proposal : null
    },
    lastMediationOutcome(): RuntimeMediationOutcome | null {
      return this.activeRun?.lastMediationOutcome ?? null
    },
    activeRunStatusLabel(): string {
      const status = this.activeRun?.status
      if (!status) {
        return 'N/A'
      }

      try {
        return enumLabel('runStatus', status)
      } catch {
        return status
      }
    },
    activeRunCurrentStepLabel(): string {
      return resolveRunDisplayValue(this.activeRun?.currentStep)
    },
    activeRunNextActionLabel(): string {
      return resolveRunDisplayValue(this.activeRun?.nextAction)
    },
    activeQueue(state): RuntimeQueueItem[] {
      return state.activeSessionId ? state.queuedTurns[state.activeSessionId] ?? [] : []
    },
    activeToolStats(): Array<{ toolId: string, label: string, kind: ToolCatalogKind, count: number }> {
      return buildToolStats(this.activeTrace)
    },
    isBusy(): boolean {
      return isBusyStatus(this.activeRun?.status)
        || (this.activeSession?.messages ?? []).some(message => (
          message.senderType === 'assistant' && message.id.startsWith('optimistic-assistant-')
        ))
    },
    activeWorkspaceConfig(): RuntimeEffectiveConfig | null {
      return this.config
    },
  },
  actions: {
    ...runtimeSessionActions,
    ...runtimeEventActions,
    ...runtimeStoreActions,
  },
})
