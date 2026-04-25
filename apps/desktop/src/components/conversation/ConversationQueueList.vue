<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  title: string
  description?: string
  items: Array<{
    id: string
    content: string
    actorLabel: string
    createdAt: number
  }>
}>()

const emit = defineEmits<{
  (event: 'remove', queueItemId: string): void
}>()

const expandedItemIds = ref<string[]>([])

watch(
  () => props.items.map((item) => item.id).join('|'),
  (itemIds) => {
    const validIds = new Set(itemIds.split('|').filter(Boolean))
    expandedItemIds.value = expandedItemIds.value.filter((itemId) => validIds.has(itemId))
  },
)

function isExpanded(queueItemId: string) {
  return expandedItemIds.value.includes(queueItemId)
}

function toggleExpanded(queueItemId: string) {
  expandedItemIds.value = isExpanded(queueItemId)
    ? expandedItemIds.value.filter((itemId) => itemId !== queueItemId)
    : [...expandedItemIds.value, queueItemId]
}
</script>

<template>
  <section v-if="items.length" class="queue-shell" data-testid="conversation-queue-list">
    <header class="queue-header">
      <div class="queue-title" data-testid="conversation-queue-title">{{ title }}</div>
      <p class="queue-description">{{ description }}</p>
    </header>
    <article
      v-for="item in items"
      :key="item.id"
      class="queue-item"
      :class="{ expanded: isExpanded(item.id) }"
      :data-testid="`conversation-queue-item-${item.id}`"
    >
      <button
        type="button"
        class="queue-summary"
        :data-testid="`conversation-queue-toggle-${item.id}`"
        :title="`${item.actorLabel}: ${item.content}`"
        @click="toggleExpanded(item.id)"
      >
        <span class="queue-line">
          <strong class="queue-actor">{{ item.actorLabel }}</strong>
          <span class="queue-separator">:</span>
          <span class="queue-message">{{ item.content }}</span>
        </span>
      </button>
      <button
        type="button"
        class="queue-remove"
        :data-testid="`conversation-queue-remove-${item.id}`"
        @click.stop="emit('remove', item.id)"
      >
        删除
      </button>
    </article>
  </section>
</template>

<style scoped>
.queue-shell,
.queue-item,
.queue-summary,
.queue-line,
.queue-header {
  display: flex;
}

.queue-shell {
  flex-direction: column;
  width: 100%;
  max-height: min(10rem, 34vh);
  padding: 0.35rem 0.6rem;
  overflow-y: auto;
  border-radius: 0.9rem;
  background: color-mix(in srgb, var(--bg-surface) 80%, var(--bg-subtle));
}

.queue-header {
  flex-direction: column;
  gap: 0.1rem;
  padding-bottom: 0.35rem;
}

.queue-title {
  font-size: 0.74rem;
  font-weight: 700;
  color: var(--text-primary);
}

.queue-description {
  margin: 0;
  font-size: 0.7rem;
  line-height: 1.4;
  color: var(--text-secondary);
}

.queue-item {
  align-items: center;
  justify-content: space-between;
  gap: 0.4rem;
}

.queue-item {
  align-items: flex-start;
  padding: 0.38rem 0;
}

.queue-item + .queue-item {
  border-top: 1px solid color-mix(in srgb, var(--text-muted) 12%, transparent);
}

.queue-summary {
  flex: 1;
  min-width: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
}

.queue-line {
  min-width: 0;
  align-items: baseline;
  gap: 0.22rem;
  font-size: 0.76rem;
  line-height: 1.45;
}

.queue-actor,
.queue-separator {
  flex-shrink: 0;
}

.queue-actor {
  font-weight: 600;
}

.queue-message {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
}

.queue-item.expanded .queue-message {
  white-space: normal;
  overflow: visible;
  text-overflow: unset;
}

.queue-remove {
  padding: 0.12rem 0.36rem;
  border-radius: 999px;
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  white-space: nowrap;
  flex-shrink: 0;
  font-size: 0.7rem;
}
</style>
