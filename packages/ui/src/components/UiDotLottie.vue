<script setup lang="ts">
import { DotLottieVue } from '@lottiefiles/dotlottie-vue'

defineProps<{
  src: string
  autoplay?: boolean
  loop?: boolean
}>()
</script>

<template>
  <DotLottieVue :src="src" :autoplay="autoplay" :loop="loop" class="h-full w-full" />
</template>
