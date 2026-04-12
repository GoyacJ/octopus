import type {
  ShellBootstrap,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'

export const WORKSPACE_CONNECTIONS: WorkspaceConnectionRecord[] = [
  {
    workspaceConnectionId: 'conn-local',
    workspaceId: 'ws-local',
    label: 'Local Workspace',
    baseUrl: 'http://127.0.0.1:43127',
    transportSecurity: 'loopback',
    authMode: 'session-token',
    status: 'connected',
  },
  {
    workspaceConnectionId: 'conn-enterprise',
    workspaceId: 'ws-enterprise',
    label: 'Enterprise Workspace',
    baseUrl: 'https://enterprise.example.test',
    transportSecurity: 'trusted',
    authMode: 'session-token',
    status: 'connected',
  },
]

export const WORKSPACE_SESSIONS: WorkspaceSessionTokenEnvelope[] = WORKSPACE_CONNECTIONS.map(connection => ({
  workspaceConnectionId: connection.workspaceConnectionId,
  token: `token-${connection.workspaceId}`,
  issuedAt: 1,
  session: {
    id: `sess-${connection.workspaceId}`,
    workspaceId: connection.workspaceId,
    userId: 'user-owner',
    clientAppId: 'octopus-desktop',
    token: `token-${connection.workspaceId}`,
    status: 'active',
    createdAt: 1,
    expiresAt: undefined,
  },
}))

export function createWorkspaceSessionEnvelope(
  connection: WorkspaceConnectionRecord,
  userId = 'user-owner',
): WorkspaceSessionTokenEnvelope {
  return {
    workspaceConnectionId: connection.workspaceConnectionId,
    token: `token-${connection.workspaceId}-${userId}`,
    issuedAt: 1,
    session: {
      id: `sess-${connection.workspaceId}-${userId}`,
      workspaceId: connection.workspaceId,
      userId,
      clientAppId: 'octopus-desktop',
      token: `token-${connection.workspaceId}-${userId}`,
      status: 'active',
      createdAt: 1,
      expiresAt: undefined,
    },
  }
}

export function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

export function createHostBootstrap(locale = 'zh-CN'): ShellBootstrap {
  return {
    hostState: {
      platform: 'tauri',
      mode: 'local',
      appVersion: '0.1.0-test',
      cargoWorkspace: true,
      shell: 'tauri2',
    },
    preferences: {
      theme: 'system',
      locale,
      compactSidebar: false,
      leftSidebarCollapsed: false,
      rightSidebarCollapsed: false,
      updateChannel: 'formal',
      fontSize: 16,
      fontFamily: 'Inter, sans-serif',
      fontStyle: 'sans',
      defaultWorkspaceId: 'ws-local',
      lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
    },
    connections: [
      {
        id: 'conn-local',
        mode: 'local',
        label: 'Local Workspace',
        workspaceId: 'ws-local',
        state: 'local-ready',
      },
      {
        id: 'conn-enterprise',
        mode: 'shared',
        label: 'Enterprise Workspace',
        workspaceId: 'ws-enterprise',
        state: 'connected',
        baseUrl: 'https://enterprise.example.test',
      },
    ],
    backend: {
      baseUrl: 'http://127.0.0.1:43127',
      authToken: 'desktop-test-token',
      state: 'ready',
      transport: 'http',
    },
    workspaceConnections: clone(WORKSPACE_CONNECTIONS),
  }
}
