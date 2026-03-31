<script setup lang="ts">
import ConversationContextPane from '@/components/layout/ConversationContextPane.vue'
import WorkbenchSidebar from '@/components/layout/WorkbenchSidebar.vue'
import { useShellStore } from '@/stores/shell'

const shell = useShellStore()
</script>

<template>
  <div class="app-shell-grid" :class="{ compact: shell.preferences.compactSidebar }">
    <WorkbenchSidebar class="shell-sidebar" />
    <main class="workbench-main scroll-y">
      <slot />
    </main>
    <ConversationContextPane class="shell-context" />
  </div>
</template>

<style scoped>
.shell-sidebar {
  grid-area: sidebar;
}

.app-shell-grid.compact {
  grid-template-columns: clamp(216px, 16vw, 248px) minmax(0, 1fr) clamp(304px, 23vw, 392px);
}

.workbench-main {
  grid-area: main;
  min-width: 0;
  padding: 1.5rem;
}

.shell-context {
  grid-area: context;
}

@media (max-width: 1280px) {
  .app-shell-grid.compact {
    grid-template-columns: clamp(204px, 22vw, 236px) minmax(0, 1fr);
  }
}

@media (max-width: 980px) {
  .workbench-main {
    padding: 1rem;
  }
}
</style>
