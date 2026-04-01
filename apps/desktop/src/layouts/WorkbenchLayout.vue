<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'

import WorkbenchSearchOverlay from '@/components/layout/WorkbenchSearchOverlay.vue'
import WorkbenchSidebar from '@/components/layout/WorkbenchSidebar.vue'
import WorkbenchTopbar from '@/components/layout/WorkbenchTopbar.vue'
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
  <div
    class="app-shell-grid"
    :class="{
      'left-collapsed': shell.leftSidebarCollapsed,
    }"
  >
    <WorkbenchTopbar class="shell-topbar" />
    <div class="workbench-body">
      <WorkbenchSidebar class="shell-sidebar" />
      <main class="workbench-main scroll-y" data-testid="workbench-main">
        <slot />
      </main>
    </div>
    <WorkbenchSearchOverlay />
  </div>
</template>

<style scoped>
.shell-topbar {
  grid-area: topbar;
}

.workbench-body {
  grid-area: body;
  min-height: 0;
}

.shell-sidebar {
  grid-area: sidebar;
  min-height: 0;
  overflow: hidden;
}

.workbench-main {
  grid-area: main;
  min-width: 0;
  padding: 1rem 1.1rem 1.2rem;
}

@media (max-width: 980px) {
  .workbench-main {
    padding: 0.85rem 0.9rem 1rem;
  }
}
</style>
