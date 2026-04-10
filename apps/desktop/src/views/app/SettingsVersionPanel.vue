<script setup lang="ts">
import { computed } from 'vue'

import { UiBadge, UiButton, UiEmptyState, UiField, UiMetricCard, UiRecordCard, UiSelect, UiStatusCallout } from '@octopus/ui'

const props = defineProps<{
  appUpdate: any
  versionStatus: any
  latestRelease: any
  updateChannel: string
  updateChannelOptions: Array<{ value: string, label: string }>
  updateStatusTone: 'info' | 'success' | 'warning' | 'error'
  updateStatusLabel: string
  updateStatusDescription: string
  primaryUpdateActionLabel: string
  primaryUpdateActionDisabled: boolean
  hasReleaseNotesLink: boolean
  formatRelativeTimestamp: (value: number | null) => string
  formatReleaseDate: (value?: string | null) => string
}>()

const emit = defineEmits<{
  'update:update-channel': [value: string]
  primary: []
  'check-updates': []
  'open-release-notes': []
}>()

const updateChannelModel = computed({
  get: () => props.updateChannel,
  set: value => emit('update:update-channel', value),
})
</script>

<template>
  <section data-testid="settings-version-center" class="space-y-8">
    <UiRecordCard
      test-id="settings-version-summary-card"
      :title="$t('settings.version.summary.title')"
      :description="$t('settings.version.summary.description')"
      class="overflow-hidden border-border-strong bg-[linear-gradient(135deg,rgba(14,165,233,0.08),rgba(16,185,129,0.06),rgba(255,255,255,0))]"
    >
      <template #eyebrow>
        {{ $t('settings.tabs.version') }}
      </template>
      <template #badges>
        <UiBadge :label="$t(`settings.version.channels.${versionStatus.currentChannel}`)" tone="info" />
        <UiBadge :label="updateStatusLabel" :tone="updateStatusTone" />
      </template>

      <div class="space-y-4">
        <p class="max-w-3xl text-[15px] leading-7 text-text-secondary">
          {{ updateStatusDescription }}
        </p>

        <div class="grid gap-3 md:grid-cols-3">
          <UiMetricCard
            :label="$t('settings.version.metrics.currentVersion')"
            :value="versionStatus.currentVersion"
            :helper="$t('settings.version.metrics.currentVersionHelper')"
          />
          <UiMetricCard
            :label="$t('settings.version.metrics.lastCheckedAt')"
            :value="formatRelativeTimestamp(versionStatus.lastCheckedAt)"
            :helper="$t('settings.version.metrics.lastCheckedHelper')"
          />
          <UiMetricCard
            :label="$t('settings.version.metrics.latestVersion')"
            :value="latestRelease?.version ?? $t('settings.version.values.noRelease')"
            :helper="latestRelease ? formatReleaseDate(latestRelease.publishedAt) : $t('settings.version.metrics.latestVersionHelper')"
            :progress="versionStatus.progress?.percent ?? null"
            :progress-label="typeof versionStatus.progress?.percent === 'number' ? `${versionStatus.progress.percent}%` : ''"
            :tone="versionStatus.state === 'update_available' ? 'warning' : versionStatus.state === 'downloaded' ? 'success' : 'default'"
          />
        </div>
      </div>

      <template #actions>
        <UiButton
          :disabled="primaryUpdateActionDisabled"
          @click="emit('primary')"
        >
          {{ primaryUpdateActionLabel }}
        </UiButton>
        <UiButton
          variant="ghost"
          :disabled="!versionStatus.capabilities.canCheck || versionStatus.state === 'checking'"
          @click="emit('check-updates')"
        >
          {{ $t('settings.version.actions.check') }}
        </UiButton>
        <UiButton
          v-if="hasReleaseNotesLink"
          variant="ghost"
          @click="emit('open-release-notes')"
        >
          {{ $t('settings.version.actions.viewReleaseNotes') }}
        </UiButton>
      </template>
    </UiRecordCard>

    <div class="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_minmax(20rem,0.8fr)]">
      <UiRecordCard
        test-id="settings-version-release-card"
        :title="$t('settings.version.release.title')"
        :description="$t('settings.version.release.description')"
      >
        <template #badges>
          <UiBadge
            v-if="latestRelease"
            :label="$t(`settings.version.channels.${latestRelease.channel}`)"
            tone="info"
          />
          <UiBadge
            :label="updateStatusLabel"
            :tone="updateStatusTone"
          />
        </template>

        <div v-if="latestRelease" class="space-y-4">
          <div class="space-y-1">
            <p class="text-[24px] font-bold tracking-[-0.03em] text-text-primary">
              {{ latestRelease.version }}
            </p>
            <p class="text-[13px] text-text-secondary">
              {{ $t('settings.version.release.publishedAt', { date: formatReleaseDate(latestRelease.publishedAt) }) }}
            </p>
          </div>
          <p class="text-[14px] leading-7 text-text-secondary">
            {{ latestRelease.notes || $t('settings.version.values.noReleaseNotes') }}
          </p>
        </div>
        <UiEmptyState
          v-else
          :title="$t('settings.version.release.emptyTitle')"
          :description="$t('settings.version.release.emptyDescription')"
        />

        <template #actions>
          <UiButton
            v-if="hasReleaseNotesLink"
            variant="ghost"
            @click="emit('open-release-notes')"
          >
            {{ $t('settings.version.actions.viewReleaseNotes') }}
          </UiButton>
        </template>
      </UiRecordCard>

      <div class="space-y-4">
        <UiRecordCard
          :title="$t('settings.version.settings.title')"
          :description="$t('settings.version.settings.description')"
        >
          <div
            data-testid="settings-version-channel-select"
            class="space-y-3"
          >
            <UiField :label="$t('settings.version.settings.channelLabel')">
              <UiSelect
                v-model="updateChannelModel"
                :options="updateChannelOptions"
                :disabled="!versionStatus.capabilities.supportsChannels"
              />
            </UiField>
            <p class="text-[13px] leading-6 text-text-secondary">
              {{ $t('settings.version.settings.channelHint') }}
            </p>
          </div>
        </UiRecordCard>

        <UiRecordCard
          :title="$t('settings.version.details.title')"
          :description="$t('settings.version.details.description')"
        >
          <div class="space-y-3">
            <div class="flex items-center justify-between gap-4 rounded-[var(--radius-m)] bg-subtle px-3 py-3">
              <span class="text-[13px] text-text-secondary">{{ $t('settings.version.details.currentVersion') }}</span>
              <span class="font-mono text-[13px] font-semibold text-text-primary">{{ versionStatus.currentVersion }}</span>
            </div>
            <div class="flex items-center justify-between gap-4 rounded-[var(--radius-m)] bg-subtle px-3 py-3">
              <span class="text-[13px] text-text-secondary">{{ $t('settings.version.details.channel') }}</span>
              <UiBadge :label="$t(`settings.version.channels.${versionStatus.currentChannel}`)" tone="info" />
            </div>
            <div class="flex items-center justify-between gap-4 rounded-[var(--radius-m)] bg-subtle px-3 py-3">
              <span class="text-[13px] text-text-secondary">{{ $t('settings.version.details.status') }}</span>
              <UiBadge :label="updateStatusLabel" :tone="updateStatusTone" />
            </div>
          </div>
        </UiRecordCard>

        <UiStatusCallout
          :tone="updateStatusTone"
          :title="$t('settings.version.callout.title')"
          :description="updateStatusDescription"
        />
      </div>
    </div>
  </section>
</template>
