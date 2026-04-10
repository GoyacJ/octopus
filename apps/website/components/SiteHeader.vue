<script setup lang="ts">
import { Globe, Moon, Sun, Github, Menu, X } from 'lucide-vue-next'

const { t, locale, locales, setLocale } = useI18n()
const colorMode = useColorMode()
const isMenuOpen = ref(false)

const toggleColorMode = () => {
  colorMode.preference = colorMode.value === 'dark' ? 'light' : 'dark'
}

const navLinks = [
  { name: 'nav.product', to: '/product' },
  { name: 'nav.scenarios', to: '/scenarios' },
  { name: 'nav.download', to: '/download' },
  { name: 'nav.about', to: '/about' },
]

const toggleMenu = () => {
  isMenuOpen.value = !isMenuOpen.value
}
</script>

<template>
  <header class="header-glass">
    <div class="container-custom h-20 flex items-center justify-between">
      <!-- Logo -->
      <NuxtLink to="/" class="flex items-center gap-3 group">
        <div class="w-10 h-10 flex items-center justify-center transition-all duration-500 group-hover:rotate-[15deg] group-hover:scale-110">
          <img src="/logo.png" alt="Octopus Logo" class="w-full h-full object-contain" />
        </div>
        <span class="font-bold text-2xl tracking-tighter">{{ t('site.name') }}</span>
      </NuxtLink>

      <!-- Desktop Navigation -->
      <nav class="hidden md:flex items-center gap-8">
        <NuxtLink
          v-for="link in navLinks"
          :key="link.to"
          :to="link.to"
          class="text-sm font-medium text-[var(--website-text-muted)] hover:text-[var(--website-accent)] transition-colors"
          active-class="text-[var(--website-accent)]"
        >
          {{ t(link.name) }}
        </NuxtLink>
      </nav>

      <!-- Actions -->
      <div class="flex items-center gap-3">
        <!-- i18n -->
        <div class="hidden sm:flex items-center gap-1 group relative">
          <button class="p-2 rounded-full hover:bg-[var(--website-surface-soft)] transition-colors text-[var(--website-text-muted)]">
            <Globe class="w-5 h-5" />
          </button>
          <div class="absolute top-full right-0 mt-2 p-2 bg-[var(--website-surface)] border border-[var(--website-border)] rounded-xl shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all">
            <button
              v-for="l in locales"
              :key="l.code"
              @click="setLocale(l.code)"
              class="block w-full text-left px-4 py-2 text-sm rounded-lg hover:bg-[var(--website-surface-soft)] whitespace-nowrap"
              :class="{ 'text-[var(--website-accent)] font-semibold': locale === l.code }"
            >
              {{ l.name }}
            </button>
          </div>
        </div>

        <!-- Color Mode -->
        <button
          v-if="colorMode"
          @click="toggleColorMode"
          class="p-2 rounded-full hover:bg-[var(--website-surface-soft)] transition-colors text-[var(--website-text-muted)]"
        >
          <Sun v-if="colorMode.value === 'dark'" class="w-5 h-5" />
          <Moon v-else class="w-5 h-5" />
        </button>

        <!-- CTA -->
        <UiButton to="/book-demo" variant="primary" size="sm" class="hidden sm:inline-flex whitespace-nowrap">
          {{ t('nav.bookDemo') }}
        </UiButton>

        <!-- Mobile Menu Toggle -->
        <button
          @click="toggleMenu"
          class="md:hidden p-2 rounded-full hover:bg-[var(--website-surface-soft)] transition-colors text-[var(--website-text-muted)]"
        >
          <component :is="isMenuOpen ? X : Menu" class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Mobile Menu -->
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="translate-y-[-10px] opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="translate-y-0 opacity-100"
      leave-to-class="translate-y-[-10px] opacity-0"
    >
      <div v-if="isMenuOpen" class="md:hidden bg-[var(--website-surface)] border-b border-[var(--website-border)] px-4 py-6">
        <nav class="flex flex-col gap-4">
          <NuxtLink
            v-for="link in navLinks"
            :key="link.to"
            :to="link.to"
            @click="isMenuOpen = false"
            class="text-lg font-medium px-4 py-2 rounded-xl hover:bg-[var(--website-surface-soft)]"
          >
            {{ t(link.name) }}
          </NuxtLink>
          <UiButton to="/book-demo" variant="primary" class="mt-4 whitespace-nowrap" @click="isMenuOpen = false">
            {{ t('nav.bookDemo') }}
          </UiButton>
        </nav>
      </div>
    </Transition>
  </header>
</template>
