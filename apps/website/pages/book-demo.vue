<script setup lang="ts">
const { t } = useI18n()
const submitted = ref(false)
const form = reactive({
  name: '',
  company: '',
  email: '',
  message: ''
})

useHead({
  title: t('pages.bookDemo.title')
})

const submitForm = () => {
  // Mock submission
  setTimeout(() => {
    submitted.value = true
  }, 600)
}
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <!-- Hero -->
    <UiSectionHero
      align="left"
      :badge="t('pages.bookDemo.badge')"
      :title="t('pages.bookDemo.title')"
      :subtitle="t('pages.bookDemo.body')"
    />

    <!-- Form Section -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="max-w-2xl mx-auto" v-reveal>
          <Transition mode="out-in">
            <div v-if="!submitted" key="form">
              <form @submit.prevent="submitForm" class="space-y-8">
                <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
                  <div class="space-y-3">
                    <label class="text-sm font-bold ml-1 uppercase tracking-widest opacity-60">{{ t('pages.bookDemo.form.nameLabel') }}</label>
                    <input v-model="form.name" type="text" required :placeholder="t('pages.bookDemo.form.namePlaceholder')" class="w-full px-5 py-4 rounded-2xl border border-[var(--website-border-strong)] bg-[var(--website-surface-soft)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--website-accent)] transition-all glass" />
                  </div>
                  <div class="space-y-3">
                    <label class="text-sm font-bold ml-1 uppercase tracking-widest opacity-60">{{ t('pages.bookDemo.form.companyLabel') }}</label>
                    <input v-model="form.company" type="text" required :placeholder="t('pages.bookDemo.form.companyPlaceholder')" class="w-full px-5 py-4 rounded-2xl border border-[var(--website-border-strong)] bg-[var(--website-surface-soft)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--website-accent)] transition-all glass" />
                  </div>
                </div>
                <div class="space-y-3">
                  <label class="text-sm font-bold ml-1 uppercase tracking-widest opacity-60">{{ t('pages.bookDemo.form.emailLabel') }}</label>
                  <input v-model="form.email" type="email" required placeholder="email@company.com" class="w-full px-5 py-4 rounded-2xl border border-[var(--website-border-strong)] bg-[var(--website-surface-soft)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--website-accent)] transition-all glass" />
                </div>
                <div class="space-y-3">
                  <label class="text-sm font-bold ml-1 uppercase tracking-widest opacity-60">{{ t('pages.bookDemo.form.messageLabel') }}</label>
                  <textarea v-model="form.message" rows="5" :placeholder="t('pages.bookDemo.form.messagePlaceholder')" class="w-full px-5 py-4 rounded-2xl border border-[var(--website-border-strong)] bg-[var(--website-surface-soft)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--website-accent)] transition-all glass"></textarea>
                </div>
                <UiButton type="submit" size="lg" class="w-full h-16 text-lg font-bold">
                  {{ t('pages.bookDemo.submit') }}
                </UiButton>
              </form>
            </div>
            <div v-else key="success" class="text-center py-20">
              <div class="w-24 h-24 bg-green-500/10 text-green-500 rounded-3xl flex items-center justify-center mx-auto mb-10 shadow-2xl shadow-green-500/20">
                <svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
              </div>
              <h2 class="text-4xl font-bold mb-6 tracking-tight">{{ t('pages.bookDemo.successTitle') }}</h2>
              <p class="text-xl text-[var(--website-text-muted)] mb-12 font-medium">{{ t('pages.bookDemo.successBody') }}</p>
              <UiButton variant="outline" to="/" size="lg" class="glass px-10">{{ t('pages.bookDemo.backHome') }}</UiButton>
            </div>
          </Transition>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
@keyframes fade-in {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}
.animate-fade-in {
  animation: fade-in 0.5s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}
.v-enter-active, .v-leave-active { transition: opacity 0.3s ease; }
.v-enter-from, .v-leave-to { opacity: 0; }
</style>
