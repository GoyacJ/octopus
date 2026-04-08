<script setup lang="ts">
import { Icon } from '@iconify/vue'
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  icon: string | object
  size?: number
  class?: string
}>(), {
  size: 16,
  class: '',
})

const isStringIcon = computed(() => typeof props.icon === 'string')
</script>

<template>
  <Icon
    v-if="isStringIcon"
    :icon="props.icon as string"
    :width="props.size"
    :height="props.size"
    :class="props.class"
  />
  <component
    :is="props.icon"
    v-else
    :size="props.size"
    :class="props.class"
  />
</template>
