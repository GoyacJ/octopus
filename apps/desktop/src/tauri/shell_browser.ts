import JSZip from 'jszip'

import type {
  AvatarUploadPayload,
  CreateNotificationInput,
  CreateHostWorkspaceConnectionInput,
  ExportWorkspaceAgentBundleResult,
  HealthcheckStatus,
  HostBackendConnection,
  HostState,
  HostUpdateChannel,
  HostUpdateStatus,
  HostWorkspaceConnectionRecord,
  NotificationFilter,
  NotificationListResponse,
  NotificationRecord,
  NotificationUnreadSummary,
  ShellBootstrap,
  ShellPreferences,
  WorkspaceDirectoryUploadEntry,
  WorkspaceFileUploadPayload,
} from '@octopus/schema'
import {
  extractProjectIdFromShellRoute,
  normalizeHostUpdateStatus,
  normalizeNotificationListResponse,
  normalizeShellPreferences,
} from '@octopus/schema'

import { emitNotification } from './notifications'
import {
  fetchHostOpenApi,
  resolveBrowserHostApiBaseUrl,
  resolveBrowserHostAuthToken,
} from './shared'
import { normalizeShellBootstrap } from './workspace_connections'

type BrowserDirectoryHandle = {
  name?: string
  getDirectoryHandle: (name: string, options?: { create?: boolean }) => Promise<BrowserDirectoryHandle>
  getFileHandle: (name: string, options?: { create?: boolean }) => Promise<{
    createWritable: () => Promise<{
      write: (data: Blob | Uint8Array | string) => Promise<void>
      close: () => Promise<void>
    }>
  }>
}

function resolveBrowserHostConfig(): { baseUrl: string, authToken: string } {
  return {
    baseUrl: resolveBrowserHostApiBaseUrl(),
    authToken: resolveBrowserHostAuthToken(),
  }
}

function decodeBase64(value: string): Uint8Array {
  const decoded = window.atob(value)
  const bytes = new Uint8Array(decoded.length)
  for (let index = 0; index < decoded.length; index += 1) {
    bytes[index] = decoded.charCodeAt(index)
  }
  return bytes
}

function copyBytesToArrayBuffer(bytes: Uint8Array<ArrayBufferLike>): ArrayBuffer {
  const buffer = new ArrayBuffer(bytes.byteLength)
  new Uint8Array(buffer).set(bytes)
  return buffer
}

function contentTypeFromPath(fileName: string) {
  const lower = fileName.toLowerCase()
  if (lower.endsWith('.md')) {
    return 'text/markdown'
  }
  if (lower.endsWith('.json')) {
    return 'application/json'
  }
  if (lower.endsWith('.png')) {
    return 'image/png'
  }
  if (lower.endsWith('.jpg') || lower.endsWith('.jpeg')) {
    return 'image/jpeg'
  }
  if (lower.endsWith('.webp')) {
    return 'image/webp'
  }
  return 'application/octet-stream'
}

function trimSingleRootDirectory(entries: WorkspaceDirectoryUploadEntry[]) {
  const topLevelNames = new Set(
    entries
      .map(entry => entry.relativePath.split('/')[0])
      .filter((value): value is string => Boolean(value)),
  )
  const rootPrefix = topLevelNames.size === 1 ? `${[...topLevelNames][0]}/` : ''
  if (!rootPrefix) {
    return entries
  }
  return entries.map(entry => ({
    ...entry,
    relativePath: entry.relativePath.startsWith(rootPrefix)
      ? entry.relativePath.slice(rootPrefix.length)
      : entry.relativePath,
  }))
}

function downloadBlob(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = fileName
  anchor.style.position = 'fixed'
  anchor.style.left = '-9999px'
  document.body.append(anchor)
  anchor.click()
  anchor.remove()
  window.setTimeout(() => URL.revokeObjectURL(url), 0)
}

function mountHiddenInput(input: HTMLInputElement) {
  input.style.position = 'fixed'
  input.style.left = '-9999px'
  input.style.top = '0'
  input.style.width = '1px'
  input.style.height = '1px'
  input.style.opacity = '0'
  input.style.pointerEvents = 'none'
  document.body.append(input)
}

async function unzipBrowserArchive(file: File): Promise<WorkspaceDirectoryUploadEntry[]> {
  const archive = await JSZip.loadAsync(file)
  const entries = await Promise.all(
    Object.values(archive.files)
      .filter(item => !item.dir && !item.name.startsWith('__MACOSX/'))
      .map(async (item) => {
        const bytes = await item.async('uint8array')
        const blob = new Blob([copyBytesToArrayBuffer(bytes)], { type: contentTypeFromPath(item.name) })
        const payload = await readBrowserFile(new File([blob], item.name.split('/').pop() ?? item.name, {
          type: blob.type,
        }))
        return {
          ...payload,
          fileName: item.name.split('/').pop() ?? payload.fileName,
          relativePath: item.name.replace(/\\/g, '/'),
        } satisfies WorkspaceDirectoryUploadEntry
      }),
  )
  return trimSingleRootDirectory(entries)
}

async function saveBundleFolderInBrowser(
  exportPayload: ExportWorkspaceAgentBundleResult,
) {
  const browserWindow = window as typeof window & {
    showDirectoryPicker?: () => Promise<BrowserDirectoryHandle>
  }
  if (!browserWindow.showDirectoryPicker) {
    throw new Error('Current browser host does not support folder export')
  }

  const rootHandle = await browserWindow.showDirectoryPicker()
  for (const entry of exportPayload.files) {
    const segments = entry.relativePath.split('/').filter(Boolean)
    if (!segments.length) {
      continue
    }

    let directoryHandle = rootHandle
    for (const segment of segments.slice(0, -1)) {
      directoryHandle = await directoryHandle.getDirectoryHandle(segment, { create: true })
    }

    const fileHandle = await directoryHandle.getFileHandle(segments.at(-1)!, { create: true })
    const writable = await fileHandle.createWritable()
    await writable.write(decodeBase64(entry.dataBase64))
    await writable.close()
  }
}

async function saveBundleZipInBrowser(
  exportPayload: ExportWorkspaceAgentBundleResult,
) {
  const archive = new JSZip()
  for (const entry of exportPayload.files) {
    archive.file(entry.relativePath, decodeBase64(entry.dataBase64))
  }
  const blob = await archive.generateAsync({ type: 'blob' })
  downloadBlob(blob, `${exportPayload.rootDirName || 'templates'}.zip`)
}

async function readBrowserFile(file: File): Promise<WorkspaceFileUploadPayload> {
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = () => reject(reader.error ?? new Error('Failed to read avatar file'))
    reader.onload = () => resolve(String(reader.result ?? ''))
    reader.readAsDataURL(file)
  })

  return {
    fileName: file.name,
    contentType: file.type || 'application/octet-stream',
    byteSize: file.size,
    dataBase64: dataUrl.split(',', 2)[1] ?? '',
  }
}

async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const payload = await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/bootstrap',
    'get',
  )

  return normalizeShellBootstrap(payload, defaultWorkspaceId, defaultProjectId)
}

async function loadPreferences(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/preferences', 'get'),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostOpenApi(
      baseUrl,
      authToken,
      '/api/v1/host/preferences',
      'put',
      { body: JSON.stringify(preferences) },
    ),
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
}

async function getHostState(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostState> {
  return (await bootstrapShellHost(defaultWorkspaceId, defaultProjectId)).hostState
}

async function healthcheck(): Promise<HealthcheckStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/health', 'get')
}

async function getHostUpdateStatus(): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-status', 'get'),
  )
}

async function checkHostUpdate(channel?: HostUpdateChannel): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-check', 'post', {
      body: JSON.stringify(channel ? { channel } : {}),
    }),
  )
}

async function downloadHostUpdate(): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-download', 'post'),
  )
}

async function installHostUpdate(): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-install', 'post'),
  )
}

async function restartDesktopBackend(): Promise<void> {
  throw new Error('Browser host runtime does not support desktop backend restart')
}

async function resolveDesktopBackendConnection(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostBackendConnection | undefined> {
  return (await bootstrapShellHost(defaultWorkspaceId, defaultProjectId)).backend
}

async function pickAvatarImage(): Promise<AvatarUploadPayload | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = 'image/png,image/jpeg,image/webp'

  return await new Promise<AvatarUploadPayload | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', async () => {
      const file = input.files?.[0]
      input.remove()
      if (!file) {
        resolve(null)
        return
      }

      resolve(await readBrowserFile(file))
    }, { once: true })
    input.click()
  })
}

async function pickResourceDirectory(): Promise<string | null> {
  const browserWindow = window as typeof window & {
    showDirectoryPicker?: () => Promise<BrowserDirectoryHandle>
  }
  if (browserWindow.showDirectoryPicker) {
    const handle = await browserWindow.showDirectoryPicker()
    return handle.name ? `/${handle.name}` : null
  }

  const input = document.createElement('input')
  input.type = 'file'
  input.setAttribute('webkitdirectory', 'true')
  input.multiple = true

  return await new Promise<string | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', () => {
      const firstFile = input.files?.[0]
      input.remove()
      if (!firstFile) {
        resolve(null)
        return
      }

      const relativePath = (firstFile.webkitRelativePath || firstFile.name).replace(/\\/g, '/')
      const rootName = relativePath.split('/')[0]?.trim()
      resolve(rootName ? `/${rootName}` : null)
    }, { once: true })
    input.click()
  })
}

async function pickResourceFile(): Promise<WorkspaceFileUploadPayload | null> {
  const input = document.createElement('input')
  input.type = 'file'

  return await new Promise<WorkspaceFileUploadPayload | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', async () => {
      const file = input.files?.[0]
      input.remove()
      if (!file) {
        resolve(null)
        return
      }

      resolve(await readBrowserFile(file))
    }, { once: true })
    input.click()
  })
}

async function pickResourceFolder(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.setAttribute('webkitdirectory', 'true')
  input.multiple = true

  return await new Promise<WorkspaceDirectoryUploadEntry[] | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', async () => {
      const files = Array.from(input.files ?? [])
      input.remove()
      if (!files.length) {
        resolve(null)
        return
      }

      resolve(await Promise.all(files.map(async (file) => ({
        ...(await readBrowserFile(file)),
        relativePath: (file.webkitRelativePath || file.name).replace(/\\/g, '/'),
      }))))
    }, { once: true })
    input.click()
  })
}

async function pickSkillArchive(): Promise<WorkspaceFileUploadPayload[] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.zip,application/zip'
  input.multiple = true

  return await new Promise<WorkspaceFileUploadPayload[] | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', async () => {
      const files = Array.from(input.files ?? [])
      input.remove()
      if (!files.length) {
        resolve(null)
        return
      }

      resolve(await Promise.all(files.map(readBrowserFile)))
    }, { once: true })
    input.click()
  })
}

async function pickSkillFolder(): Promise<WorkspaceDirectoryUploadEntry[][] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.setAttribute('webkitdirectory', 'true')
  input.multiple = true

  return await new Promise<WorkspaceDirectoryUploadEntry[][] | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', async () => {
      const files = Array.from(input.files ?? [])
      input.remove()
      if (!files.length) {
        resolve(null)
        return
      }

      const payloads = await Promise.all(files.map(async (file) => ({
        ...(await readBrowserFile(file)),
        relativePath: (file.webkitRelativePath || file.name).replace(/\\/g, '/'),
      })))
      const grouped = new Map<string, WorkspaceDirectoryUploadEntry[]>()
      for (const payload of payloads) {
        const groupKey = payload.relativePath.split('/')[0] ?? payload.fileName
        grouped.set(groupKey, [...(grouped.get(groupKey) ?? []), payload])
      }
      resolve(Array.from(grouped.values()))
    }, { once: true })
    input.click()
  })
}

async function pickAgentBundleFolder(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.setAttribute('webkitdirectory', 'true')
  input.multiple = true

  return await new Promise<WorkspaceDirectoryUploadEntry[] | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', async () => {
      const files = Array.from(input.files ?? [])
      input.remove()
      if (!files.length) {
        resolve(null)
        return
      }

      const payloads = await Promise.all(files.map(async (file) => ({
        ...(await readBrowserFile(file)),
        relativePath: (file.webkitRelativePath || file.name).replace(/\\/g, '/'),
      })))
      const topLevelNames = new Set(
        payloads
          .map(file => file.relativePath.split('/')[0])
          .filter((value): value is string => Boolean(value)),
      )
      const rootPrefix = topLevelNames.size === 1 ? `${[...topLevelNames][0]}/` : ''

      resolve(payloads.map(file => ({
        ...file,
        relativePath: rootPrefix && file.relativePath.startsWith(rootPrefix)
          ? file.relativePath.slice(rootPrefix.length)
          : file.relativePath,
      })))
    }, { once: true })
    input.click()
  })
}

async function pickAgentBundleArchive(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.zip,application/zip'

  return await new Promise<WorkspaceDirectoryUploadEntry[] | null>((resolve) => {
    mountHiddenInput(input)
    input.addEventListener('change', async () => {
      const file = input.files?.[0]
      input.remove()
      if (!file) {
        resolve(null)
        return
      }

      resolve(await unzipBrowserArchive(file))
    }, { once: true })
    input.click()
  })
}

async function saveAgentBundleExport(
  exportPayload: ExportWorkspaceAgentBundleResult,
  format: 'folder' | 'zip',
): Promise<void> {
  if (format === 'folder') {
    await saveBundleFolderInBrowser(exportPayload)
    return
  }
  await saveBundleZipInBrowser(exportPayload)
}

async function listWorkspaceConnections(): Promise<HostWorkspaceConnectionRecord[]> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/workspace-connections', 'get')
}

async function createWorkspaceConnection(
  input: CreateHostWorkspaceConnectionInput,
): Promise<HostWorkspaceConnectionRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/workspace-connections', 'post', {
    body: JSON.stringify(input),
  })
}

async function deleteWorkspaceConnection(workspaceConnectionId: string): Promise<void> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/workspace-connections/{connectionId}',
    'delete',
    {
      pathParams: {
        connectionId: workspaceConnectionId,
      },
    },
  )
}

async function listNotifications(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationListResponse> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeNotificationListResponse(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications', 'get', {
      queryParams: {
        scope: filter.scope,
      },
    }),
  )
}

async function createNotification(input: CreateNotificationInput): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const notification = await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications', 'post', {
    body: JSON.stringify(input),
  })
  emitNotification(notification)
  return notification
}

async function markNotificationRead(id: string): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/notifications/{notificationId}/read',
    'post',
    {
      pathParams: {
        notificationId: id,
      },
    },
  )
}

async function markAllNotificationsRead(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationUnreadSummary> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications/read-all', 'post', {
    body: JSON.stringify(filter),
  })
}

async function dismissNotificationToast(id: string): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/notifications/{notificationId}/dismiss-toast',
    'post',
    {
      pathParams: {
        notificationId: id,
      },
    },
  )
}

async function getNotificationUnreadSummary(): Promise<NotificationUnreadSummary> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications/unread-summary', 'get')
}

export const browserShellClient = {
  bootstrapShellHost,
  loadPreferences,
  savePreferences,
  getHostState,
  getHostUpdateStatus,
  checkHostUpdate,
  downloadHostUpdate,
  installHostUpdate,
  healthcheck,
  restartDesktopBackend,
  resolveDesktopBackendConnection,
  pickAvatarImage,
  pickResourceDirectory,
  pickResourceFile,
  pickResourceFolder,
  pickSkillArchive,
  pickSkillFolder,
  pickAgentBundleArchive,
  pickAgentBundleFolder,
  saveAgentBundleExport,
  listWorkspaceConnections,
  createWorkspaceConnection,
  deleteWorkspaceConnection,
  listNotifications,
  createNotification,
  markNotificationRead,
  markAllNotificationsRead,
  dismissNotificationToast,
  getNotificationUnreadSummary,
}
