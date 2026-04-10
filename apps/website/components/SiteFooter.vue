<script setup lang="ts">
import { Github, Twitter, Linkedin } from 'lucide-vue-next'
const { t } = useI18n()

const currentYear = new Date().getFullYear()

const footerLinks = [
  {
    title: 'footer.product',
    links: [
      { name: 'nav.product', to: '/product' },
      { name: 'nav.scenarios', to: '/scenarios' },
      { name: 'nav.download', to: '/download' },
    ]
  },
  {
    title: 'footer.company',
    links: [
      { name: 'nav.about', to: '/about' },
      { name: 'footer.blog', to: '#' },
    ]
  },
  {
    title: 'footer.support',
    links: [
      { name: 'footer.docs', to: '#' },
      { name: 'footer.contact', to: '/book-demo' },
    ]
  }
]
</script>

<template>
  <footer class="bg-[var(--website-surface-soft)] border-t border-[var(--website-border)] pt-16 pb-8">
    <div class="container-custom">
      <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-8 mb-12">
        <!-- Logo & Bio -->
        <div class="col-span-2">
          <NuxtLink to="/" class="flex items-center gap-2 mb-4 group">
            <div class="w-8 h-8 bg-[var(--website-accent)] rounded-lg flex items-center justify-center transition-transform group-hover:scale-110">
              <span class="text-white font-bold text-lg">O</span>
            </div>
            <span class="font-bold text-xl tracking-tight">{{ t('site.name') }}</span>
          </NuxtLink>
          <p class="text-[var(--website-text-muted)] text-sm max-w-xs leading-relaxed">
            {{ t('site.description') }}
          </p>
          <div class="flex gap-4 mt-6">
            <a href="#" class="text-[var(--website-text-muted)] hover:text-[var(--website-accent)] transition-colors">
              <Github class="w-5 h-5" />
            </a>
            <a href="#" class="text-[var(--website-text-muted)] hover:text-[var(--website-accent)] transition-colors">
              <Twitter class="w-5 h-5" />
            </a>
            <a href="#" class="text-[var(--website-text-muted)] hover:text-[var(--website-accent)] transition-colors">
              <Linkedin class="w-5 h-5" />
            </a>
          </div>
        </div>

        <!-- Links -->
        <div v-for="section in footerLinks" :key="section.title">
          <h4 class="font-semibold text-sm mb-4 uppercase tracking-wider text-[var(--website-text)]">
            {{ t(section.title) }}
          </h4>
          <ul class="space-y-3">
            <li v-for="link in section.links" :key="link.name">
              <NuxtLink :to="link.to" class="text-sm text-[var(--website-text-muted)] hover:text-[var(--website-accent)] transition-colors">
                {{ t(link.name) }}
              </NuxtLink>
            </li>
          </ul>
        </div>
      </div>

      <div class="border-t border-[var(--website-border)] pt-8 flex flex-col md:flex-row justify-between items-center gap-4">
        <p class="text-[var(--website-text-muted)] text-xs">
          © {{ currentYear }} {{ t('site.name') }}. All rights reserved.
        </p>
        <div class="flex gap-6">
          <a href="#" class="text-[var(--website-text-muted)] text-xs hover:text-[var(--website-accent)]">{{ t('footer.privacy') }}</a>
          <a href="#" class="text-[var(--website-text-muted)] text-xs hover:text-[var(--website-accent)]">{{ t('footer.terms') }}</a>
        </div>
      </div>
    </div>
  </footer>
</template>
