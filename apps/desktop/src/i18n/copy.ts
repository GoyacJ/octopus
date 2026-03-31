import i18n from '@/plugins/i18n'

type TranslationValues = Record<string, string | number>

function hasKey(key: string): boolean {
  return i18n.global.te(key)
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

export function resolveMockField(entity: string, id: string | undefined, field: string, fallback = ''): string {
  if (!id) {
    return fallback
  }

  const key = mockKey(entity, id, field, fallback)
  return hasKey(key) ? translate(key) : resolveCopy(fallback)
}

export function resolveMockList(entity: string, id: string | undefined, field: string, fallback: string[] = []): string[] {
  if (!id) {
    return fallback.map((item) => resolveCopy(item))
  }

  return fallback.map((item, index) => {
    const key = `mock.${entity}.${id}.${field}.${index}`
    return hasKey(key) ? translate(key) : resolveCopy(item)
  })
}

export function mockKey(entity: string, id: string | undefined, field: string, fallback = ''): string {
  if (!id) {
    return fallback
  }

  const key = `mock.${entity}.${id}.${field}`
  return hasKey(key) ? key : fallback
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
