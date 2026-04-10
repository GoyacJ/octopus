<script setup lang="ts">
// Site configuration
const { t } = useI18n()

useHead({
  titleTemplate: (titleChunk) => {
    return titleChunk ? `${titleChunk} - ${t('site.name')}` : t('site.name')
  },
  meta: [
    { name: 'viewport', content: 'width=device-width, initial-scale=1' },
    { name: 'description', content: t('site.description') }
  ],
  link: [
    { rel: 'icon', type: 'image/png', href: '/favicon.png' }
  ]
})
</script>

<template>
  <div class="flex flex-col min-h-screen">
    <SiteHeader />
    <main class="flex-grow">
      <NuxtPage />
    </main>
    <SiteFooter />
  </div>
</template>

<style>
.page-enter-active,
.page-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}
.page-enter-from,
.page-leave-to {
  opacity: 0;
  transform: translateY(5px);
}
</style>
