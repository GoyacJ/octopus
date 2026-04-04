import type {
  CreateRuntimeSessionInput,
  HostBackendConnection,
  ResolveRuntimeApprovalInput,
  RuntimeBootstrap,
  RuntimeEventEnvelope,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  SubmitRuntimeTurnInput,
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

async function withResolvedRuntimeBackend<T>(
  onReady: (backend: HostBackendConnection) => Promise<T>,
  onUnavailable: () => T | Promise<T>,
): Promise<T> {
  const backend = await resolveRuntimeBackendConnection()
  if (backend?.state !== 'ready') {
    return await onUnavailable()
  }

  return await onReady(backend)
}

export async function bootstrapRuntime(): Promise<RuntimeBootstrap> {
  return await withResolvedRuntimeBackend(
    async (backend) => await fetchBackend<RuntimeBootstrap>(backend, '/runtime/bootstrap', {
      method: 'GET',
    }),
    () => bootstrapMockRuntime(),
  )
}

export async function createRuntimeSession(input: CreateRuntimeSessionInput): Promise<RuntimeSessionDetail> {
  return await withResolvedRuntimeBackend(
    async (backend) => await fetchBackend<RuntimeSessionDetail>(backend, '/runtime/sessions', {
      method: 'POST',
      body: JSON.stringify(input),
    }),
    () => createMockRuntimeSession(input.conversationId, input.projectId, input.title),
  )
}

export async function loadRuntimeSession(sessionId: string): Promise<RuntimeSessionDetail> {
  return await withResolvedRuntimeBackend(
    async (backend) => await fetchBackend<RuntimeSessionDetail>(backend, `/runtime/sessions/${sessionId}`, {
      method: 'GET',
    }),
    () => loadMockRuntimeSession(sessionId),
  )
}

export async function pollRuntimeEvents(sessionId: string): Promise<RuntimeEventEnvelope[]> {
  return await withResolvedRuntimeBackend(
    async (backend) => await fetchBackend<RuntimeEventEnvelope[]>(backend, `/runtime/sessions/${sessionId}/events`, {
      method: 'GET',
    }),
    () => pollMockRuntimeEvents(),
  )
}

export async function submitRuntimeUserTurn(
  sessionId: string,
  input: SubmitRuntimeTurnInput,
): Promise<RuntimeRunSnapshot> {
  return await withResolvedRuntimeBackend(
    async (backend) => await fetchBackend<RuntimeRunSnapshot>(backend, `/runtime/sessions/${sessionId}/turns`, {
      method: 'POST',
      body: JSON.stringify(input),
    }),
    () => submitMockRuntimeUserTurn(sessionId, input.content, input.modelId, input.permissionMode),
  )
}

export async function resolveRuntimeApproval(
  sessionId: string,
  approvalId: string,
  input: ResolveRuntimeApprovalInput,
): Promise<void> {
  await withResolvedRuntimeBackend(
    async (backend) => {
      await fetchBackend(backend, `/runtime/sessions/${sessionId}/approvals/${approvalId}`, {
        method: 'POST',
        body: JSON.stringify(input),
      })
    },
    () => resolveMockRuntimeApproval(sessionId, approvalId, input.decision),
  )
}

export async function listRuntimeSessions(): Promise<RuntimeSessionSummary[]> {
  return await withResolvedRuntimeBackend(
    async (backend) => await fetchBackend(backend, '/runtime/sessions', {
      method: 'GET',
    }),
    () => listMockRuntimeSessions(),
  )
}
