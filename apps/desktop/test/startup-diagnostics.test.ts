import { describe, expect, it } from 'vitest'

import { describeStartupFailure } from '@/startup/diagnostics'

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
})
