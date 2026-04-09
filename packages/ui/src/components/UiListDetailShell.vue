<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  listClass?: string
  detailClass?: string
}>(), {
  class: '',
  listClass: '',
  detailClass: '',
})
</script>

<template>
  <div :class="cn('grid gap-4 xl:grid-cols-[360px_minmax(0,1fr)]', props.class)">
    <aside :class="cn('min-w-0', props.listClass)">
      <slot name="list" />
    </aside>
    <section :class="cn('min-w-0', props.detailClass)">
      <slot />
    </section>
  </div>
</template>
