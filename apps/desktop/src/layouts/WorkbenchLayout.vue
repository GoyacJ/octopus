<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'

import WorkbenchSearchOverlay from '@/components/layout/WorkbenchSearchOverlay.vue'
import WorkbenchSidebar from '@/components/layout/WorkbenchSidebar.vue'
import WorkbenchTopbar from '@/components/layout/WorkbenchTopbar.vue'
import DesktopPetHost from '@/components/pet/DesktopPetHost.vue'
import { useShellStore } from '@/stores/shell'

const shell = useShellStore()

function handleGlobalKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
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
  <div class="flex h-screen w-screen overflow-hidden bg-background font-sans text-text-primary antialiased">
    <!-- Notion Style: Sidebar is full height -->
    <WorkbenchSidebar />

    <div class="flex flex-1 flex-col min-w-0 relative">
      <WorkbenchTopbar />
      
      <main class="flex-1 overflow-y-auto min-w-0" data-testid="workbench-main">
        <!-- Removed fixed padding to allow full-bleed layouts (like Conversation) -->
        <div class="w-full h-full">
          <slot />
        </div>
      </main>
    </div>

    <WorkbenchSearchOverlay />
    <DesktopPetHost />
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
