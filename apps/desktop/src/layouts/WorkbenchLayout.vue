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
    class="flex h-screen w-screen overflow-hidden bg-[radial-gradient(ellipse_at_top_left,var(--sidebar)_0%,#000_100%)] font-sans text-text-primary antialiased p-3 gap-3"
  >
    <!-- Unified Sidebar (The Command Column) -->
    <WorkbenchSidebar class="z-20 rounded-[var(--radius-2xl)] border border-white/5 shadow-2xl" />

    <!-- Main Content Area: The Floating Canvas -->
    <div 
      class="relative flex min-w-0 flex-1 flex-col bg-background rounded-[var(--radius-2xl)] shadow-[0_20px_50px_rgba(0,0,0,0.5)] border border-white/5 overflow-hidden transition-all duration-slow ease-apple"
    >
      <WorkbenchTopbar />

      <main
        class="min-w-0 flex-1 overflow-hidden"
        data-testid="workbench-main"
      >
        <div 
          data-testid="workbench-main-canvas" 
          class="flex h-full min-w-0 flex-col p-4 lg:p-5"
        >
          <div class="flex-1 flex flex-col rounded-[var(--radius-xl)] bg-surface/50 backdrop-blur-sm shadow-[var(--layer-depth-1)] overflow-hidden border border-white/5">
            <slot />
          </div>
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
