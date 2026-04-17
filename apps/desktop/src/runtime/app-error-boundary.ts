import { reactive, readonly } from 'vue'
import type { App } from 'vue'
import type { Router } from 'vue-router'

import { installRuntimeDiagnostics } from '@/startup/diagnostics'

export type RuntimeAppErrorSource = 'component' | 'router' | 'window' | 'unhandledrejection'

export type RuntimeAppErrorRecord = {
  id: number
  name: string
  message: string
  stack: string
  source: RuntimeAppErrorSource
  info: string
  occurredAt: number
}

type RuntimeAppErrorState = {
  current: RuntimeAppErrorRecord | null
  resetToken: number
}

const state = reactive<RuntimeAppErrorState>({
  current: null,
  resetToken: 0,
})

let nextErrorId = 0

function normalizeRuntimeError(error: unknown) {
  if (error instanceof Error) {
    return {
      name: error.name || 'Error',
      message: error.message || 'Unknown runtime error',
      stack: error.stack ?? '',
    }
  }

  if (typeof error === 'string') {
    return {
      name: 'Error',
      message: error,
      stack: '',
    }
  }

  return {
    name: 'Error',
    message: 'Unknown runtime error',
    stack: '',
  }
}

export function reportRuntimeAppError(
  error: unknown,
  options: {
    source: RuntimeAppErrorSource
    info?: string
  },
): void {
  const normalized = normalizeRuntimeError(error)
  state.current = {
    id: nextErrorId += 1,
    name: normalized.name,
    message: normalized.message,
    stack: normalized.stack,
    source: options.source,
    info: options.info ?? '',
    occurredAt: Date.now(),
  }
}

export function clearRuntimeAppError(): void {
  state.current = null
}

export function retryRuntimeAppSurface(): void {
  state.current = null
  state.resetToken += 1
}

export function formatRuntimeAppErrorDetails(error: RuntimeAppErrorRecord): string {
  return [
    `Source: ${error.source}`,
    `Occurred At: ${new Date(error.occurredAt).toISOString()}`,
    `Name: ${error.name}`,
    `Message: ${error.message}`,
    error.info ? `Info: ${error.info}` : '',
    error.stack ? `Stack:\n${error.stack}` : '',
  ]
    .filter(Boolean)
    .join('\n')
}

export function installRuntimeAppErrorHandling(app: App, router: Router): () => void {
  app.config.errorHandler = (error, _instance, info) => {
    reportRuntimeAppError(error, {
      source: 'component',
      info,
    })
  }

  const stopRuntimeDiagnostics = installRuntimeDiagnostics((error, source) => {
    reportRuntimeAppError(error, {
      source: source === 'unhandledrejection' ? 'unhandledrejection' : 'window',
    })
  })

  const stopRouterErrorHandling = router.onError((error) => {
    reportRuntimeAppError(error, {
      source: 'router',
    })
  })

  return () => {
    stopRuntimeDiagnostics()
    stopRouterErrorHandling()
  }
}

export function resetRuntimeAppErrorState(): void {
  state.current = null
  state.resetToken = 0
  nextErrorId = 0
}

export const runtimeAppErrorState = readonly(state)
