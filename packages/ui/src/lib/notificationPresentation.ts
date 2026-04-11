import type { NotificationLevel } from '@octopus/schema'
import { Bell, CheckCircle2, CircleAlert, Info } from 'lucide-vue-next'

type NotificationPresentation = {
  dotClass: string
  rowClass: string
  titleClass: string
  toastSurfaceClass: string
  toastIconClass: string
  toastTitleClass: string
}

export const notificationLevelIcons = {
  info: Info,
  success: CheckCircle2,
  warning: CircleAlert,
  error: Bell,
} as const

const notificationPresentations: Record<NotificationLevel, NotificationPresentation> = {
  info: {
    dotClass: 'bg-status-info',
    rowClass: 'border-l-2 border-l-status-info',
    titleClass: 'text-text-primary',
    toastSurfaceClass: 'border-[color-mix(in_srgb,var(--color-status-info)_22%,var(--border))]',
    toastIconClass: 'text-status-info',
    toastTitleClass: 'text-text-primary',
  },
  success: {
    dotClass: 'bg-status-success',
    rowClass: 'border-l-2 border-l-status-success',
    titleClass: 'text-text-primary',
    toastSurfaceClass: 'border-[color-mix(in_srgb,var(--color-status-success)_22%,var(--border))]',
    toastIconClass: 'text-status-success',
    toastTitleClass: 'text-text-primary',
  },
  warning: {
    dotClass: 'bg-status-warning',
    rowClass: 'border-l-2 border-l-status-warning bg-[color-mix(in_srgb,var(--color-status-warning-soft)_42%,var(--bg-surface))] hover:bg-[color-mix(in_srgb,var(--color-status-warning-soft)_64%,var(--bg-subtle))]',
    titleClass: 'text-status-warning',
    toastSurfaceClass: 'border-[color-mix(in_srgb,var(--color-status-warning)_22%,var(--border))] bg-[color-mix(in_srgb,var(--color-status-warning-soft)_48%,var(--bg-popover))]',
    toastIconClass: 'text-status-warning',
    toastTitleClass: 'text-status-warning',
  },
  error: {
    dotClass: 'bg-status-error',
    rowClass: 'border-l-2 border-l-status-error bg-[color-mix(in_srgb,var(--color-status-error-soft)_42%,var(--bg-surface))] hover:bg-[color-mix(in_srgb,var(--color-status-error-soft)_64%,var(--bg-subtle))]',
    titleClass: 'text-status-error',
    toastSurfaceClass: 'border-[color-mix(in_srgb,var(--color-status-error)_22%,var(--border))] bg-[color-mix(in_srgb,var(--color-status-error-soft)_48%,var(--bg-popover))]',
    toastIconClass: 'text-status-error',
    toastTitleClass: 'text-status-error',
  },
}

export function getNotificationPresentation(level?: NotificationLevel | null): NotificationPresentation {
  if (!level) {
    return notificationPresentations.info
  }

  return notificationPresentations[level]
}
