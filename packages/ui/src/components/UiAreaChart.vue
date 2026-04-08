<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  data: number[]
  labels?: string[]
  width?: number
  height?: number
  strokeColor?: string
  fillColor?: string
}>(), {
  width: 400,
  height: 120,
  strokeColor: 'var(--brand-primary)',
  fillColor: 'rgba(var(--brand-primary-rgb), 0.1)'
})

const points = computed(() => {
  if (!props.data || props.data.length < 2) return ''
  const min = Math.min(...props.data)
  const max = Math.max(...props.data)
  const range = max - min === 0 ? 1 : max - min
  
  const stepX = props.width / (props.data.length - 1)
  
  return props.data.map((val, i) => {
    const x = i * stepX
    const y = props.height - ((val - min) / range) * props.height
    return `${x},${y}`
  }).join(' ')
})

const areaPoints = computed(() => {
  if (!points.value) return ''
  return `${points.value} ${props.width},${props.height} 0,${props.height}`
})
</script>

<template>
  <div class="relative w-full h-full min-h-[120px]">
    <svg
      width="100%"
      height="100%"
      :viewBox="`0 0 ${width} ${height}`"
      preserveAspectRatio="none"
      class="overflow-visible"
    >
      <defs>
        <linearGradient id="areaGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" :stop-color="strokeColor" stop-opacity="0.2" />
          <stop offset="100%" :stop-color="strokeColor" stop-opacity="0" />
        </linearGradient>
      </defs>
      
      <!-- Area Fill -->
      <polyline
        :points="areaPoints"
        fill="url(#areaGradient)"
        stroke="none"
      />
      
      <!-- Line -->
      <polyline
        :points="points"
        fill="none"
        :stroke="strokeColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </div>
</template>
