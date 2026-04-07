import type { WorkspaceConnectionRecord } from '@octopus/schema'

type Translate = (key: string) => string

export function resolveWorkspaceLabel(
  connection: WorkspaceConnectionRecord | null | undefined,
  activeWorkspaceName: string | null | undefined,
  t: Translate,
): string {
  if (connection?.transportSecurity === 'loopback') {
    return t('topbar.localWorkspace')
  }

  return connection?.label ?? activeWorkspaceName ?? t('common.workspace')
}
