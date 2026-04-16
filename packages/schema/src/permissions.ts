import type { PermissionMode } from './shared'

export type RuntimePermissionMode = 'read-only' | 'workspace-write' | 'danger-full-access'

export const RUNTIME_PERMISSION_MODE_BY_UI_MODE: Record<PermissionMode, RuntimePermissionMode> = {
  readonly: 'read-only',
  auto: 'workspace-write',
  'danger-full-access': 'danger-full-access',
}

export function resolveRuntimePermissionMode(mode: PermissionMode | RuntimePermissionMode): RuntimePermissionMode {
  if (isRuntimePermissionMode(mode)) {
    return mode
  }

  return RUNTIME_PERMISSION_MODE_BY_UI_MODE[mode]
}

export const UI_PERMISSION_MODE_BY_RUNTIME_MODE: Record<RuntimePermissionMode, PermissionMode> = {
  'read-only': 'readonly',
  'workspace-write': 'auto',
  'danger-full-access': 'danger-full-access',
}

export function resolveUiPermissionMode(mode: PermissionMode | RuntimePermissionMode): PermissionMode {
  if (!isRuntimePermissionMode(mode)) {
    return mode
  }

  return UI_PERMISSION_MODE_BY_RUNTIME_MODE[mode]
}

export function isRuntimePermissionMode(value: string): value is RuntimePermissionMode {
  return value === 'read-only' || value === 'workspace-write' || value === 'danger-full-access'
}
