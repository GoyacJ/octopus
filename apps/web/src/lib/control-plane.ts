import { inject, type InjectionKey } from 'vue'
import { createApiClient, type ApiClient } from '@octopus/api-client'

export type ControlPlaneClient = Pick<
  ApiClient,
  | 'listRuns'
  | 'createRun'
  | 'getRun'
  | 'getRunTimeline'
  | 'listInboxItems'
  | 'listAuditEvents'
  | 'resumeRun'
>

export const controlPlaneClientKey = Symbol('control-plane-client') as InjectionKey<ControlPlaneClient>

export function createDefaultControlPlaneClient() {
  return createApiClient({
    baseUrl: import.meta.env.VITE_CONTROL_PLANE_BASE_URL ?? '/api/v1',
  })
}

export function useControlPlaneClient() {
  const client = inject(controlPlaneClientKey)

  if (!client) {
    throw new Error('Control-plane client is not available in the current app context')
  }

  return client
}
