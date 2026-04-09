import type { RuntimeEventEnvelope } from '@octopus/schema'

import { createWorkspaceHeaders, decodeApiError, joinBaseUrl, openWorkspaceOpenApiStream } from './shared'
import { assertWorkspaceRequestReady } from './workspace_api'
import type {
  RuntimeEventSubscription,
  RuntimeEventSubscriptionOptions,
  WorkspaceClientContext,
} from './workspace-client'

function parseRuntimeEventBlock(block: string): RuntimeEventEnvelope | null {
  const lines = block
    .split('\n')
    .map(line => line.trimEnd())
    .filter(Boolean)

  let data = ''
  let id = ''
  let eventType = ''

  for (const line of lines) {
    if (line.startsWith('id:')) {
      id = line.slice(3).trim()
      continue
    }
    if (line.startsWith('event:')) {
      eventType = line.slice(6).trim()
      continue
    }
    if (line.startsWith('data:')) {
      data += `${line.slice(5).trim()}`
    }
  }

  if (!data) {
    return null
  }

  const parsed = JSON.parse(data) as RuntimeEventEnvelope
  return {
    ...parsed,
    id: parsed.id || id,
    eventType: parsed.eventType || parsed.kind || eventType || 'runtime.error',
  }
}

export async function fetchWorkspaceVoid(
  context: WorkspaceClientContext,
  path: string,
  init?: RequestInit & {
    idempotencyKey?: string
  },
): Promise<void> {
  const session = assertWorkspaceRequestReady(context)
  const headers = createWorkspaceHeaders({
    ...init,
    session,
    workspace: context.connection,
    idempotencyKey: init?.idempotencyKey,
  })
  const requestId = headers.get('X-Request-Id') ?? 'req-unknown'
  const response = await fetch(joinBaseUrl(context.connection.baseUrl, path), {
    ...init,
    headers,
  })
  if (!response.ok) {
    throw await decodeApiError(response, requestId, context.connection.workspaceConnectionId)
  }
}

export async function openRuntimeSseStream(
  context: WorkspaceClientContext,
  sessionId: string,
  options: RuntimeEventSubscriptionOptions,
): Promise<RuntimeEventSubscription> {
  const session = assertWorkspaceRequestReady(context)
  const controller = new AbortController()
  const response = await openWorkspaceOpenApiStream(
    context.connection,
    '/api/v1/runtime/sessions/{sessionId}/events',
    {
      session,
      signal: controller.signal,
      pathParams: {
        sessionId,
      },
      queryParams: options.lastEventId
        ? {
            after: options.lastEventId,
          }
        : undefined,
      headers: {
        Accept: 'text/event-stream',
        ...(options.lastEventId ? { 'Last-Event-ID': options.lastEventId } : {}),
      },
    },
  )

  if (!response.headers.get('Content-Type')?.includes('text/event-stream')) {
    throw new Error('Runtime event stream is unavailable')
  }

  if (!response.body) {
    throw new Error('Runtime event stream body is unavailable')
  }

  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  const consume = async () => {
    try {
      while (true) {
        const result = await reader.read()
        if (result.done) {
          break
        }

        buffer += decoder.decode(result.value, { stream: true })
        const blocks = buffer.split('\n\n')
        buffer = blocks.pop() ?? ''

        for (const block of blocks) {
          const event = parseRuntimeEventBlock(block)
          if (event) {
            options.onEvent(event)
          }
        }
      }

      if (!controller.signal.aborted) {
        options.onError(new Error('Runtime event stream closed'))
      }
    } catch (error) {
      if (!controller.signal.aborted) {
        options.onError(error instanceof Error ? error : new Error('Runtime event stream failed'))
      }
    }
  }

  void consume()

  return {
    mode: 'sse',
    close() {
      controller.abort()
    },
  }
}
