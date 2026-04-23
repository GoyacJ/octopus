<script setup lang="ts">
import { computed } from 'vue'
import {
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from 'reka-ui'

import { prefersReducedMotion } from '../lib/motion'
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  open?: boolean
  title?: string
  description?: string
  closeLabel?: string
  contentClass?: string
  bodyClass?: string
  footerClass?: string
  contentTestId?: string
  bodyTestId?: string
  respectReducedMotion?: boolean
  reducedMotion?: boolean
}>(), {
  open: false,
  title: '',
  description: '',
  closeLabel: 'Close',
  contentClass: '',
  bodyClass: '',
  footerClass: '',
  contentTestId: 'ui-dialog-content',
  bodyTestId: 'ui-dialog-body',
  respectReducedMotion: true,
  reducedMotion: undefined,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const accessibleTitle = computed(() => props.title || props.closeLabel)
const visibleDescription = computed(() => props.description?.trim() ?? '')
const accessibleDescription = computed(() => visibleDescription.value || accessibleTitle.value)
const reducedMotionActive = computed(() =>
  props.respectReducedMotion !== false && (props.reducedMotion ?? prefersReducedMotion()),
)
const reducedMotionState = computed(() => (reducedMotionActive.value ? 'true' : 'false'))
const overlayClasses = computed(() => cn(
  'fixed inset-0 z-50 bg-[var(--color-overlay)]',
  reducedMotionActive.value ? 'transition-none' : 'transition-opacity',
))
</script>

<template>
  <DialogRoot :open="props.open" @update:open="emit('update:open', $event)">
    <DialogPortal>
      <DialogOverlay :class="overlayClasses" :data-reduced-motion="reducedMotionState" />
      <DialogContent
        :data-testid="props.contentTestId"
        data-ui-dialog-content="true"
        :data-reduced-motion="reducedMotionState"
        :class="cn(
          'fixed left-1/2 top-1/2 z-50 flex max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-2xl -translate-x-1/2 -translate-y-1/2 flex-col gap-4 overflow-hidden rounded-[var(--radius-xl)] border border-border bg-popover p-5 shadow-lg md:w-full md:p-6',
          props.contentClass,
        )"
      >
        <DialogTitle class="sr-only">
          {{ accessibleTitle }}
        </DialogTitle>
        <DialogDescription class="sr-only">
          {{ accessibleDescription }}
        </DialogDescription>

        <header
          v-if="$slots.header || props.title || props.description"
          class="shrink-0 flex items-start justify-between gap-3 border-b border-border pb-2"
        >
          <div class="min-w-0 flex-1">
            <slot name="header">
              <div class="space-y-1.5">
                <div class="text-section-title font-bold tracking-[-0.02em] text-text-primary">
                  {{ accessibleTitle }}
                </div>
                <div v-if="visibleDescription" class="text-body leading-relaxed text-text-secondary">
                  {{ visibleDescription }}
                </div>
              </div>
            </slot>
          </div>

          <DialogClose as-child>
            <button
              type="button"
              class="ui-focus-ring inline-flex size-7 shrink-0 items-center justify-center rounded-[var(--radius-xs)] text-text-tertiary transition-colors hover:bg-subtle hover:text-text-primary"
              data-testid="ui-dialog-close"
              :aria-label="props.closeLabel"
            >
              ×
            </button>
          </DialogClose>
        </header>

        <div
          :data-testid="props.bodyTestId"
          data-ui-dialog-body="true"
          :class="cn('min-h-0 min-w-0 flex-1 overflow-y-auto text-text-primary', props.bodyClass)"
        >
          <slot />
        </div>

        <footer
          v-if="$slots.footer"
          :class="cn('shrink-0 flex justify-end gap-2 pt-2', props.footerClass)"
        >
          <slot name="footer" />
        </footer>

        <section
          v-if="$slots.danger"
          class="mt-2 shrink-0 flex flex-col gap-3 rounded-[var(--radius-l)] border border-transparent bg-[var(--color-status-error-soft)] p-4"
          data-testid="ui-dialog-danger"
        >
          <slot name="danger" />
        </section>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
