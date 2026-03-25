import type {
  ApprovalResolutionRequest,
  ErrorResponse,
  RunDetailResponse,
  TaskSubmissionRequest,
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
  submitTask(payload: TaskSubmissionRequest) {
    return postJson<RunDetailResponse>('/api/v1/runs/task', payload)
  },
  getRun(runId: string) {
    return getJson<RunDetailResponse>(`/api/v1/runs/${runId}`)
  },
  resolveApproval(approvalId: string, payload: ApprovalResolutionRequest) {
    return postJson<RunDetailResponse>(`/api/v1/approvals/${approvalId}/resolve`, payload)
  },
  resumeRun(runId: string) {
    return postJson<RunDetailResponse>(`/api/v1/runs/${runId}/resume`, {})
  },
}
