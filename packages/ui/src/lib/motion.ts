// Calm Intelligence motion stays restrained: no bounce, no overshoot.
export const MOTION_DURATIONS = {
  fast: 'var(--duration-fast)',
  normal: 'var(--duration-normal)',
  slow: 'var(--duration-slow)',
} as const

export const MOTION_EASINGS = {
  apple: 'var(--ease-apple)',
  inOut: 'var(--ease-in-out)',
} as const

export type MotionDurationKey = keyof typeof MOTION_DURATIONS
export type MotionEasingKey = keyof typeof MOTION_EASINGS

export function makeTransition(
  property: string | string[],
  duration: MotionDurationKey = 'normal',
  easing: MotionEasingKey = 'apple',
): string {
  const properties = Array.isArray(property) ? property : [property]
  const transitionDuration = MOTION_DURATIONS[duration]
  const transitionEasing = MOTION_EASINGS[easing]

  return properties
    .map((currentProperty) => `${currentProperty} ${transitionDuration} ${transitionEasing}`)
    .join(', ')
}

export function prefersReducedMotion(): boolean {
  if (typeof globalThis === 'undefined' || typeof globalThis.matchMedia !== 'function') {
    return false
  }

  return globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches
}
