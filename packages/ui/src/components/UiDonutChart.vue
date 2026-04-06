<script setup lang="ts">
import { computed } from 'vue'

export interface DonutItem {
  id: string
  label: string
  value: number
  color?: string
}

const props = withDefaults(defineProps<{
  items: DonutItem[]
  size?: number
  strokeWidth?: number
  totalLabel?: string
}>(), {
  size: 120,
  strokeWidth: 12,
  totalLabel: 'Total'
})

const total = computed(() => props.items.reduce((acc, item) => acc + item.value, 0))

const segments = computed(() => {
  let currentAngle = 0
  const radius = (props.size - props.strokeWidth) / 2
  const circumference = 2 * Math.PI * radius

  return props.items.map((item, index) => {
    const percentage = item.value / total.value
    const arcLength = percentage * circumference
    const offset = currentAngle
    currentAngle += arcLength

    return {
      ...item,
      dashArray: `${arcLength} ${circumference}`,
      dashOffset: -offset,
      color: item.color || `hsl(${index * 60}, 70%, 50%)`
    }
  })
})
</script>

<template>
  <div class="relative flex items-center justify-center" :style="{ width: `${size}px`, height: `${size}px` }">
    <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`" class="rotate-[-90deg]">
      <circle
        v-for="segment in segments"
        :key="segment.id"
        :cx="size / 2"
        :cy="size / 2"
        :r="(size - strokeWidth) / 2"
        fill="none"
        :stroke="segment.color"
        :stroke-width="strokeWidth"
        :stroke-dasharray="segment.dashArray"
        :stroke-dashoffset="segment.dashOffset"
        stroke-linecap="round"
        class="transition-all duration-700 ease-apple"
      />
    </svg>
    <div class="absolute inset-0 flex flex-col items-center justify-center text-center">
      <span class="text-xs font-bold text-text-primary leading-none">{{ total }}</span>
      <span class="text-[10px] text-text-tertiary uppercase tracking-tighter mt-1">{{ props.totalLabel }}</span>
    </div>
  </div>
</template>
