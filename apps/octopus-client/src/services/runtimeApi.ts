import type {
  ApprovalResolutionRequest,
  AutomationCreateRequest,
  AutomationDetailResponse,
  AutomationListResponse,
  ErrorResponse,
  KnowledgeCandidateCreateRequest,
  KnowledgeCandidateResponse,
  KnowledgePromotionRequest,
  KnowledgePromotionResponse,
  KnowledgeSpaceListResponse,
  McpEventDeliveryRequest,
  McpEventDeliveryResponse,
  RunDetailResponse,
  TaskSubmissionRequest,
  TriggerDeliveryRequest,
  TriggerDeliveryResponse,
} from '@octopus/contracts'

const defaultHeaders = {
  'content-type': 'application/json',
}

export class RuntimeApiError extends Error {
  status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'RuntimeApiError'
    this.status = status
  }
}

const parseResponse = async <T>(response: Response): Promise<T> => {
  if (!response.ok) {
    let message = `Request failed with status ${response.status}`

    try {
      const payload = (await response.json()) as ErrorResponse
      message = payload.message ?? message
    } catch {
      // Fall back to the generic status message if the response is not JSON.
    }

    throw new RuntimeApiError(response.status, message)
  }

  return (await response.json()) as T
}

const postJson = async <T>(path: string, body: unknown): Promise<T> => {
  const response = await fetch(path, {
    method: 'POST',
    headers: defaultHeaders,
    body: JSON.stringify(body),
  })

  return parseResponse<T>(response)
}

const getJson = async <T>(path: string): Promise<T> => {
  const response = await fetch(path)

  return parseResponse<T>(response)
}

export const runtimeApi = {
  listAutomations() {
    return getJson<AutomationListResponse>('/api/v1/automations')
  },
  createAutomation(payload: AutomationCreateRequest) {
    return postJson<AutomationDetailResponse>('/api/v1/automations', payload)
  },
  listKnowledgeSpaces() {
    return getJson<KnowledgeSpaceListResponse>('/api/v1/knowledge/spaces')
  },
  createCandidateFromRun(payload: KnowledgeCandidateCreateRequest) {
    return postJson<KnowledgeCandidateResponse>('/api/v1/knowledge/candidates/from-run', payload)
  },
  promoteCandidate(candidateId: string, payload: KnowledgePromotionRequest) {
    return postJson<KnowledgePromotionResponse>(`/api/v1/knowledge/candidates/${candidateId}/promote`, payload)
  },
  submitTask(payload: TaskSubmissionRequest) {
    return postJson<RunDetailResponse>('/api/v1/runs/task', payload)
  },
  getRun(runId: string) {
    return getJson<RunDetailResponse>(`/api/v1/runs/${runId}`)
  },
  resolveApproval(approvalId: string, payload: ApprovalResolutionRequest) {
    return postJson<RunDetailResponse>(`/api/v1/approvals/${approvalId}/resolve`, payload)
  },
  deliverTrigger(payload: TriggerDeliveryRequest) {
    return postJson<TriggerDeliveryResponse>('/api/v1/triggers/deliver', payload)
  },
  deliverMcpEvent(payload: McpEventDeliveryRequest) {
    return postJson<McpEventDeliveryResponse>('/api/v1/mcp/events/deliver', payload)
  },
  resumeRun(runId: string) {
    return postJson<RunDetailResponse>(`/api/v1/runs/${runId}/resume`, {})
  },
}
