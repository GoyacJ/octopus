import type {
  HostReleaseSummary as OpenApiHostReleaseSummary,
  HostUpdateCapabilities as OpenApiHostUpdateCapabilities,
  HostUpdateChannel as OpenApiHostUpdateChannel,
  HostUpdateProgress as OpenApiHostUpdateProgress,
  HostUpdateState as OpenApiHostUpdateState,
  HostUpdateStatus as OpenApiHostUpdateStatus,
} from './generated'

export type HostUpdateChannel = OpenApiHostUpdateChannel
export type HostUpdateState = OpenApiHostUpdateState
export type HostReleaseSummary = OpenApiHostReleaseSummary
export type HostUpdateCapabilities = OpenApiHostUpdateCapabilities
export type HostUpdateProgress = OpenApiHostUpdateProgress

export interface HostUpdateStatus extends Omit<OpenApiHostUpdateStatus, 'latestRelease' | 'progress' | 'errorCode' | 'errorMessage' | 'lastCheckedAt'> {
  latestRelease: HostReleaseSummary | null
  progress: HostUpdateProgress | null
  errorCode: string | null
  errorMessage: string | null
  lastCheckedAt: number | null
}

export function createDefaultHostUpdateCapabilities(): HostUpdateCapabilities {
  return {
    canCheck: true,
    canDownload: false,
    canInstall: false,
    supportsChannels: true,
  }
}

export function createDefaultHostUpdateStatus(
  overrides: Partial<HostUpdateStatus> = {},
): HostUpdateStatus {
  return {
    currentVersion: '0.1.0',
    currentChannel: 'formal',
    state: 'idle',
    latestRelease: null,
    lastCheckedAt: null,
    progress: null,
    capabilities: createDefaultHostUpdateCapabilities(),
    errorCode: null,
    errorMessage: null,
    ...overrides,
  }
}

export function normalizeHostUpdateStatus(
  status?: Partial<OpenApiHostUpdateStatus> | null,
): HostUpdateStatus {
  return createDefaultHostUpdateStatus({
    currentVersion: status?.currentVersion ?? '0.1.0',
    currentChannel: status?.currentChannel ?? 'formal',
    state: status?.state ?? 'idle',
    latestRelease: status?.latestRelease ?? null,
    lastCheckedAt: typeof status?.lastCheckedAt === 'number' ? status.lastCheckedAt : null,
    progress: status?.progress ?? null,
    capabilities: {
      ...createDefaultHostUpdateCapabilities(),
      ...status?.capabilities,
    },
    errorCode: status?.errorCode ?? null,
    errorMessage: status?.errorMessage ?? null,
  })
}
