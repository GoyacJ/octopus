function fallbackCopyText(value: string): boolean {
  const textarea = document.createElement('textarea')
  textarea.value = value
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.opacity = '0'
  document.body.appendChild(textarea)
  textarea.select()
  const copied = document.execCommand('copy')
  document.body.removeChild(textarea)
  return copied
}

export async function copyTextToClipboard(value: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value)
    return
  }

  if (!fallbackCopyText(value)) {
    throw new Error('Clipboard copy failed')
  }
}

export function buildRoutePermalink(routeFullPath: string): string {
  if (typeof window === 'undefined') {
    return routeFullPath
  }

  const normalizedPath = routeFullPath.startsWith('/') ? routeFullPath : `/${routeFullPath}`
  const url = new URL(window.location.href)
  url.hash = `#${normalizedPath}`
  return url.toString()
}
