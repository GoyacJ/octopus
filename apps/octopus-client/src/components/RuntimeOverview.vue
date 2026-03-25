<script setup lang="ts">
import { computed } from 'vue'

import { contractCatalog, interactionSurfaces } from '@octopus/contracts'

const highlightedObjects = computed(() =>
  contractCatalog.coreObjects.filter((entry) =>
    ['Run', 'ApprovalRequest', 'Artifact', 'KnowledgeAsset', 'CapabilityGrant'].includes(entry.name),
  ),
)

const highlightedEvents = computed(() => contractCatalog.events.slice(0, 4))
</script>

<template>
  <section class="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
    <article class="rounded-[32px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-7 shadow-sm">
      <p class="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">Unified Agent Runtime Platform</p>
      <h2 class="mt-3 text-3xl font-semibold leading-tight">
        A buildable skeleton for contracts, runtime boundaries, and the first control-plane shell.
      </h2>
      <p class="mt-4 max-w-2xl text-sm leading-6 text-[var(--text-muted)]">
        The current batch freezes shared contracts, exposes a minimal HTTP runtime slice, and anchors the
        desktop/web control plane around the same vocabulary.
      </p>

      <div class="mt-6 grid gap-3 sm:grid-cols-2">
        <div
          v-for="surface in interactionSurfaces"
          :key="surface"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
        >
          <p class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">Surface</p>
          <p class="mt-2 text-lg font-medium">{{ surface }}</p>
        </div>
      </div>
    </article>

    <aside class="space-y-6">
      <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold">Core Objects</h3>
          <span class="text-sm text-[var(--text-muted)]">{{ contractCatalog.coreObjects.length }}</span>
        </div>
        <ul class="mt-4 space-y-3 text-sm">
          <li
            v-for="entry in highlightedObjects"
            :key="entry.name"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
          >
            <p class="font-medium">{{ entry.name }}</p>
            <p class="mt-1 text-[var(--text-muted)]">{{ entry.bounded_context }}</p>
          </li>
        </ul>
      </section>

      <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold">Event Skeletons</h3>
          <span class="text-sm text-[var(--text-muted)]">{{ contractCatalog.events.length }}</span>
        </div>
        <ul class="mt-4 space-y-3 text-sm">
          <li
            v-for="event in highlightedEvents"
            :key="event.name"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
          >
            <p class="font-medium">{{ event.name }}</p>
            <p class="mt-1 text-[var(--text-muted)]">{{ event.required_fields.join(', ') }}</p>
          </li>
        </ul>
      </section>
    </aside>
  </section>
</template>

