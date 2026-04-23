<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { DotLottieVue } from '@lottiefiles/dotlottie-vue'

import { prefersReducedMotion } from '../lib/motion'

const props = withDefaults(defineProps<{
  src: string
  autoplay?: boolean
  loop?: boolean
  lazy?: boolean
  respectReducedMotion?: boolean
  reducedMotion?: boolean
}>(), {
  autoplay: undefined,
  loop: undefined,
  lazy: true,
  respectReducedMotion: true,
  reducedMotion: undefined,
})

const host = ref<HTMLElement | null>(null)
const lazyReady = ref(!props.lazy)
const reducedMotionActive = computed(() =>
  props.respectReducedMotion !== false && (props.reducedMotion ?? prefersReducedMotion()),
)
const reducedMotionState = computed(() => (reducedMotionActive.value ? 'true' : 'false'))
let observer: IntersectionObserver | null = null

function markLazyReady() {
  if (lazyReady.value) {
    return
  }

  lazyReady.value = true
  observer?.disconnect()
  observer = null
}

function resolveAutoplay() {
  if (reducedMotionActive.value) {
    return false
  }

  return props.autoplay
}

function resolveLoop() {
  if (reducedMotionActive.value) {
    return false
  }

  return props.loop
}

onMounted(() => {
  if (!props.lazy) {
    lazyReady.value = true
    return
  }

  if (typeof globalThis.IntersectionObserver !== 'function' || !host.value) {
    lazyReady.value = true
    return
  }

  observer = new globalThis.IntersectionObserver((entries) => {
    if (entries.some(entry => entry.isIntersecting || entry.intersectionRatio > 0)) {
      markLazyReady()
    }
  }, {
    rootMargin: '160px',
  })

  observer.observe(host.value)
})

onBeforeUnmount(() => {
  observer?.disconnect()
  observer = null
})
</script>

<template>
  <div
    ref="host"
    data-testid="ui-dotlottie"
    class="h-full w-full"
    :data-reduced-motion="reducedMotionState"
    :data-lazy-ready="lazyReady ? 'true' : 'false'"
    :data-autoplay="resolveAutoplay() === undefined ? undefined : String(resolveAutoplay())"
    :data-loop="resolveLoop() === undefined ? undefined : String(resolveLoop())"
  >
    <DotLottieVue
      v-if="lazyReady"
      :src="props.src"
      :autoplay="resolveAutoplay()"
      :loop="resolveLoop()"
      class="h-full w-full"
    />
  </div>
</template>
