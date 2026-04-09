<script setup lang="ts">
const localeHead = useLocaleHead()

useHead(() => ({
  htmlAttrs: localeHead.value.htmlAttrs,
  link: localeHead.value.link,
  meta: localeHead.value.meta,
}))
</script>

<template>
  <div class="brand-shell">
    <BrandSiteHeader />
    <main>
      <NuxtPage />
    </main>
    <BrandSiteFooter />
  </div>
</template>
