<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Rive } from '@rive-app/canvas'

import { prefersReducedMotion } from '../lib/motion'

const props = withDefaults(defineProps<{
  src: string
  stateMachines?: string[]
  autoplay?: boolean
  lazy?: boolean
  respectReducedMotion?: boolean
  reducedMotion?: boolean
}>(), {
  stateMachines: undefined,
  autoplay: undefined,
  lazy: true,
  respectReducedMotion: true,
  reducedMotion: undefined,
})

const canvas = ref<HTMLCanvasElement | null>(null)
const lazyReady = ref(!props.lazy)
const reducedMotionActive = computed(() =>
  props.respectReducedMotion !== false && (props.reducedMotion ?? prefersReducedMotion()),
)
const reducedMotionState = computed(() => (reducedMotionActive.value ? 'true' : 'false'))
let rive: Rive | null = null
let observer: IntersectionObserver | null = null

function resolveAutoplay() {
  if (reducedMotionActive.value) {
    return false
  }

  return props.autoplay ?? true
}

function initializeRive() {
  if (rive || !canvas.value) {
    return
  }

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
}

function markLazyReady() {
  if (lazyReady.value) {
    return
  }

  lazyReady.value = true
  observer?.disconnect()
  observer = null
}

onMounted(() => {
  if (!props.lazy) {
    lazyReady.value = true
    initializeRive()
    return
  }

  if (typeof globalThis.IntersectionObserver !== 'function' || !canvas.value) {
    lazyReady.value = true
    initializeRive()
    return
  }

  observer = new globalThis.IntersectionObserver((entries) => {
    if (entries.some(entry => entry.isIntersecting || entry.intersectionRatio > 0)) {
      markLazyReady()
      initializeRive()
    }
  }, {
    rootMargin: '160px',
  })

  observer.observe(canvas.value)
})

onBeforeUnmount(() => {
  observer?.disconnect()
  observer = null
  rive?.cleanup()
  rive = null
})
</script>

<template>
  <canvas
    ref="canvas"
    data-testid="ui-rive-canvas"
    class="h-full w-full"
    :data-reduced-motion="reducedMotionState"
    :data-lazy-ready="lazyReady ? 'true' : 'false'"
    :data-autoplay="String(resolveAutoplay())"
  />
</template>
