import type {
  RuntimeBootstrap,
  RuntimeDecisionAction,
  RuntimeEventEnvelope,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
} from '@octopus/schema'

import {
  bootstrapMockRuntime,
  createMockRuntimeSession,
  listMockRuntimeSessions,
  loadMockRuntimeSession,
  pollMockRuntimeEvents,
  resolveMockRuntimeApproval,
  submitMockRuntimeUserTurn,
} from './mock'
import { fetchBackend } from './shared'
import { resolveRuntimeBackendConnection } from './shell'

export async function bootstrapRuntime(): Promise<RuntimeBootstrap> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    return bootstrapMockRuntime()
  }

  return await fetchBackend<RuntimeBootstrap>(backend, '/runtime/bootstrap', {
    method: 'GET',
  })
}

export async function createRuntimeSession(
  conversationId: string,
  projectId: string,
  title: string,
  _workingDir?: string,
): Promise<RuntimeSessionDetail> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    return createMockRuntimeSession(conversationId, projectId, title)
  }

  return await fetchBackend<RuntimeSessionDetail>(backend, '/runtime/sessions', {
    method: 'POST',
    body: JSON.stringify({ conversationId, projectId, title }),
  })
}

export async function loadRuntimeSession(sessionId: string): Promise<RuntimeSessionDetail> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    return loadMockRuntimeSession(sessionId)
  }

  return await fetchBackend<RuntimeSessionDetail>(backend, `/runtime/sessions/${sessionId}`, {
    method: 'GET',
  })
}

export async function pollRuntimeEvents(sessionId: string): Promise<RuntimeEventEnvelope[]> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    return pollMockRuntimeEvents()
  }

  return await fetchBackend<RuntimeEventEnvelope[]>(backend, `/runtime/sessions/${sessionId}/events`, {
    method: 'GET',
  })
}

export async function submitRuntimeUserTurn(
  sessionId: string,
  content: string,
  modelId: string,
  permissionMode: string,
): Promise<RuntimeRunSnapshot> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    return submitMockRuntimeUserTurn(sessionId, content, modelId, permissionMode)
  }

  return await fetchBackend<RuntimeRunSnapshot>(backend, `/runtime/sessions/${sessionId}/turns`, {
    method: 'POST',
    body: JSON.stringify({ content, modelId, permissionMode }),
  })
}

export async function resolveRuntimeApproval(
  sessionId: string,
  approvalId: string,
  decision: RuntimeDecisionAction,
): Promise<void> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    resolveMockRuntimeApproval(sessionId, approvalId, decision)
    return
  }

  await fetchBackend(backend, `/runtime/sessions/${sessionId}/approvals/${approvalId}`, {
    method: 'POST',
    body: JSON.stringify({ decision }),
  })
}

export async function listRuntimeSessions(): Promise<RuntimeSessionSummary[]> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    return listMockRuntimeSessions()
  }

  return await fetchBackend(backend, '/runtime/sessions', {
    method: 'GET',
  })
}
