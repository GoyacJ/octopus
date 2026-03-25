import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'

import type {
  ApprovalDecision,
  AutomationCreateRequest,
  AutomationDetailResponse,
  InboxItemRecord,
  KnowledgeAssetRecord,
  KnowledgeCandidateRecord,
  KnowledgeSpaceDetailResponse,
  McpEventDeliveryRequest,
  RunDetailResponse,
  RunRecord,
  RuntimeEventEnvelope,
  TaskSubmissionRequest,
  TriggerDeliveryRecord,
  TriggerDeliveryRequest,
} from '@octopus/contracts'

import { RuntimeApiError, runtimeApi } from '@/services/runtimeApi'

const upsertAutomation = (
  current: AutomationDetailResponse[],
  incoming: AutomationDetailResponse,
): AutomationDetailResponse[] => {
  const existingIndex = current.findIndex((entry) => entry.automation.id === incoming.automation.id)
  if (existingIndex === -1) {
    return [...current, incoming]
  }

  return current.map((entry, index) => (index === existingIndex ? incoming : entry))
}

const upsertCandidate = (
  current: KnowledgeCandidateRecord[],
  incoming: KnowledgeCandidateRecord,
): KnowledgeCandidateRecord[] => {
  const existingIndex = current.findIndex((entry) => entry.id === incoming.id)
  if (existingIndex === -1) {
    return [...current, incoming]
  }

  return current.map((entry, index) => (index === existingIndex ? incoming : entry))
}

const upsertAsset = (
  current: KnowledgeAssetRecord[],
  incoming: KnowledgeAssetRecord,
): KnowledgeAssetRecord[] => {
  const existingIndex = current.findIndex((entry) => entry.id === incoming.id)
  if (existingIndex === -1) {
    return [...current, incoming]
  }

  return current.map((entry, index) => (index === existingIndex ? incoming : entry))
}

const upsertRun = (current: RunRecord[], incoming: RunRecord): RunRecord[] => {
  const existingIndex = current.findIndex((entry) => entry.id === incoming.id)
  if (existingIndex === -1) {
    return [...current, incoming]
  }

  return current.map((entry, index) => (index === existingIndex ? incoming : entry))
}

const upsertInboxItem = (
  current: InboxItemRecord[],
  incoming: InboxItemRecord,
): InboxItemRecord[] => {
  const existingIndex = current.findIndex((entry) => entry.id === incoming.id)
  if (existingIndex === -1) {
    return [...current, incoming]
  }

  return current.map((entry, index) => (index === existingIndex ? incoming : entry))
}

const runEventTopics = ['run.state_changed', 'approval.updated', 'trigger.delivery_updated']
const automationEventTopics = ['automation.updated', 'trigger.delivery_updated']
const knowledgeEventTopics = ['knowledge.candidate_updated', 'knowledge.asset_updated']

export const useRuntimeControlStore = defineStore('runtime-control', () => {
  const automations = ref<AutomationDetailResponse[]>([])
  const knowledgeSpaces = ref<KnowledgeSpaceDetailResponse[]>([])
  const runs = ref<RunRecord[]>([])
  const inboxItems = ref<InboxItemRecord[]>([])
  const currentRunDetail = ref<RunDetailResponse | null>(null)
  const errorMessage = ref('')
  const hasLoadedRuns = ref(false)
  const hasLoadedInboxItems = ref(false)
  const hasLoadedAutomations = ref(false)
  const hasLoadedKnowledgeSpaces = ref(false)

  const isLoadingRuns = ref(false)
  const isLoadingInboxItems = ref(false)
  const isLoadingAutomations = ref(false)
  const isLoadingKnowledgeSpaces = ref(false)
  const isCreatingAutomation = ref(false)
  const isSubmittingTask = ref(false)
  const isResolvingApproval = ref(false)
  const isResumingRun = ref(false)
  const isCreatingCandidate = ref(false)
  const isDeliveringMcpEvent = ref(false)
  const activeTriggerId = ref<string | null>(null)
  const promotingCandidateId = ref<string | null>(null)
  const runtimeEventSource = shallowRef<EventSource | null>(null)
  const lastEventSequence = ref<number | null>(null)

  const currentRun = computed(() => currentRunDetail.value?.run ?? null)
  const currentApproval = computed(() => currentRunDetail.value?.approval ?? null)
  const currentArtifact = computed(() => currentRunDetail.value?.artifact ?? null)
  const currentInboxItem = computed(() => currentRunDetail.value?.inbox_item ?? null)

  const clearError = () => {
    errorMessage.value = ''
  }

  const setError = (error: unknown) => {
    if (error instanceof RuntimeApiError) {
      errorMessage.value = error.message
      return
    }

    errorMessage.value = 'Request failed.'
  }

  const syncRunDetail = (detail: RunDetailResponse) => {
    currentRunDetail.value = detail
    runs.value = upsertRun(runs.value, detail.run)

    if (detail.inbox_item) {
      inboxItems.value = upsertInboxItem(inboxItems.value, detail.inbox_item)
    }
  }

  const refreshCurrentRun = async (runId: string | null | undefined) => {
    if (!runId) {
      return null
    }

    const detail = await runtimeApi.getRun(runId)
    syncRunDetail(detail)
    return detail
  }

  const loadRuns = async (force = false) => {
    if (isLoadingRuns.value || (hasLoadedRuns.value && !force)) {
      return
    }

    clearError()
    isLoadingRuns.value = true

    try {
      const response = await runtimeApi.listRuns()
      runs.value = response.items
      hasLoadedRuns.value = true
    } catch (error) {
      setError(error)
    } finally {
      isLoadingRuns.value = false
    }
  }

  const loadInboxItems = async (force = false) => {
    if (isLoadingInboxItems.value || (hasLoadedInboxItems.value && !force)) {
      return
    }

    clearError()
    isLoadingInboxItems.value = true

    try {
      const response = await runtimeApi.listInboxItems()
      inboxItems.value = response.items
      hasLoadedInboxItems.value = true
    } catch (error) {
      setError(error)
    } finally {
      isLoadingInboxItems.value = false
    }
  }

  const loadAutomations = async (force = false) => {
    if (isLoadingAutomations.value || (hasLoadedAutomations.value && !force)) {
      return
    }

    clearError()
    isLoadingAutomations.value = true

    try {
      const response = await runtimeApi.listAutomations()
      automations.value = response.items
      hasLoadedAutomations.value = true
    } catch (error) {
      setError(error)
    } finally {
      isLoadingAutomations.value = false
    }
  }

  const loadKnowledgeSpaces = async (force = false) => {
    if (isLoadingKnowledgeSpaces.value || (hasLoadedKnowledgeSpaces.value && !force)) {
      return
    }

    clearError()
    isLoadingKnowledgeSpaces.value = true

    try {
      const response = await runtimeApi.listKnowledgeSpaces()
      knowledgeSpaces.value = response.items
      hasLoadedKnowledgeSpaces.value = true
    } catch (error) {
      setError(error)
    } finally {
      isLoadingKnowledgeSpaces.value = false
    }
  }

  const createAutomation = async (payload: AutomationCreateRequest) => {
    clearError()
    isCreatingAutomation.value = true

    try {
      const detail = await runtimeApi.createAutomation(payload)
      automations.value = upsertAutomation(automations.value, detail)
      return detail
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isCreatingAutomation.value = false
    }
  }

  const updateAutomationDelivery = (
    triggerId: string,
    delivery: TriggerDeliveryRecord,
    runDetail: RunDetailResponse | null,
  ) => {
    automations.value = automations.value.map((entry) => {
      if (entry.trigger.id !== triggerId) {
        return entry
      }

      return {
        ...entry,
        automation: {
          ...entry.automation,
          last_run_id: runDetail?.run.id ?? entry.automation.last_run_id,
        },
        latest_delivery: delivery,
        latest_run: runDetail ?? entry.latest_run,
      }
    })
  }

  const deliverTrigger = async (payload: TriggerDeliveryRequest) => {
    clearError()
    activeTriggerId.value = payload.trigger_id

    try {
      const response = await runtimeApi.deliverTrigger(payload)
      if (response.run) {
        syncRunDetail(response.run)
      }

      updateAutomationDelivery(payload.trigger_id, response.delivery, response.run ?? null)
      return response
    } catch (error) {
      setError(error)
      throw error
    } finally {
      activeTriggerId.value = null
    }
  }

  const deliverMcpEvent = async (payload: McpEventDeliveryRequest) => {
    clearError()
    isDeliveringMcpEvent.value = true

    try {
      const response = await runtimeApi.deliverMcpEvent(payload)
      response.items.forEach((entry) => {
        updateAutomationDelivery(entry.delivery.trigger_id, entry.delivery, entry.run ?? null)
      })

      const latestRun = [...response.items].reverse().find((entry) => entry.run)?.run ?? null
      if (latestRun) {
        syncRunDetail(latestRun)
      }

      return response
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isDeliveringMcpEvent.value = false
    }
  }

  const submitTask = async (payload: TaskSubmissionRequest) => {
    clearError()
    isSubmittingTask.value = true

    try {
      const detail = await runtimeApi.submitTask(payload)
      syncRunDetail(detail)
      return detail
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isSubmittingTask.value = false
    }
  }

  const createCandidateFromRun = async (knowledgeSpaceId: string) => {
    if (!currentRun.value) {
      return null
    }

    clearError()
    isCreatingCandidate.value = true

    try {
      const response = await runtimeApi.createCandidateFromRun({
        run_id: currentRun.value.id,
        knowledge_space_id: knowledgeSpaceId,
        created_by: currentRun.value.requested_by,
      })

      knowledgeSpaces.value = knowledgeSpaces.value.map((entry) => {
        if (entry.space.id !== knowledgeSpaceId) {
          return entry
        }

        return {
          ...entry,
          candidates: upsertCandidate(entry.candidates, response.candidate),
        }
      })

      await refreshCurrentRun(response.candidate.run_id)
      return response.candidate
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isCreatingCandidate.value = false
    }
  }

  const promoteCandidate = async (candidateId: string) => {
    const targetSpace = knowledgeSpaces.value.find((space) =>
      space.candidates.some((candidate) => candidate.id === candidateId),
    )
    if (!targetSpace) {
      return null
    }

    clearError()
    promotingCandidateId.value = candidateId

    try {
      const response = await runtimeApi.promoteCandidate(candidateId, {
        promoted_by: targetSpace.space.owner_refs[0] ?? 'owner-1',
      })

      knowledgeSpaces.value = knowledgeSpaces.value.map((entry) => {
        if (entry.space.id !== response.candidate.knowledge_space_id) {
          return entry
        }

        return {
          ...entry,
          candidates: upsertCandidate(entry.candidates, response.candidate),
          assets: upsertAsset(entry.assets, response.asset),
        }
      })

      await refreshCurrentRun(response.candidate.run_id)
      return response
    } catch (error) {
      setError(error)
      throw error
    } finally {
      promotingCandidateId.value = null
    }
  }

  const resolveApproval = async (decision: ApprovalDecision, reviewedBy: string) => {
    if (!currentApproval.value) {
      return null
    }

    clearError()
    isResolvingApproval.value = true

    try {
      const detail = await runtimeApi.resolveApproval(currentApproval.value.id, {
        decision,
        reviewed_by: reviewedBy,
      })
      syncRunDetail(detail)
      return detail
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isResolvingApproval.value = false
    }
  }

  const resumeRun = async () => {
    if (!currentRun.value) {
      return null
    }

    clearError()
    isResumingRun.value = true

    try {
      const detail = await runtimeApi.resumeRun(currentRun.value.id)
      syncRunDetail(detail)
      return detail
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isResumingRun.value = false
    }
  }

  const selectRun = async (runId: string) => {
    clearError()

    try {
      return await refreshCurrentRun(runId)
    } catch (error) {
      setError(error)
      throw error
    }
  }

  const refreshFromRuntimeEvent = async (event: RuntimeEventEnvelope) => {
    try {
      if (event.run_id && runEventTopics.includes(event.topic)) {
        await refreshCurrentRun(event.run_id)
      }

      const refreshTasks: Array<Promise<void>> = []

      if (runEventTopics.includes(event.topic)) {
        refreshTasks.push(loadRuns(true), loadInboxItems(true))
      }

      if (automationEventTopics.includes(event.topic)) {
        refreshTasks.push(loadAutomations(true))
      }

      if (knowledgeEventTopics.includes(event.topic)) {
        refreshTasks.push(loadKnowledgeSpaces(true))
      }

      await Promise.all(refreshTasks)
    } catch (error) {
      setError(error)
    }
  }

  const startRuntimeEventStream = () => {
    if (runtimeEventSource.value || typeof EventSource === 'undefined') {
      return
    }

    const source = new EventSource('/api/v1/events/stream')
    source.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data) as RuntimeEventEnvelope
        lastEventSequence.value = payload.sequence
        void refreshFromRuntimeEvent(payload)
      } catch {
        errorMessage.value = 'Request failed.'
      }
    }

    runtimeEventSource.value = source
  }

  const stopRuntimeEventStream = () => {
    runtimeEventSource.value?.close()
    runtimeEventSource.value = null
  }

  return {
    activeTriggerId,
    automations,
    currentApproval,
    currentArtifact,
    currentInboxItem,
    currentRun,
    currentRunDetail,
    errorMessage,
    hasLoadedAutomations,
    hasLoadedInboxItems,
    hasLoadedKnowledgeSpaces,
    hasLoadedRuns,
    inboxItems,
    isCreatingAutomation,
    isCreatingCandidate,
    isDeliveringMcpEvent,
    isLoadingAutomations,
    isLoadingInboxItems,
    isLoadingKnowledgeSpaces,
    isLoadingRuns,
    isResolvingApproval,
    isResumingRun,
    isSubmittingTask,
    knowledgeSpaces,
    lastEventSequence,
    promotingCandidateId,
    runs,
    clearError,
    createAutomation,
    createCandidateFromRun,
    deliverMcpEvent,
    deliverTrigger,
    loadAutomations,
    loadInboxItems,
    loadKnowledgeSpaces,
    loadRuns,
    promoteCandidate,
    resolveApproval,
    resumeRun,
    selectRun,
    startRuntimeEventStream,
    stopRuntimeEventStream,
    submitTask,
  }
})
