import type { Router } from 'vue-router'

export function resolveInitialRoute(): string {
  if (typeof window === 'undefined') {
    return '/'
  }

  const hashPath = window.location.hash.replace(/^#/, '')
  return hashPath || '/'
}

function normalizeInitialHash(initialRoute: string): void {
  if (typeof window === 'undefined' || window.location.hash) {
    return
  }

  window.location.hash = initialRoute
}

export async function prepareRouterStartup(router: Router): Promise<void> {
  const initialRoute = resolveInitialRoute()
  normalizeInitialHash(initialRoute)
  await router.push(initialRoute)
  await router.isReady()
}
