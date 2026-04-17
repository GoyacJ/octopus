<script setup lang="ts">
import { DotLottieVue } from '@lottiefiles/dotlottie-vue'

import { prefersReducedMotion } from '../lib/motion'

const props = defineProps<{
  src: string
  autoplay?: boolean
  loop?: boolean
  respectReducedMotion?: boolean
  reducedMotion?: boolean
}>()

function resolveAutoplay() {
  const reducedMotion = props.reducedMotion ?? prefersReducedMotion()

  if (props.respectReducedMotion !== false && reducedMotion) {
    return false
  }

  return props.autoplay
}
</script>

<template>
  <div
    data-testid="ui-dotlottie"
    class="h-full w-full"
    :data-autoplay="resolveAutoplay() === undefined ? undefined : String(resolveAutoplay())"
  >
    <DotLottieVue :src="props.src" :autoplay="resolveAutoplay()" :loop="props.loop" class="h-full w-full" />
  </div>
</template>
