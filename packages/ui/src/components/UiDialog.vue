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
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const accessibleTitle = computed(() => props.title || props.closeLabel)
const visibleDescription = computed(() => props.description?.trim() ?? '')
const accessibleDescription = computed(() => visibleDescription.value || accessibleTitle.value)
</script>

<template>
  <DialogRoot :open="props.open" @update:open="emit('update:open', $event)">
    <DialogPortal>
      <DialogOverlay class="fixed inset-0 z-50 bg-black/42 backdrop-blur-md transition-opacity" />
      <DialogContent
        :data-testid="props.contentTestId"
        data-ui-dialog-content="true"
        :class="cn(
          'fixed left-1/2 top-1/2 z-50 flex w-[calc(100%-2rem)] max-w-2xl -translate-x-1/2 -translate-y-1/2 flex-col gap-5 rounded-[calc(var(--radius-xl)+2px)] border border-border bg-popover p-5 shadow-lg md:w-full md:p-6',
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
          class="flex items-start justify-between gap-3 border-b border-border/60 pb-4"
        >
          <div class="min-w-0 flex-1">
            <slot name="header">
              <div class="space-y-1">
                <div class="text-xl font-semibold tracking-[-0.02em] text-text-primary">
                  {{ accessibleTitle }}
                </div>
                <div v-if="visibleDescription" class="text-sm leading-6 text-text-secondary">
                  {{ visibleDescription }}
                </div>
              </div>
            </slot>
          </div>

          <DialogClose as-child>
            <button
              type="button"
              class="inline-flex size-8 shrink-0 items-center justify-center rounded-md text-text-secondary transition-colors hover:bg-subtle hover:text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/20"
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
          :class="cn('min-w-0 text-text-primary', props.bodyClass)"
        >
          <slot />
        </div>

        <footer
          v-if="$slots.footer"
          :class="cn('flex justify-end gap-2 border-t border-border/60 pt-4', props.footerClass)"
        >
          <slot name="footer" />
        </footer>

        <section
          v-if="$slots.danger"
          class="flex flex-col gap-3 rounded-[calc(var(--radius-lg)+2px)] border border-destructive/20 bg-destructive/[0.04] p-4"
          data-testid="ui-dialog-danger"
        >
          <slot name="danger" />
        </section>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
