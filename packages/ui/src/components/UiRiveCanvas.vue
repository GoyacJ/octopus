<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { Rive } from '@rive-app/canvas'

const props = defineProps<{
  src: string
  stateMachines?: string[]
  autoplay?: boolean
}>()

const canvas = ref<HTMLCanvasElement | null>(null)
let rive: Rive | null = null

onMounted(() => {
  if (!canvas.value) {
    return
  }

  rive = new Rive({
    src: props.src,
    canvas: canvas.value,
    autoplay: props.autoplay ?? true,
    stateMachines: props.stateMachines,
  })
})

onBeforeUnmount(() => {
  rive?.cleanup()
  rive = null
})
</script>

<template>
  <canvas ref="canvas" class="h-full w-full" />
</template>
