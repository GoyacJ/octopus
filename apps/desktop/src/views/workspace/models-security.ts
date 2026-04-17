import type { RuntimeSecretReferenceStatus } from '@octopus/schema'

type Translate = (key: string, params?: Record<string, unknown>) => string

export type ModelCredentialSecurityState =
  | 'missing'
  | 'configured'
  | 'pending-save'
  | 'environment-variable'
  | 'environment-variable-missing'
  | 'system-managed'
  | 'system-managed-missing'
  | 'migration-failed'
  | 'inline-redacted'
  | 'reference-error'

const SECRET_REFERENCE_PREFIXES = ['env:', 'secret-ref:', 'keychain:', 'op://', 'vault:']

export function getConfiguredModelCredentialStatus(
  secretReferences: RuntimeSecretReferenceStatus[] | undefined,
  configuredModelId: string | undefined,
): RuntimeSecretReferenceStatus | null {
  if (!configuredModelId) {
    return null
  }

  return secretReferences?.find(reference =>
    reference.path === `configuredModels.${configuredModelId}.credentialRef`)
    ?? null
}

export function resolveModelCredentialSecurityState(input: {
  credentialRef?: string | null
  secretStatus?: RuntimeSecretReferenceStatus | null
  hasPendingApiKey?: boolean
}): ModelCredentialSecurityState {
  const credentialRef = input.credentialRef?.trim() ?? ''
  const secretStatus = input.secretStatus?.status

  if (input.hasPendingApiKey) {
    return 'pending-save'
  }

  if (secretStatus === 'migration-failed') {
    return 'migration-failed'
  }
  if (secretStatus === 'reference-error') {
    return 'reference-error'
  }
  if (secretStatus === 'inline-redacted') {
    return 'inline-redacted'
  }

  if (!credentialRef) {
    return 'missing'
  }

  if (credentialRef.startsWith('env:')) {
    return secretStatus === 'reference-missing'
      ? 'environment-variable-missing'
      : 'environment-variable'
  }

  if (credentialRef.startsWith('secret-ref:')) {
    return secretStatus === 'reference-missing'
      ? 'system-managed-missing'
      : 'system-managed'
  }

  if (SECRET_REFERENCE_PREFIXES.some(prefix => credentialRef.startsWith(prefix))) {
    return 'configured'
  }

  return 'configured'
}

export function resolveModelCredentialTone(state: ModelCredentialSecurityState): 'info' | 'warning' | 'error' {
  switch (state) {
    case 'missing':
    case 'environment-variable-missing':
    case 'system-managed-missing':
    case 'migration-failed':
    case 'reference-error':
      return 'error'
    case 'configured':
    case 'pending-save':
    case 'inline-redacted':
      return 'warning'
    default:
      return 'info'
  }
}

export function isModelCredentialBlocked(state: ModelCredentialSecurityState): boolean {
  return state === 'migration-failed'
    || state === 'inline-redacted'
    || state === 'reference-error'
}

export function localizeModelCredentialLabel(
  t: Translate,
  state: ModelCredentialSecurityState,
): string {
  return t(`models.security.states.${state}`)
}

export function localizeModelCredentialDescription(
  t: Translate,
  state: ModelCredentialSecurityState,
): string {
  return t(`models.security.descriptions.${state}`)
}

export function localizeModelRuntimeMessage(
  t: Translate,
  message: string,
  fallbackKey = 'models.messages.genericError',
): string {
  const trimmed = message.trim()
  if (!trimmed) {
    return t(fallbackKey)
  }

  const unknownKeyMatch = trimmed.match(/unknown runtime config key `([^`]+)`/i)
  if (unknownKeyMatch) {
    return t('models.messages.unknownRuntimeConfigKey', {
      key: unknownKeyMatch[1],
    })
  }

  const deprecatedKeyMatch = trimmed.match(/deprecated runtime config key `([^`]+)`; use `([^`]+)` instead/i)
  if (deprecatedKeyMatch) {
    return t('models.messages.deprecatedRuntimeConfigKey', {
      key: deprecatedKeyMatch[1],
      replacement: deprecatedKeyMatch[2],
    })
  }

  const missingEnvMatch = trimmed.match(/missing configured credential env var `([^`]+)` for provider `([^`]+)`/i)
  if (missingEnvMatch) {
    return t('models.messages.missingConfiguredCredentialEnv', {
      envKey: missingEnvMatch[1],
      providerId: missingEnvMatch[2],
    })
  }

  const invalidRuntimeConfigMatch = trimmed.match(/invalid runtime config:\s*(.+)$/i)
  if (invalidRuntimeConfigMatch) {
    return t('models.messages.invalidRuntimeConfig', {
      message: invalidRuntimeConfigMatch[1],
    })
  }

  if (/runtime config document must be a JSON object/i.test(trimmed)) {
    return t('models.messages.runtimeConfigMustBeObject')
  }

  if (/runtime config patch must be a JSON object/i.test(trimmed)) {
    return t('models.messages.runtimePatchMustBeObject')
  }

  if (/failed to validate configured model/i.test(trimmed)) {
    return t('models.messages.validateFailed')
  }

  if (/failed to validate runtime config/i.test(trimmed)) {
    return t('models.messages.validateFailed')
  }

  if (/failed to save runtime config/i.test(trimmed)) {
    return t('models.messages.saveFailed')
  }

  if (/no active workspace connection selected/i.test(trimmed)) {
    return t('models.messages.noWorkspaceConnection')
  }

  if (/settings only supports workspace runtime configuration/i.test(trimmed)) {
    return t('models.messages.workspaceOnly')
  }

  return t(fallbackKey)
}

export function localizeModelRuntimeMessages(
  t: Translate,
  messages: string[],
  fallbackKey = 'models.messages.genericError',
): string[] {
  return messages.map(message => localizeModelRuntimeMessage(t, message, fallbackKey))
}
