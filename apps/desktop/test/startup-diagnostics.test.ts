// @vitest-environment jsdom

import { describe, expect, it, vi } from 'vitest'

import {
  describeStartupFailure,
  installRuntimeDiagnostics,
  installStartupDiagnostics,
} from '@/startup/diagnostics'

function createUnhandledRejectionEvent(reason: unknown): PromiseRejectionEvent {
  const event = new Event('unhandledrejection') as PromiseRejectionEvent
  Object.defineProperty(event, 'reason', {
    configurable: true,
    value: reason,
  })
  return event
}

describe('startup diagnostics', () => {
  it('keeps error message and stack when the startup failure is an Error', () => {
    const error = new Error('bootstrap exploded')
    error.stack = 'Error: bootstrap exploded\n    at main.ts:1:1'

    expect(describeStartupFailure(error)).toEqual({
      title: 'Desktop startup failed',
      message: 'bootstrap exploded',
      stack: 'Error: bootstrap exploded\n    at main.ts:1:1',
    })
  })

  it('falls back to a readable message for non-Error failures', () => {
    expect(describeStartupFailure('bad state')).toEqual({
      title: 'Desktop startup failed',
      message: 'bad state',
      stack: '',
    })
    expect(describeStartupFailure({ code: 'E_FAIL' })).toEqual({
      title: 'Desktop startup failed',
      message: 'Unknown startup error',
      stack: '',
    })
  })

  it('routes startup window failures to the provided reporter and removes listeners on cleanup', () => {
    const report = vi.fn()
    const teardown = installStartupDiagnostics(report)

    const startupError = new Error('startup exploded')
    window.dispatchEvent(new ErrorEvent('error', { error: startupError, message: startupError.message }))
    window.dispatchEvent(createUnhandledRejectionEvent('startup rejection'))

    expect(report).toHaveBeenCalledTimes(2)
    expect(report).toHaveBeenNthCalledWith(1, startupError, 'error')
    expect(report).toHaveBeenNthCalledWith(2, 'startup rejection', 'unhandledrejection')

    teardown()
    report.mockClear()

    window.dispatchEvent(new ErrorEvent('error', { message: 'ignored' }))
    window.dispatchEvent(createUnhandledRejectionEvent('ignored rejection'))

    expect(report).not.toHaveBeenCalled()
  })

  it('routes runtime window failures to the provided reporter and removes listeners on cleanup', () => {
    const report = vi.fn()
    const teardown = installRuntimeDiagnostics(report)

    const runtimeError = new Error('runtime exploded')
    window.dispatchEvent(new ErrorEvent('error', { error: runtimeError, message: runtimeError.message }))
    window.dispatchEvent(createUnhandledRejectionEvent('runtime rejection'))

    expect(report).toHaveBeenCalledTimes(2)
    expect(report).toHaveBeenNthCalledWith(1, runtimeError, 'error')
    expect(report).toHaveBeenNthCalledWith(2, 'runtime rejection', 'unhandledrejection')

    teardown()
    report.mockClear()

    window.dispatchEvent(new ErrorEvent('error', { message: 'ignored runtime' }))
    window.dispatchEvent(createUnhandledRejectionEvent('ignored runtime rejection'))

    expect(report).not.toHaveBeenCalled()
  })
})
