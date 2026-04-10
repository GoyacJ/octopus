<script setup lang="ts">
import { UiPopover } from '@octopus/ui'
import {
  Apple,
  Monitor as Windows,
  Terminal as Linux,
  ChevronRight,
  Download,
  ArrowDown,
} from 'lucide-vue-next'

import {
  buildDownloadPageModel,
  detectCurrentPlatform,
  type DownloadPlatform,
  type NormalizedAsset,
  type NormalizedRelease,
} from '../utils/github-releases'

const { t, locale } = useI18n()
const runtimeConfig = useRuntimeConfig()
const currentPlatform = ref<DownloadPlatform>('macos')
const heroMenuOpen = ref(false)

const { releases, pending, error, releasesUrl } = useGithubReleases()

interface HeroMenuItem {
  key: string
  label: string
}

const platformIcons = {
  macos: Apple,
  windows: Windows,
  linux: Linux,
} satisfies Record<DownloadPlatform, typeof Apple>

useHead({
  title: t('pages.download.title'),
})

onMounted(() => {
  currentPlatform.value = detectCurrentPlatform(window.navigator.userAgent)
})

const downloadModel = computed(() => buildDownloadPageModel(releases.value, currentPlatform.value))
const stableReleaseLabel = computed(() => (
  downloadModel.value.latestStable
    ? t('pages.download.version', { version: downloadModel.value.latestStable.tagName })
    : t('pages.download.loading')
))
const previewReleaseLabel = computed(() => (
  downloadModel.value.latestPreview
    ? `${t('pages.download.latestPreview')} ${downloadModel.value.latestPreview.tagName}`
    : ''
))
const heroPlatformLabel = computed(() => t(`pages.download.platforms.${downloadModel.value.heroPlatform}`))
const heroMenuItems = computed<HeroMenuItem[]>(() => downloadModel.value.heroAssets.map((asset) => ({
  key: createAssetKey(asset),
  label: `${asset.variantLabel} ${asset.fileExtension}`,
})))
const heroAssetsByKey = computed(() => new Map(downloadModel.value.heroAssets.map((asset) => [createAssetKey(asset), asset])))
const supportUrl = computed(() => runtimeConfig.public.demoUrl as string)
const hasDownloads = computed(() => downloadModel.value.platformCards.length > 0)

function createAssetKey(asset: NormalizedAsset) {
  return `${asset.releaseTag}:${asset.id}`
}

function getLocalizedVariantLabel(asset: NormalizedAsset) {
  return t(`pages.download.architectures.${asset.variantKey}`)
}

function getChannelLabel(channel: NormalizedAsset['channel'] | NormalizedRelease['channel']) {
  return t(`pages.download.channel.${channel}`)
}

function getFormattedDate(publishedAt: string) {
  return new Intl.DateTimeFormat(locale.value === 'zh-CN' ? 'zh-CN' : 'en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  }).format(new Date(publishedAt))
}

function getReleaseAssetSummaries(release: NormalizedRelease) {
  return Array.from(new Set(
    release.assets.map((asset) => `${t(`pages.download.platforms.${asset.platform}`)} · ${getLocalizedVariantLabel(asset)} · ${asset.fileExtension}`),
  ))
}

function getHeroAsset(key: string) {
  return heroAssetsByKey.value.get(key) ?? null
}

function handleHeroDownloadSelect(key: string) {
  const asset = heroAssetsByKey.value.get(key)
  if (!asset || !import.meta.client) {
    return
  }

  heroMenuOpen.value = false
  window.location.assign(asset.downloadUrl)
}

function scrollToAvailablePlatforms() {
  const element = document.getElementById('platform-selection')
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <UiSectionHero
      :badge="stableReleaseLabel"
      :title="t('pages.download.title')"
      :subtitle="t('pages.download.subtitle')"
      :show-floating="false"
      align="center"
    >
      <template #actions>
        <div class="flex w-full flex-col items-center gap-6">
          <div class="flex w-full flex-wrap items-center justify-center gap-3">
            <UiBadge variant="outline" class="border-[var(--website-border-strong)] px-4 py-1.5 normal-case tracking-normal">
              {{ t('pages.download.latestStable') }}: {{ downloadModel.latestStable?.tagName ?? '--' }}
            </UiBadge>
            <UiBadge
              v-if="previewReleaseLabel"
              variant="secondary"
              class="px-4 py-1.5 normal-case tracking-normal"
            >
              {{ previewReleaseLabel }}
            </UiBadge>
          </div>

          <div class="flex w-full flex-wrap items-center justify-center gap-4">
            <UiPopover
              :open="heroMenuOpen"
              align="center"
              class="min-w-[18rem] rounded-[28px] border-[var(--website-border-strong)] bg-white p-2 shadow-[0_28px_64px_rgba(249,115,22,0.14)] transition-none"
              @update:open="heroMenuOpen = $event"
            >
              <template #trigger>
                <button
                  type="button"
                  class="inline-flex h-16 items-center justify-center gap-3 rounded-[var(--radius-m)] bg-[var(--website-accent)] px-12 text-lg font-black text-white shadow-2xl shadow-[var(--website-accent)]/20 transition-all hover:bg-[var(--website-accent-hover)] disabled:pointer-events-none disabled:opacity-50"
                  :disabled="pending || !heroMenuItems.length"
                >
                  <Download class="h-6 w-6" />
                  {{ t('pages.download.cta') }} {{ heroPlatformLabel }}
                  <ArrowDown class="h-5 w-5 opacity-60" />
                </button>
              </template>

              <div class="flex min-w-[18rem] flex-col gap-1">
                <button
                  v-for="item in heroMenuItems"
                  :key="item.key"
                  type="button"
                  class="flex w-full items-start justify-between gap-4 rounded-[18px] px-5 py-4 text-left transition-colors hover:bg-[var(--website-surface-soft)]"
                  @click="handleHeroDownloadSelect(item.key)"
                >
                  <div class="flex flex-col gap-1">
                    <span class="text-sm font-semibold text-[var(--website-text)]">
                      {{ getLocalizedVariantLabel(getHeroAsset(item.key)!) }} {{ getHeroAsset(item.key)?.fileExtension }}
                    </span>
                    <span class="text-xs text-[var(--website-text-muted)]">
                      {{ getChannelLabel(getHeroAsset(item.key)!.channel) }} · {{ getHeroAsset(item.key)?.releaseTag }}
                    </span>
                  </div>
                  <span class="text-xs font-semibold text-[var(--website-accent)]">
                    {{ t('pages.download.directDownload') }}
                  </span>
                </button>
              </div>
            </UiPopover>

            <UiButton
              type="button"
              variant="outline"
              size="lg"
              class="px-8 h-16 text-lg glass font-bold border-[var(--website-border-strong)]"
              @click="scrollToAvailablePlatforms"
            >
              {{ t('pages.download.viewVersions') }}
              <ArrowDown class="ml-2 w-5 h-5 opacity-50" />
            </UiButton>
          </div>
        </div>
      </template>

      <template #visual>
        <div class="max-w-md mx-auto relative py-10" v-reveal>
          <div class="absolute inset-0 bg-[var(--website-accent)] opacity-10 blur-[100px] rounded-full"></div>
          <div class="relative w-40 h-44 mx-auto bg-gradient-to-b from-[var(--website-surface)] to-[var(--website-surface-soft)] rounded-[3rem] shadow-2xl border border-[var(--website-border-strong)] flex items-center justify-center transform rotate-3 hover:rotate-0 transition-transform duration-500">
            <img src="/logo.png" alt="Octopus" class="w-24 h-24 object-contain" />
          </div>
        </div>
      </template>
    </UiSectionHero>

    <section id="platform-selection" class="section-padding relative scroll-mt-28 md:scroll-mt-32">
      <div class="container-custom relative z-10">
        <div class="text-center mb-20" v-reveal>
          <h2 class="text-4xl font-bold tracking-tight mb-4">{{ t('pages.download.availableTitle') }}</h2>
          <p class="text-[var(--website-text-muted)] font-medium">{{ t('pages.download.availableBody') }}</p>
        </div>

        <div v-if="pending" class="max-w-3xl mx-auto" v-reveal>
          <UiCard variant="outline" class="border-[var(--website-border-strong)] p-10 text-center">
            <p class="text-base font-semibold">{{ t('pages.download.loading') }}</p>
          </UiCard>
        </div>

        <div v-else-if="error" class="max-w-3xl mx-auto" v-reveal>
          <UiCard variant="outline" class="border-[var(--website-border-strong)] p-10 text-center">
            <p class="text-base font-semibold mb-6">{{ t('pages.download.failed') }}</p>
            <UiButton :href="releasesUrl" target="_blank" rel="noreferrer">
              {{ t('pages.download.allReleases') }}
            </UiButton>
          </UiCard>
        </div>

        <div v-else-if="!hasDownloads" class="max-w-3xl mx-auto" v-reveal>
          <UiCard variant="outline" class="border-[var(--website-border-strong)] p-10 text-center">
            <p class="text-base font-semibold mb-6">{{ t('pages.download.empty') }}</p>
            <UiButton :href="releasesUrl" target="_blank" rel="noreferrer">
              {{ t('pages.download.allReleases') }}
            </UiButton>
          </UiCard>
        </div>

        <div v-else class="grid grid-cols-1 lg:grid-cols-3 gap-8 max-w-7xl mx-auto">
          <UiCard
            v-for="platformCard in downloadModel.platformCards"
            :key="platformCard.platform"
            variant="default"
            padding="lg"
            hover
            v-reveal
            class="group border-[var(--website-border-strong)]"
          >
            <div class="w-16 h-16 rounded-3xl bg-[var(--website-surface-soft)] text-[var(--website-text-muted)] flex items-center justify-center mb-6 transition-all duration-500 group-hover:bg-[var(--website-accent)] group-hover:text-white shadow-lg">
              <component :is="platformIcons[platformCard.platform]" class="w-8 h-8" />
            </div>

            <h3 class="text-2xl font-black mb-2">{{ t(`pages.download.platforms.${platformCard.platform}`) }}</h3>
            <p class="text-sm text-[var(--website-text-muted)] mb-8 font-medium">{{ t(`pages.download.hints.${platformCard.platform}`) }}</p>

            <div class="space-y-4">
              <div
                v-for="asset in platformCard.assets"
                :key="createAssetKey(asset)"
                class="rounded-[var(--radius-m)] border border-[var(--website-border)] bg-[var(--website-surface-soft)]/40 p-4"
              >
                <div class="flex items-start justify-between gap-4">
                  <div class="min-w-0">
                    <div class="flex flex-wrap items-center gap-2 mb-3">
                      <UiBadge size="sm" variant="outline" class="normal-case tracking-normal">
                        {{ getLocalizedVariantLabel(asset) }}
                      </UiBadge>
                      <UiBadge
                        size="sm"
                        :variant="asset.channel === 'stable' ? 'secondary' : 'glass'"
                        class="normal-case tracking-normal"
                      >
                        {{ getChannelLabel(asset.channel) }}
                      </UiBadge>
                    </div>
                    <p class="text-base font-bold break-all">{{ asset.releaseTag }}</p>
                    <p class="text-sm text-[var(--website-text-muted)] mt-1">{{ asset.fileExtension }}</p>
                  </div>

                  <UiButton
                    variant="outline"
                    size="sm"
                    :href="asset.downloadUrl"
                    target="_blank"
                    rel="noreferrer"
                    class="shrink-0"
                  >
                    <Download class="w-4 h-4 mr-2" />
                    {{ t('pages.download.directDownload') }}
                  </UiButton>
                </div>
              </div>
            </div>
          </UiCard>
        </div>

        <div class="mt-24 text-center text-[var(--website-text-muted)]" v-reveal>
          <p class="text-sm mb-6 font-medium">{{ t('pages.download.requirements') }}</p>
          <div class="flex flex-wrap items-center justify-center gap-8">
            <a :href="supportUrl" class="text-sm font-bold hover:text-[var(--website-accent)] flex items-center transition-colors">
              {{ t('pages.download.support') }}
              <ChevronRight class="w-4 h-4 ml-1" />
            </a>
          </div>
        </div>
      </div>
    </section>

    <section id="release-history" class="pb-24 md:pb-32">
      <div class="container-custom">
        <div class="text-center mb-12" v-reveal>
          <h2 class="text-3xl md:text-4xl font-bold tracking-tight mb-4">{{ t('pages.download.historyTitle') }}</h2>
          <p class="text-[var(--website-text-muted)] font-medium">{{ t('pages.download.historyBody') }}</p>
        </div>

        <div class="max-w-5xl mx-auto space-y-4">
          <UiCard
            v-for="release in downloadModel.history"
            :key="release.id"
            variant="default"
            padding="lg"
            class="border-[var(--website-border-strong)]"
            v-reveal
          >
            <div class="flex flex-col gap-6">
              <div class="min-w-0">
                <div class="flex flex-wrap items-center gap-3 mb-3">
                  <h3 class="text-xl font-black break-all">{{ release.name }}</h3>
                  <UiBadge
                    size="sm"
                    :variant="release.channel === 'stable' ? 'secondary' : 'glass'"
                    class="normal-case tracking-normal"
                  >
                    {{ getChannelLabel(release.channel) }}
                  </UiBadge>
                </div>
                <p class="text-sm text-[var(--website-text-muted)] mb-4">{{ release.tagName }}</p>
                <p class="text-sm text-[var(--website-text-muted)] mb-4">{{ t('pages.download.releasedOn', { date: getFormattedDate(release.publishedAt) }) }}</p>
                <div class="flex flex-wrap gap-2">
                  <UiBadge
                    v-for="summary in getReleaseAssetSummaries(release)"
                    :key="`${release.id}-${summary}`"
                    size="sm"
                    variant="outline"
                    class="normal-case tracking-normal"
                  >
                    {{ summary }}
                  </UiBadge>
                </div>
                <div class="mt-6 rounded-[var(--radius-m)] border border-[var(--website-border)] bg-[var(--website-surface-soft)]/40 p-4">
                  <p class="mb-3 text-sm font-bold text-[var(--website-text)]">{{ t('pages.download.notesTitle') }}</p>
                  <div v-if="release.noteSections.length" class="space-y-4">
                    <div
                      v-for="section in release.noteSections"
                      :key="`${release.id}-${section.title || 'notes'}`"
                      class="space-y-2"
                    >
                      <p
                        v-if="section.title"
                        class="text-sm font-semibold text-[var(--website-text)]"
                      >
                        {{ section.title }}
                      </p>
                      <ul class="space-y-2 text-sm leading-6 text-[var(--website-text-muted)]">
                        <li
                          v-for="item in section.items"
                          :key="`${release.id}-${section.title}-${item}`"
                          class="flex gap-2"
                        >
                          <span class="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--website-accent)]"></span>
                          <span>{{ item }}</span>
                        </li>
                      </ul>
                    </div>
                  </div>
                  <p v-else class="text-sm text-[var(--website-text-muted)]">{{ t('pages.download.notesEmpty') }}</p>
                </div>
              </div>
            </div>
          </UiCard>
        </div>
      </div>
    </section>

    <section class="section-padding bg-[var(--website-surface-soft)]/50 border-y border-[var(--website-border)]">
      <div class="container-custom">
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-20 items-center">
          <div class="relative group" v-reveal>
            <div class="absolute -inset-2 bg-gradient-to-tr from-[var(--website-accent)] to-amber-500 rounded-[2.5rem] blur-2xl opacity-10 group-hover:opacity-20 transition-opacity"></div>
            <UiCard variant="glass" padding="none" class="shadow-2xl border-[var(--website-border-strong)] rounded-[2.5rem] overflow-hidden">
              <img src="/screenshots/dashboard.png" class="w-full h-auto transition-transform duration-700 group-hover:scale-[1.02]" alt="Octopus Experience" />
            </UiCard>
          </div>
          <div class="max-w-md" v-reveal>
            <h2 class="text-4xl font-bold mb-8 tracking-tighter">{{ t('pages.download.experience.title') }}</h2>
            <p class="text-lg text-[var(--website-text-muted)] leading-relaxed mb-10 font-medium">
              {{ t('pages.download.experience.desc') }}
            </p>
            <ul class="space-y-6">
              <li v-for="i in ['fastBoot', 'lowMemory', 'privacy']" :key="i" class="flex items-center gap-4">
                <div class="w-6 h-6 rounded-full bg-[var(--website-accent)]/10 flex items-center justify-center text-[var(--website-accent)] shadow-sm">
                  <ChevronRight class="w-4 h-4" />
                </div>
                <span class="text-base font-bold tracking-tight">{{ t(`pages.download.experience.bullets.${i}`) }}</span>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
