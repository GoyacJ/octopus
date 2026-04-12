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

export function resolveRunDisplayValue(
  value?: string | null,
  values?: TranslationValues,
): string {
  if (!value) {
    return ''
  }

  return resolveCopy(value, values)
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

export function formatList(values: Array<string | null | undefined>): string {
  const items = values.filter((value): value is string => Boolean(value?.trim())).map(value => value.trim())
  if (!items.length) {
    return ''
  }

  if (i18n.global.locale.value.startsWith('zh')) {
    return items.join('、')
  }

  if (typeof Intl !== 'undefined' && typeof Intl.ListFormat === 'function') {
    return new Intl.ListFormat(i18n.global.locale.value, {
      style: 'long',
      type: 'conjunction',
    }).format(items)
  }

  return items.join(', ')
}
