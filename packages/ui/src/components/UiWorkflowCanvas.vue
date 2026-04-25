<script setup lang="ts">
import { computed } from 'vue'

interface Node {
  id: string
  label: string
  type?: string
  position: { x: number, y: number }
  data?: {
    role?: string
  }
}

interface Edge {
  id: string
  source: string
  target: string
  animated?: boolean
  label?: string
}

const props = defineProps<{
  nodes: Node[]
  edges: Edge[]
  readonly?: boolean
}>()

const bounds = computed(() => {
  if (props.nodes.length === 0) {
    return { minX: 0, minY: 0, width: 1, height: 1 }
  }

  const xs = props.nodes.map(node => node.position.x)
  const ys = props.nodes.map(node => node.position.y)
  const minX = Math.min(...xs)
  const minY = Math.min(...ys)
  const maxX = Math.max(...xs)
  const maxY = Math.max(...ys)

  return {
    minX,
    minY,
    width: Math.max(maxX - minX, 1),
    height: Math.max(maxY - minY, 1),
  }
})

const positionedNodes = computed(() => props.nodes.map(node => ({
  ...node,
  left: `${((node.position.x - bounds.value.minX) / bounds.value.width) * 72 + 14}%`,
  top: `${((node.position.y - bounds.value.minY) / bounds.value.height) * 58 + 18}%`,
})))

const positionedEdges = computed(() => props.edges.map((edge) => {
  const source = positionedNodes.value.find(node => node.id === edge.source)
  const target = positionedNodes.value.find(node => node.id === edge.target)

  if (!source || !target) {
    return null
  }

  const x1 = Number.parseFloat(source.left)
  const y1 = Number.parseFloat(source.top)
  const x2 = Number.parseFloat(target.left)
  const y2 = Number.parseFloat(target.top)

  return {
    ...edge,
    x1: `${x1}%`,
    y1: `${y1}%`,
    x2: `${x2}%`,
    y2: `${y2}%`,
  }
}).filter(edge => edge !== null))
</script>

<template>
  <div class="relative h-full min-h-[400px] w-full overflow-hidden rounded-[var(--radius-l)] border border-border bg-subtle/50">
    <svg class="absolute inset-0 h-full w-full" aria-hidden="true">
      <defs>
        <pattern id="ui-workflow-grid" width="20" height="20" patternUnits="userSpaceOnUse">
          <path d="M 20 0 L 0 0 0 20" fill="none" stroke="var(--border-subtle)" stroke-width="1" opacity="0.45" />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#ui-workflow-grid)" />
      <line
        v-for="edge in positionedEdges"
        :key="edge.id"
        :x1="edge.x1"
        :y1="edge.y1"
        :x2="edge.x2"
        :y2="edge.y2"
        class="stroke-text-tertiary"
        stroke-width="1.5"
        stroke-linecap="round"
        :stroke-dasharray="edge.animated ? '5 5' : undefined"
      />
    </svg>

    <div
      v-for="node in positionedNodes"
      :key="node.id"
      class="absolute -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 shadow-sm"
      :class="node.type === 'input' ? 'ring-2 ring-primary/20' : ''"
      :style="{ left: node.left, top: node.top }"
    >
      <div class="text-sm font-semibold text-text-primary">{{ node.label }}</div>
      <div v-if="node.data?.role" class="mt-1 text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
        {{ node.data.role }}
      </div>
    </div>

    <div v-if="readonly" class="pointer-events-none absolute left-3 top-3 rounded-[var(--radius-xs)] border border-border bg-surface/80 px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-text-tertiary backdrop-blur-sm">
      Read Only View
    </div>
  </div>
</template>
