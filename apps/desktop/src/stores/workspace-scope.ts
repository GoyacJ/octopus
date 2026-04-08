import * as tauriClient from '@/tauri/client'
import { useShellStore } from '@/stores/shell'

export function activeWorkspaceConnectionId(): string {
  return useShellStore().activeWorkspaceConnection?.workspaceConnectionId ?? ''
}

export function resolveWorkspaceClientForConnection(workspaceConnectionId?: string) {
  const shell = useShellStore()
  const connectionId = workspaceConnectionId ?? activeWorkspaceConnectionId()
  if (!connectionId) {
    return null
  }

  const connection = shell.workspaceConnections.find(item => item.workspaceConnectionId === connectionId)
  if (!connection) {
    return null
  }

  return {
    connectionId,
    client: tauriClient.createWorkspaceClient({
      connection,
      session: shell.workspaceSessionsState[connectionId],
    }),
  }
}

export function ensureWorkspaceClientForConnection(workspaceConnectionId?: string) {
  const resolved = resolveWorkspaceClientForConnection(workspaceConnectionId)
  if (!resolved) {
    throw new Error('Active workspace connection is unavailable')
  }
  return resolved
}

export function createWorkspaceRequestToken(nextValue = 0): number {
  return nextValue + 1
}
