<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()

onMounted(() => {
  void router.replace({
    name: 'workspace-agents',
    params: {
      workspaceId: route.params.workspaceId,
    },
    query: {
      ...route.query,
      tab: 'team',
    },
  })
})
</script>

<template>
  <div />
</template>
