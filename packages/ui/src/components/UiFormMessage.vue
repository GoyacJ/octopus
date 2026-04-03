<script setup lang="ts">
defineProps<{
  message?: string
}>()
</script>

<template>
  <p v-if="message" class="text-sm text-muted-foreground">{{ message }}</p>
</template>
