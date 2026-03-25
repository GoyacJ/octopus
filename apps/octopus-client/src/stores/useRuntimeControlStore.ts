import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

import type {
  ApprovalDecision,
  AutomationCreateRequest,
  AutomationDetailResponse,
  RunDetailResponse,
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

export const useRuntimeControlStore = defineStore('runtime-control', () => {
  const automations = ref<AutomationDetailResponse[]>([])
  const currentRunDetail = ref<RunDetailResponse | null>(null)
  const errorMessage = ref('')
  const hasLoadedAutomations = ref(false)

  const isLoadingAutomations = ref(false)
  const isCreatingAutomation = ref(false)
  const isSubmittingTask = ref(false)
  const isResolvingApproval = ref(false)
  const isResumingRun = ref(false)
  const activeTriggerId = ref<string | null>(null)

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
      currentRunDetail.value = response.run ?? currentRunDetail.value
      updateAutomationDelivery(payload.trigger_id, response.delivery, response.run ?? null)
      return response
    } catch (error) {
      setError(error)
      throw error
    } finally {
      activeTriggerId.value = null
    }
  }

  const submitTask = async (payload: TaskSubmissionRequest) => {
    clearError()
    isSubmittingTask.value = true

    try {
      currentRunDetail.value = await runtimeApi.submitTask(payload)
      return currentRunDetail.value
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isSubmittingTask.value = false
    }
  }

  const resolveApproval = async (decision: ApprovalDecision, reviewedBy: string) => {
    if (!currentApproval.value) {
      return null
    }

    clearError()
    isResolvingApproval.value = true

    try {
      currentRunDetail.value = await runtimeApi.resolveApproval(currentApproval.value.id, {
        decision,
        reviewed_by: reviewedBy,
      })
      return currentRunDetail.value
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
      currentRunDetail.value = await runtimeApi.resumeRun(currentRun.value.id)
      return currentRunDetail.value
    } catch (error) {
      setError(error)
      throw error
    } finally {
      isResumingRun.value = false
    }
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
    isCreatingAutomation,
    isLoadingAutomations,
    isResolvingApproval,
    isResumingRun,
    isSubmittingTask,
    clearError,
    createAutomation,
    deliverTrigger,
    loadAutomations,
    resolveApproval,
    resumeRun,
    submitTask,
  }
})
