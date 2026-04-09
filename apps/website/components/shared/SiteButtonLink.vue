<script setup lang="ts">
import { UiButton } from '@octopus/ui'

interface Props {
  href: string
  label: string
  variant?: 'primary' | 'outline' | 'secondary' | 'ghost'
  size?: 'default' | 'sm' | 'lg'
  class?: string
  newTab?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'default',
  class: '',
  newTab: false,
})
</script>

<template>
  <UiButton
    as="a"
    :href="props.href"
    :variant="props.variant"
    :size="props.size"
    :class="props.class"
    :target="props.newTab ? '_blank' : undefined"
    :rel="props.newTab ? 'noreferrer' : undefined"
  >
    {{ props.label }}
  </UiButton>
</template>
