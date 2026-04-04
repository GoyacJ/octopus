import i18n from '@/plugins/i18n'

type TranslationValues = Record<string, string | number>
type RunDisplayField = 'currentStep' | 'nextAction'

function hasKey(key: string): boolean {
  return i18n.global.te(key)
}

export function mockKey(scope: string, id: string, field: string, fallback = ''): string {
  const key = `mock.${scope}.${id}.${field}`
  return hasKey(key) ? key : fallback
}

export function resolveMockField(
  scope: string,
  id: string,
  field: string,
  fallback = '',
  values?: TranslationValues,
): string {
  return resolveCopy(mockKey(scope, id, field, fallback), values)
}

export function translate(key: string, values?: TranslationValues): string {
  if (!hasKey(key)) {
    return key
  }

  return String(i18n.global.t(key, values ?? {}))
}

export function resolveCopy(value?: string | null, values?: TranslationValues): string {
  if (!value) {
    return ''
  }

  return hasKey(value) ? translate(value, values) : value
}

export function resolveRunDisplayValue(
  value?: string | null,
  options?: { runId?: string, field?: RunDisplayField },
  values?: TranslationValues,
): string {
  if (!value) {
    return ''
  }

  if (hasKey(value)) {
    return resolveCopy(value, values)
  }

  const resolvedValue = options?.runId && options.field
    ? mockKey('run', options.runId, options.field, value)
    : value

  return resolveCopy(resolvedValue, values)
}

export function enumLabel(group: string, value?: string | null): string {
  if (!value) {
    return ''
  }

  const key = `enum.${group}.${value}`
  return hasKey(key) ? translate(key) : value
}

export function formatDateTime(timestamp?: number): string {
  if (!timestamp) {
    return translate('common.na')
  }

  return new Date(timestamp).toLocaleString(i18n.global.locale.value, {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

export function countLabel(key: string, count: number): string {
  return translate(key, { count })
}
