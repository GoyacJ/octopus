<script setup lang="ts">
import type { ShellNavigationItem } from '../types'

defineProps<{
  workspaceName: string
  tagline: string
  navigation: ShellNavigationItem[]
  currentPath: string
  laterLabel: string
}>()

const emit = defineEmits<{
  navigate: [item: ShellNavigationItem]
}>()
</script>

<template>
  <div class="min-h-screen px-4 py-4 md:px-6 md:py-6">
    <div
      class="mx-auto grid min-h-[calc(100vh-2rem)] max-w-[1600px] gap-4 rounded-[32px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] p-3 shadow-[var(--oc-shadow-soft)] backdrop-blur md:grid-cols-[280px,minmax(0,1fr)] md:p-4"
    >
      <aside
        class="flex flex-col gap-5 rounded-[28px] bg-[linear-gradient(180deg,rgba(40,100,255,0.08),transparent_45%),var(--oc-bg-panel)] p-5"
      >
        <div class="space-y-2">
          <p class="font-[var(--oc-font-display)] text-3xl leading-none tracking-tight">
            octopus
          </p>
          <p class="text-sm text-[color:var(--oc-text-secondary)]">
            {{ workspaceName }}
          </p>
          <p class="text-sm leading-6 text-[color:var(--oc-text-muted)]">
            {{ tagline }}
          </p>
        </div>

        <nav class="grid gap-2">
          <button
            v-for="item in navigation"
            :key="item.id"
            type="button"
            class="flex items-center justify-between rounded-[20px] border px-3 py-3 text-left transition"
            :class="
              item.enabled
                ? currentPath === item.to
                  ? 'border-[color:var(--oc-accent)] bg-[color:var(--oc-accent-soft)] text-[color:var(--oc-text-primary)]'
                  : 'border-[color:var(--oc-border-subtle)] bg-transparent text-[color:var(--oc-text-secondary)] hover:border-[color:var(--oc-accent)]/50 hover:bg-[color:var(--oc-accent-soft)]'
                : 'cursor-not-allowed border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-muted)]/60 text-[color:var(--oc-text-muted)]'
            "
            @click="item.enabled && emit('navigate', item)"
          >
            <span
              class="flex items-center gap-3"
            >
              <span
                class="text-lg"
                :class="item.iconClass"
              />
              <span class="text-sm font-medium">{{ item.label }}</span>
            </span>
            <span
              v-if="!item.enabled"
              class="rounded-full bg-black/6 px-2 py-1 text-[11px] uppercase tracking-[0.18em] dark:bg-white/8"
            >
              {{ laterLabel }}
            </span>
          </button>
        </nav>
      </aside>

      <div class="flex min-w-0 flex-col gap-4 rounded-[28px] bg-[color:var(--oc-bg-panel)] p-4 md:p-6">
        <header
          class="flex flex-wrap items-center justify-between gap-3 border-b border-[color:var(--oc-border-subtle)] pb-4"
        >
          <div class="space-y-1">
            <p class="text-[11px] uppercase tracking-[0.24em] text-[color:var(--oc-text-muted)]">
              Control Plane
            </p>
            <h1 class="font-[var(--oc-font-display)] text-3xl leading-none tracking-tight">
              Governance-first shell
            </h1>
          </div>

          <div class="flex items-center gap-2">
            <slot name="actions" />
          </div>
        </header>

        <main class="min-w-0 flex-1">
          <slot />
        </main>
      </div>
    </div>
  </div>
</template>
