<script setup lang="ts">
import { useWebsiteTheme } from '../../composables/useWebsiteTheme'

const { t } = useI18n()
const theme = useWebsiteTheme()
</script>

<template>
  <div class="inline-flex items-center gap-1 rounded-full border border-border-subtle bg-surface/90 p-1 shadow-xs">
    <button
      v-for="option in theme.options"
      :key="option"
      type="button"
      class="rounded-full px-3 py-1.5 text-[12px] font-semibold transition-colors duration-fast"
      :class="theme.preference.value === option ? 'bg-accent text-text-primary' : 'text-text-secondary hover:bg-accent hover:text-text-primary'"
      @click="theme.setPreference(option)"
    >
      {{ t(`theme.${option}`) }}
    </button>
  </div>
</template>
