<script setup lang="ts">
interface Props {
  cols?: number
  rows?: number
  variant?: 'default' | 'soft' | 'outline' | 'glass'
}

withDefaults(defineProps<Props>(), {
  cols: 1,
  rows: 1,
  variant: 'default'
})

const colClasses = {
  1: 'md:col-span-1',
  2: 'md:col-span-2',
  3: 'md:col-span-3',
  4: 'md:col-span-4',
  5: 'md:col-span-5',
  6: 'md:col-span-6',
  7: 'md:col-span-7',
  8: 'md:col-span-8',
  9: 'md:col-span-9',
  10: 'md:col-span-10',
  11: 'md:col-span-11',
  12: 'md:col-span-12',
}

const rowClasses = {
  1: 'md:row-span-1',
  2: 'md:row-span-2',
  3: 'md:row-span-3',
}
</script>

<template>
  <div :class="[colClasses[cols], rowClasses[rows]]">
    <UiCard :variant="variant" padding="none" class="h-full relative group overflow-hidden">
      <slot />
    </UiCard>
  </div>
</template>
