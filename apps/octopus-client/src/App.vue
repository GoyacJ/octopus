<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
</script>

<template>
  <div class="min-h-screen bg-[var(--surface-canvas)] text-[var(--text-primary)]">
    <header class="border-b border-[var(--border-muted)] bg-[var(--surface-panel)]/95 backdrop-blur">
      <div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
        <div>
          <p class="text-xs uppercase tracking-[0.28em] text-[var(--text-muted)]">Octopus</p>
          <h1 class="text-xl font-semibold">Control Plane Skeleton</h1>
        </div>
        <nav class="flex gap-3 text-sm text-[var(--text-muted)]">
          <RouterLink class="rounded-full px-3 py-1 transition hover:bg-[var(--surface-elevated)] hover:text-[var(--text-primary)]" to="/">
            Home
          </RouterLink>
        </nav>
      </div>
    </header>
    <main class="mx-auto max-w-6xl px-6 py-8">
      <RouterView />
    </main>
  </div>
</template>

