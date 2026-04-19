import type { WorkspaceConnectionRecord } from '@octopus/schema'

type Translate = (key: string) => string

export function resolveWorkspaceLabel(
  connection: WorkspaceConnectionRecord | null | undefined,
  activeWorkspaceName: string | null | undefined,
  t: Translate,
): string {
  const normalizedName = activeWorkspaceName?.trim()
  const normalizedConnectionLabel = connection?.label?.trim()

  if (normalizedName && normalizedName !== normalizedConnectionLabel) {
    return normalizedName
  }

  if (connection?.transportSecurity === 'loopback') {
    return t('topbar.localWorkspace')
  }

  return connection?.label ?? normalizedName ?? t('common.workspace')
}
