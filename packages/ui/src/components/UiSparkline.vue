<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  data: number[]
  width?: number
  height?: number
  strokeWidth?: number
  strokeColor?: string
  fillColor?: string
}>(), {
  width: 100,
  height: 30,
  strokeWidth: 2,
  strokeColor: 'currentColor',
  fillColor: 'transparent'
})

const points = computed(() => {
  if (!props.data || props.data.length === 0) return ''
  const min = Math.min(...props.data)
  const max = Math.max(...props.data)
  const range = max - min === 0 ? 1 : max - min
  
  const padding = props.strokeWidth
  const usableWidth = props.width - padding * 2
  const usableHeight = props.height - padding * 2
  
  const stepX = props.data.length > 1 ? usableWidth / (props.data.length - 1) : 0
  
  return props.data.map((val, i) => {
    const x = padding + i * stepX
    const y = padding + usableHeight - ((val - min) / range) * usableHeight
    return `${x},${y}`
  }).join(' ')
})
</script>

<template>
  <svg
    :width="width"
    :height="height"
    :viewBox="`0 0 ${width} ${height}`"
    class="overflow-visible"
    aria-hidden="true"
  >
    <polyline
      :points="points"
      :fill="fillColor"
      :stroke="strokeColor"
      :stroke-width="strokeWidth"
      stroke-linecap="round"
      stroke-linejoin="round"
      vector-effect="non-scaling-stroke"
    />
  </svg>
</template>
