<script setup lang="ts" generic="TItem">
import { useVirtualizer } from '@tanstack/vue-virtual'
import { ref, watch } from 'vue'

const props = withDefaults(defineProps<{
  items: TItem[]
  estimateSize?: number
  overscan?: number
  maxHeight?: number
}>(), {
  estimateSize: 56,
  overscan: 5,
  maxHeight: 320,
})

const containerRef = ref<HTMLElement | null>(null)

const virtualizer = useVirtualizer({
  count: props.items.length,
  getScrollElement: () => containerRef.value,
  estimateSize: () => props.estimateSize,
  overscan: props.overscan,
})

watch(() => props.items.length, (count) => {
  virtualizer.value.setOptions({
    ...virtualizer.value.options,
    count,
  })
})
</script>

<template>
  <div
    ref="containerRef"
    class="overflow-auto"
    :style="{ maxHeight: `${props.maxHeight}px` }"
  >
    <div
      class="relative w-full"
      :style="{ height: `${virtualizer.getTotalSize()}px` }"
    >
      <div
        v-for="virtualItem in virtualizer.getVirtualItems()"
        :key="String(virtualItem.key)"
        class="absolute left-0 top-0 w-full"
        :style="{ transform: `translateY(${virtualItem.start}px)` }"
      >
        <slot :item="props.items[virtualItem.index]" :index="virtualItem.index" />
      </div>
    </div>
  </div>
</template>
