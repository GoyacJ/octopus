<script setup lang="ts">
const { t } = useI18n()

useHead(() => ({
  title: t('pages.home.title'),
  meta: [{ name: 'description', content: t('pages.home.body') }],
}))
</script>

<template>
  <section class="website-placeholder">
    <div class="website-placeholder__inner">
      <div class="website-placeholder__card">
        <p class="website-placeholder__eyebrow">{{ t('site.name') }}</p>
        <h1 class="website-placeholder__title">{{ t('pages.home.title') }}</h1>
        <p class="website-placeholder__body">{{ t('pages.home.body') }}</p>
        <div class="website-placeholder__meta">
          <span class="website-placeholder__pill">{{ t('site.status') }}</span>
          <span class="website-placeholder__pill">/</span>
          <span class="website-placeholder__pill">Nuxt 3</span>
        </div>
      </div>
    </div>
  </section>
</template>
