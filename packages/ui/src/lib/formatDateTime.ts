export function formatDateTime(timestamp?: number): string {
  if (timestamp == null) {
    return ''
  }

  const date = new Date(timestamp)

  if (Number.isNaN(date.getTime())) {
    return ''
  }

  return new Intl.DateTimeFormat(undefined, {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date)
}
