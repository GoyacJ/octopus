<script setup lang="ts">
import AgentCenterView from '@/views/agents/AgentCenterView.vue'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})
</script>

<template>
  <AgentCenterView scope="workspace" :embedded="props.embedded" />
</template>
