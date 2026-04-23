<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  width?: 'standard' | 'wide' | 'full'
  density?: 'compact' | 'regular' | 'comfortable'
  padded?: boolean
  class?: string
  contentClass?: string
  testId?: string
}>(), {
  width: 'standard',
  density: 'regular',
  padded: true,
  class: '',
  contentClass: '',
  testId: '',
})

const widthClass = computed(() => {
  if (props.width === 'full') return 'max-w-none'
  if (props.width === 'wide') return 'max-w-[1240px]'
  return 'max-w-[1120px]'
})
</script>

<template>
  <section
    :data-testid="props.testId || undefined"
    :data-density="props.density"
    :class="cn('ui-page-shell w-full', props.padded && 'ui-page-shell--padded', props.class)"
  >
    <div :class="cn('ui-page-shell__content mx-auto flex min-w-0 flex-col', widthClass, props.contentClass)">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.ui-page-shell[data-density="compact"] {
  --page-shell-padding-horizontal: var(--density-compact-horizontal-padding);
  --page-shell-padding-horizontal-lg: var(--density-compact-horizontal-padding-lg);
  --page-shell-padding-vertical: var(--density-compact-vertical-padding);
  --page-shell-padding-vertical-lg: var(--density-compact-vertical-padding-lg);
  --page-shell-layout-gap: var(--density-compact-layout-gap);
}

.ui-page-shell[data-density="regular"] {
  --page-shell-padding-horizontal: var(--density-regular-horizontal-padding);
  --page-shell-padding-horizontal-lg: var(--density-regular-horizontal-padding-lg);
  --page-shell-padding-vertical: var(--density-regular-vertical-padding);
  --page-shell-padding-vertical-lg: var(--density-regular-vertical-padding-lg);
  --page-shell-layout-gap: var(--density-regular-layout-gap);
}

.ui-page-shell[data-density="comfortable"] {
  --page-shell-padding-horizontal: var(--density-comfortable-horizontal-padding);
  --page-shell-padding-horizontal-lg: var(--density-comfortable-horizontal-padding-lg);
  --page-shell-padding-vertical: var(--density-comfortable-vertical-padding);
  --page-shell-padding-vertical-lg: var(--density-comfortable-vertical-padding-lg);
  --page-shell-layout-gap: var(--density-comfortable-layout-gap);
}

.ui-page-shell--padded {
  padding-inline: var(--page-shell-padding-horizontal);
  padding-block: var(--page-shell-padding-vertical);
}

.ui-page-shell__content {
  gap: var(--page-shell-layout-gap);
}

@media (min-width: 1024px) {
  .ui-page-shell--padded {
    padding-inline: var(--page-shell-padding-horizontal-lg, var(--page-shell-padding-horizontal));
    padding-block: var(--page-shell-padding-vertical-lg, var(--page-shell-padding-vertical));
  }
}
</style>
