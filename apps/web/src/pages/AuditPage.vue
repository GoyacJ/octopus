<script setup lang="ts">
import { computed } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'
import { useControlPlaneClient } from '@/lib/control-plane'

const { t } = useI18n()
const client = useControlPlaneClient()

const auditQuery = useQuery({
  queryKey: ['audit'],
  queryFn: () => client.listAuditEvents(),
})

const auditEvents = computed(() => auditQuery.data.value ?? [])
</script>

<template>
  <div class="grid gap-4">
    <UiSurfaceCard
      :eyebrow="t('pages.auditEyebrow')"
      :title="t('pages.auditTitle')"
      :body="t('pages.auditBody')"
    >
      <UiStatusBadge
        :label="t('status.ready')"
        tone="accent"
      />
    </UiSurfaceCard>

    <section class="rounded-[24px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-panel)] p-5">
      <div class="space-y-2">
        <p class="text-[11px] uppercase tracking-[0.22em] text-[color:var(--oc-text-muted)]">
          {{ t('pages.auditFeedEyebrow') }}
        </p>
        <h2 class="font-[var(--oc-font-display)] text-2xl tracking-tight">
          {{ t('pages.auditFeedTitle') }}
        </h2>
        <p class="text-sm leading-7 text-[color:var(--oc-text-secondary)]">
          {{ t('pages.auditFeedBody') }}
        </p>
      </div>

      <div class="mt-5 grid gap-3">
        <p
          v-if="auditQuery.isLoading.value"
          class="text-sm text-[color:var(--oc-text-muted)]"
        >
          {{ t('common.loading') }}
        </p>
        <p
          v-else-if="auditEvents.length === 0"
          class="text-sm text-[color:var(--oc-text-muted)]"
        >
          {{ t('pages.auditEmpty') }}
        </p>
        <article
          v-for="event in auditEvents"
          :key="event.id"
          class="rounded-[20px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] p-4"
        >
          <div class="flex flex-wrap items-center justify-between gap-3">
            <p class="font-medium text-[color:var(--oc-text-primary)]">
              {{ event.action }}
            </p>
            <UiStatusBadge
              :label="event.actorId"
              tone="accent"
            />
          </div>
          <p class="mt-3 text-sm leading-6 text-[color:var(--oc-text-secondary)]">
            {{ event.summary }}
          </p>
          <div class="mt-4 flex flex-wrap gap-4 text-xs uppercase tracking-[0.18em] text-[color:var(--oc-text-muted)]">
            <span>{{ t('common.subject') }}: {{ event.subjectType }}/{{ event.subjectId }}</span>
            <span>{{ t('common.occurredAt') }}: {{ event.occurredAt }}</span>
          </div>
        </article>
      </div>
    </section>
  </div>
</template>
