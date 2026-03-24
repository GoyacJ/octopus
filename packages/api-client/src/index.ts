import type { components } from './generated/control-plane'

export const controlPlaneSpecPath = 'proto/openapi/control-plane.v1.yaml'

type FetchLike = typeof fetch

export type RunRecord = components['schemas']['Run']
export type CreateRunInput = components['schemas']['CreateRunRequest']
export type ResumeRunInput = components['schemas']['ResumeRunRequest']
export type ResumeResult = components['schemas']['ResumeAcceptance']
export type InboxItemRecord = components['schemas']['InboxItem']
export type TimelineEventRecord = components['schemas']['TimelineEvent']
export type AuditEventRecord = components['schemas']['AuditEvent']
export type InteractionResponsePayload = components['schemas']['InteractionResponse']

export interface ApiClientConfig {
  baseUrl: string
  fetch?: FetchLike
}

export interface ApiClient {
  readonly config: Readonly<{ baseUrl: string }>
  listRuns(): Promise<RunRecord[]>
  createRun(input: CreateRunInput): Promise<RunRecord>
  getRun(runId: string): Promise<RunRecord>
  getRunTimeline(runId: string): Promise<TimelineEventRecord[]>
  listInboxItems(): Promise<InboxItemRecord[]>
  listAuditEvents(): Promise<AuditEventRecord[]>
  resumeRun(runId: string, input: ResumeRunInput): Promise<ResumeResult>
}

interface ErrorResponse {
  error?: string
}

interface ItemsEnvelope<T> {
  items: T[]
}

export function createApiClient(config: ApiClientConfig): ApiClient {
  const fetchImpl = config.fetch ?? globalThis.fetch

  if (!fetchImpl) {
    throw new Error('No fetch implementation available for the control-plane client')
  }

  const baseUrl = normalizeBaseUrl(config.baseUrl)

  async function request<T>(path: string, init?: RequestInit): Promise<T> {
    const response = await fetchImpl(`${baseUrl}${path}`, {
      headers: {
        'content-type': 'application/json',
        ...init?.headers,
      },
      ...init,
    })

    if (!response.ok) {
      throw await buildRequestError(response)
    }

    return (await response.json()) as T
  }

  return {
    config: { baseUrl },
    async listRuns() {
      const payload = await request<ItemsEnvelope<RunRecord>>('/runs', { method: 'GET' })
      return payload.items
    },
    createRun(input) {
      return request<RunRecord>('/runs', {
        method: 'POST',
        body: JSON.stringify(input),
      })
    },
    getRun(runId) {
      return request<RunRecord>(`/runs/${encodeURIComponent(runId)}`, { method: 'GET' })
    },
    async getRunTimeline(runId) {
      const payload = await request<ItemsEnvelope<TimelineEventRecord>>(
        `/runs/${encodeURIComponent(runId)}/timeline`,
        { method: 'GET' },
      )
      return payload.items
    },
    async listInboxItems() {
      const payload = await request<ItemsEnvelope<InboxItemRecord>>('/inbox/items', { method: 'GET' })
      return payload.items
    },
    async listAuditEvents() {
      const payload = await request<ItemsEnvelope<AuditEventRecord>>('/audit/events', { method: 'GET' })
      return payload.items
    },
    resumeRun(runId, input) {
      return request<ResumeResult>(`/runs/${encodeURIComponent(runId)}/resume`, {
        method: 'POST',
        body: JSON.stringify(input),
      })
    },
  }
}

function normalizeBaseUrl(baseUrl: string) {
  return baseUrl.endsWith('/') ? baseUrl.slice(0, -1) : baseUrl
}

async function buildRequestError(response: Response) {
  let detail = response.statusText

  try {
    const payload = (await response.json()) as ErrorResponse
    if (payload.error) {
      detail = payload.error
    }
  } catch {
    // Keep the HTTP status text when the response body is absent or non-JSON.
  }

  return new Error(`Control-plane request failed (${response.status}): ${detail}`)
}
