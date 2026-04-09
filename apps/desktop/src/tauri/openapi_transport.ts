import type {
  HostBackendConnection,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'
import type { OctopusApiPaths } from '@octopus/schema/generated'

import { decodeApiError } from './error_mapping'
import { createRequestId, joinBaseUrl } from './workspace_session'

export type OpenApiPath = keyof OctopusApiPaths & string
export type OpenApiMethod<TPath extends OpenApiPath> = keyof OctopusApiPaths[TPath] & string
type OpenApiOperation<
  TPath extends OpenApiPath,
  TMethod extends OpenApiMethod<TPath>,
> = OctopusApiPaths[TPath][TMethod]
export type OpenApiResponse<
  TPath extends OpenApiPath,
  TMethod extends OpenApiMethod<TPath>,
> = OpenApiOperation<TPath, TMethod> extends { response: infer TResponse } ? TResponse : never
type OpenApiScalar = string | number | boolean
type OpenApiPathParamNames<TPath extends string> =
  TPath extends `${string}{${infer Param}}${infer Rest}`
    ? Param | OpenApiPathParamNames<Rest>
    : never

export type OpenApiPathParams<TPath extends OpenApiPath> =
  [OpenApiPathParamNames<TPath>] extends [never]
    ? Record<string, never>
    : { [TParam in OpenApiPathParamNames<TPath>]: OpenApiScalar }

export interface OpenApiRequestOptions<TPath extends OpenApiPath> extends RequestInit {
  pathParams?: OpenApiPathParams<TPath>
  queryParams?: Record<string, OpenApiScalar | null | undefined>
}

export interface WorkspaceRequestOptions extends Omit<RequestInit, 'headers'> {
  headers?: HeadersInit
  idempotencyKey?: string
  session?: WorkspaceSessionTokenEnvelope
  workspace?: WorkspaceConnectionRecord
}

export interface WorkspaceOpenApiRequestOptions<TPath extends OpenApiPath> extends WorkspaceRequestOptions {
  pathParams?: OpenApiPathParams<TPath>
  queryParams?: Record<string, OpenApiScalar | null | undefined>
}

export function normalizeComparableApiPath(value: string): string {
  let normalized = value.trim()
  const queryIndex = normalized.indexOf('?')
  if (queryIndex >= 0) {
    normalized = normalized.slice(0, queryIndex)
  }

  normalized = normalized.replace(/\$\{[^}]*\}/g, '{param}')
  normalized = normalized.replace(/\$\{.*$/, '')
  normalized = normalized.replace(/:[A-Za-z0-9_]+/g, '{param}')
  normalized = normalized.replace(/\*[A-Za-z0-9_]+/g, '{param}')
  normalized = normalized.replace(/\{[^}]+\}/g, '{param}')
  normalized = normalized.replace(/(?<!\/)\{param\}/g, '')
  normalized = normalized.replace(/\/+/g, '/')

  if (normalized.length > 1 && normalized.endsWith('/')) {
    normalized = normalized.slice(0, -1)
  }

  return normalized
}

function resolveHttpMethod(method: string): string {
  return method.toUpperCase()
}

function resolveOpenApiPath<TPath extends OpenApiPath>(
  path: TPath,
  pathParams?: OpenApiPathParams<TPath>,
  queryParams?: Record<string, OpenApiScalar | null | undefined>,
): string {
  const resolvedPath = path.replace(/\{([^}]+)\}/g, (_, key: string) => {
    const value = pathParams?.[key as keyof OpenApiPathParams<TPath>]
    if (value === undefined || value === null) {
      throw new Error(`Missing OpenAPI path param: ${key}`)
    }

    return encodeURIComponent(String(value))
  })

  if (!queryParams) {
    return resolvedPath
  }

  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(queryParams)) {
    if (value === undefined || value === null) {
      continue
    }
    search.set(key, String(value))
  }

  return search.size ? `${resolvedPath}?${search.toString()}` : resolvedPath
}

async function fetchJson<T>(
  input: string,
  init: RequestInit,
  requestId: string,
  workspaceConnectionId?: string,
): Promise<T> {
  const response = await fetch(input, init)

  if (!response.ok) {
    throw await decodeApiError(response, requestId, workspaceConnectionId)
  }

  if (response.status === 204 || response.status === 205 || response.headers.get('Content-Length') === '0') {
    return undefined as T
  }

  return await response.json() as T
}

export function createHostHeaders(authToken: string, init?: RequestInit): Headers {
  const headers = new Headers(init?.headers)
  headers.set('Authorization', `Bearer ${authToken}`)
  headers.set('Accept', 'application/json')
  headers.set('X-Request-Id', createRequestId())
  if (!headers.has('Content-Type') && init?.body) {
    headers.set('Content-Type', 'application/json')
  }

  return headers
}

export async function fetchHostApi<T>(
  baseUrl: string,
  authToken: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  const headers = createHostHeaders(authToken, init)
  const requestId = headers.get('X-Request-Id') ?? createRequestId()

  return await fetchJson<T>(joinBaseUrl(baseUrl, path), {
    ...init,
    headers,
  }, requestId)
}

export async function fetchHostOpenApi<
  TPath extends OpenApiPath,
  TMethod extends OpenApiMethod<TPath>,
>(
  baseUrl: string,
  authToken: string,
  path: TPath,
  method: TMethod,
  init?: OpenApiRequestOptions<TPath>,
): Promise<OpenApiResponse<TPath, TMethod>> {
  const { pathParams, queryParams, ...requestInit } = init ?? {}
  return await fetchHostApi<OpenApiResponse<TPath, TMethod>>(
    baseUrl,
    authToken,
    resolveOpenApiPath(path, pathParams, queryParams),
    {
      ...requestInit,
      method: resolveHttpMethod(method),
    },
  )
}

export async function fetchBackend<T>(
  backend: HostBackendConnection | undefined,
  path: string,
  init?: RequestInit,
): Promise<T> {
  if (backend?.state !== 'ready' || !backend.baseUrl) {
    throw new Error('Desktop backend is unavailable')
  }

  const requestId = createRequestId()
  const headers = new Headers(init?.headers)
  if (!headers.has('Content-Type') && init?.body) {
    headers.set('Content-Type', 'application/json')
  }
  headers.set('X-Request-Id', requestId)

  return await fetchJson<T>(joinBaseUrl(backend.baseUrl, path), {
    ...init,
    headers,
  }, requestId)
}

export function createWorkspaceHeaders(options: WorkspaceRequestOptions = {}): Headers {
  const headers = new Headers(options.headers)

  if (options.session?.token) {
    headers.set('Authorization', `Bearer ${options.session.token}`)
  }
  if (options.workspace?.workspaceId) {
    headers.set('X-Workspace-Id', options.workspace.workspaceId)
  }
  if (!headers.has('Content-Type') && options.body) {
    headers.set('Content-Type', 'application/json')
  }
  if (!headers.has('Accept')) {
    headers.set('Accept', 'application/json')
  }

  headers.set('X-Request-Id', createRequestId())

  if (options.idempotencyKey) {
    headers.set('Idempotency-Key', options.idempotencyKey)
  }

  return headers
}

export async function fetchWorkspaceApi<T>(
  workspace: WorkspaceConnectionRecord,
  path: string,
  options: WorkspaceRequestOptions = {},
): Promise<T> {
  const headers = createWorkspaceHeaders({
    ...options,
    workspace,
  })
  const requestId = headers.get('X-Request-Id') ?? createRequestId()

  return await fetchJson<T>(joinBaseUrl(workspace.baseUrl, path), {
    ...options,
    headers,
  }, requestId, workspace.workspaceConnectionId)
}

export async function fetchWorkspaceOpenApi<
  TPath extends OpenApiPath,
  TMethod extends OpenApiMethod<TPath>,
>(
  workspace: WorkspaceConnectionRecord,
  path: TPath,
  method: TMethod,
  options: WorkspaceOpenApiRequestOptions<TPath> = {},
): Promise<OpenApiResponse<TPath, TMethod>> {
  const { pathParams, queryParams, ...requestOptions } = options
  return await fetchWorkspaceApi<OpenApiResponse<TPath, TMethod>>(
    workspace,
    resolveOpenApiPath(path, pathParams, queryParams),
    {
      ...requestOptions,
      method: resolveHttpMethod(method),
    },
  )
}

export async function openWorkspaceOpenApiStream<TPath extends OpenApiPath>(
  workspace: WorkspaceConnectionRecord,
  path: TPath,
  options: WorkspaceOpenApiRequestOptions<TPath> = {},
): Promise<Response> {
  const { pathParams, queryParams, ...requestOptions } = options
  const headers = createWorkspaceHeaders({
    ...requestOptions,
    workspace,
  })
  const requestId = headers.get('X-Request-Id') ?? createRequestId()
  const response = await fetch(
    joinBaseUrl(workspace.baseUrl, resolveOpenApiPath(path, pathParams, queryParams)),
    {
      ...requestOptions,
      method: 'GET',
      headers,
    },
  )

  if (!response.ok) {
    throw await decodeApiError(response, requestId, workspace.workspaceConnectionId)
  }

  return response
}
