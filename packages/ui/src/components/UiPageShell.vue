<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  width?: 'standard' | 'wide' | 'full'
  padded?: boolean
  class?: string
  contentClass?: string
  testId?: string
}>(), {
  width: 'standard',
  padded: true,
  class: '',
  contentClass: '',
  testId: '',
})

const widthClass = computed(() => {
  if (props.width === 'full') return 'max-w-none'
  if (props.width === 'wide') return 'max-w-[1240px]'
  return 'max-w-[1120px]'
})
</script>

<template>
  <section
    :data-testid="props.testId || undefined"
    :class="cn('w-full', props.padded && 'px-4 py-5 lg:px-6 lg:py-6', props.class)"
  >
    <div :class="cn('mx-auto flex min-w-0 flex-col gap-5', widthClass, props.contentClass)">
      <slot />
    </div>
  </section>
</template>
