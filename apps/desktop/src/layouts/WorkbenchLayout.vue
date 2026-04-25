<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'

import WorkbenchRail from '@/components/layout/WorkbenchRail.vue'
import WorkbenchSearchOverlay from '@/components/layout/WorkbenchSearchOverlay.vue'
import WorkbenchSidebar from '@/components/layout/WorkbenchSidebar.vue'
import WorkbenchTopbar from '@/components/layout/WorkbenchTopbar.vue'
import { useShellStore } from '@/stores/shell'

const shell = useShellStore()

function handleGlobalKeydown(event: KeyboardEvent) {
  if (shell.matchesSearchShortcut(event)) {
    event.preventDefault()
    shell.openSearch()
    return
  }

  if (event.key === 'Escape' && shell.searchOpen) {
    shell.closeSearch()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleGlobalKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <div
    data-testid="workbench-shell"
    class="flex h-screen w-screen overflow-hidden bg-sidebar font-sans text-text-primary antialiased"
  >
    <WorkbenchRail />
    <WorkbenchSidebar />

    <div v-if="shell.leftSidebarCollapsed" data-testid="sidebar-rail" class="hidden" />

    <div class="relative flex min-w-0 flex-1 flex-col border-l border-border/70 bg-background shadow-xl">
      <WorkbenchTopbar />

      <main
        class="min-w-0 flex-1 overflow-y-auto bg-[color-mix(in_srgb,var(--background)_94%,var(--sidebar)_6%)]"
        data-testid="workbench-main"
      >
        <div data-testid="workbench-main-canvas" class="flex h-full min-w-0 flex-col">
          <slot />
        </div>
      </main>
    </div>

    <WorkbenchSearchOverlay />
  </div>
</template>

<style scoped>
main {
  scrollbar-gutter: stable;
}

:deep(.scroll-y) {
  overflow-y: auto;
  scrollbar-width: thin;
}
</style>
