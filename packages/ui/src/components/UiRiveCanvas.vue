<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { Rive } from '@rive-app/canvas'

import { prefersReducedMotion } from '../lib/motion'

const props = defineProps<{
  src: string
  stateMachines?: string[]
  autoplay?: boolean
  respectReducedMotion?: boolean
  reducedMotion?: boolean
}>()

const canvas = ref<HTMLCanvasElement | null>(null)
let rive: Rive | null = null

function resolveAutoplay() {
  const reducedMotion = props.reducedMotion ?? prefersReducedMotion()

  if (props.respectReducedMotion !== false && reducedMotion) {
    return false
  }

  return props.autoplay ?? true
}

onMounted(() => {
  if (!canvas.value) {
    return
  }

  try {
    if (!canvas.value.getContext?.('2d')) {
      return
    }
  } catch {
    return
  }

  rive = new Rive({
    src: props.src,
    canvas: canvas.value,
    autoplay: resolveAutoplay(),
    stateMachines: props.stateMachines,
  })
})

onBeforeUnmount(() => {
  rive?.cleanup()
  rive = null
})
</script>

<template>
  <canvas
    ref="canvas"
    data-testid="ui-rive-canvas"
    class="h-full w-full"
    :data-autoplay="String(resolveAutoplay())"
  />
</template>
