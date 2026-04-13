import { resolveRuntimePermissionMode, type RuntimePermissionMode } from '@octopus/schema'

import { fetchWorkspaceOpenApi } from './shared'
import { openRuntimeSseStream } from './runtime_events'
import { assertWorkspaceRequestReady } from './workspace_api'
import type { WorkspaceClient, WorkspaceClientContext } from './workspace-client'

export function createRuntimeApi(context: WorkspaceClientContext): WorkspaceClient['runtime'] {
  return {
    async bootstrap() {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/bootstrap', 'get', {
        session: assertWorkspaceRequestReady(context),
      })
    },
    async getConfig() {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config', 'get', {
        session: assertWorkspaceRequestReady(context),
      })
    },
    async validateConfig(patch) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config/validate', 'post', {
        session: assertWorkspaceRequestReady(context),
        body: JSON.stringify(patch),
      })
    },
    async validateConfiguredModel(input) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config/configured-models/probe', 'post', {
        session: assertWorkspaceRequestReady(context),
        body: JSON.stringify(input),
      })
    },
    async saveConfig(patch) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/config/scopes/{scope}', 'patch', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          scope: 'workspace',
        },
        body: JSON.stringify(patch),
      })
    },
    async getProjectConfig(projectId) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/runtime-config', 'get', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          projectId,
        },
      })
    },
    async validateProjectConfig(projectId, patch) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/runtime-config/validate', 'post', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          projectId,
        },
        body: JSON.stringify(patch),
      })
    },
    async saveProjectConfig(projectId, patch) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/runtime-config', 'patch', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          projectId,
        },
        body: JSON.stringify(patch),
      })
    },
    async getUserConfig() {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/personal-center/profile/runtime-config', 'get', {
        session: assertWorkspaceRequestReady(context),
      })
    },
    async validateUserConfig(patch) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/personal-center/profile/runtime-config/validate', 'post', {
        session: assertWorkspaceRequestReady(context),
        body: JSON.stringify(patch),
      })
    },
    async saveUserConfig(patch) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/personal-center/profile/runtime-config', 'patch', {
        session: assertWorkspaceRequestReady(context),
        body: JSON.stringify(patch),
      })
    },
    async listSessions() {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions', 'get', {
        session: assertWorkspaceRequestReady(context),
      })
    },
    async createSession(input, idempotencyKey) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions', 'post', {
        session: assertWorkspaceRequestReady(context),
        body: JSON.stringify(input),
        idempotencyKey,
      })
    },
    async loadSession(sessionId) {
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}', 'get', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          sessionId,
        },
      })
    },
    async deleteSession(sessionId) {
      await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}', 'delete', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          sessionId,
        },
      })
    },
    async pollEvents(sessionId, options = {}) {
      return await fetchWorkspaceOpenApi(
        context.connection,
        '/api/v1/runtime/sessions/{sessionId}/events',
        'get',
        {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            sessionId,
          },
          queryParams: {
            after: options.after,
          },
        },
      )
    },
    async subscribeEvents(sessionId, options) {
      return await openRuntimeSseStream(context, sessionId, options)
    },
    async submitUserTurn(sessionId, input, idempotencyKey) {
      const resolvedPermissionMode: RuntimePermissionMode = resolveRuntimePermissionMode(input.permissionMode ?? 'read-only')
      return await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}/turns', 'post', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          sessionId,
        },
        body: JSON.stringify({
          ...input,
          permissionMode: resolvedPermissionMode,
        }),
        idempotencyKey,
      })
    },
    async resolveApproval(sessionId, approvalId, input, idempotencyKey) {
      await fetchWorkspaceOpenApi(context.connection, '/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}', 'post', {
        session: assertWorkspaceRequestReady(context),
        pathParams: {
          sessionId,
          approvalId,
        },
        body: JSON.stringify(input),
        idempotencyKey,
      })
    },
  }
}
