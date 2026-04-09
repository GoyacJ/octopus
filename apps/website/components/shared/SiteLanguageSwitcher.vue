<script setup lang="ts">
const { locale, locales } = useI18n()
const switchLocalePath = useSwitchLocalePath()

const availableLocales = computed(() => locales.value.map((entry) => ({
  code: entry.code,
  name: entry.name ?? entry.code,
  path: switchLocalePath(entry.code),
})))
</script>

<template>
  <div class="inline-flex items-center gap-1 rounded-full border border-border-subtle bg-surface/90 p-1 shadow-xs">
    <NuxtLink
      v-for="entry in availableLocales"
      :key="entry.code"
      :to="entry.path"
      class="rounded-full px-3 py-1.5 text-[12px] font-semibold transition-colors duration-fast"
      :class="locale === entry.code ? 'bg-accent text-text-primary' : 'text-text-secondary hover:bg-accent hover:text-text-primary'"
    >
      {{ entry.name }}
    </NuxtLink>
  </div>
</template>
