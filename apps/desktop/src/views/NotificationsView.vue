<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute } from "vue-router";

import { useHubStore } from "../stores/hub";

const route = useRoute();
const hub = useHubStore();

async function loadNotificationSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);

  await Promise.all([
    hub.loadConnectionStatus(),
    hub.loadNotifications(workspaceId)
  ]);
}

watch(
  () => route.params.workspaceId,
  () => {
    void loadNotificationSurface();
  }
);

onMounted(() => {
  void loadNotificationSurface();
});
</script>

<template>
  <section class="notifications-layout">
    <article class="surface-card hero">
      <p class="eyebrow">Notifications</p>
      <h1>{{ hub.notifications.length }} pending signals</h1>
      <p class="muted">Read-only reminders remain visible without approval actions in this slice.</p>
    </article>

    <article class="surface-card">
      <ul v-if="hub.notifications.length > 0" class="stack-list">
        <li
          v-for="notification in hub.notifications"
          :key="notification.id"
          class="notification-card"
        >
          <strong>{{ notification.title }}</strong>
          <p>{{ notification.message }}</p>
          <div class="meta-list">
            <span>Status: {{ notification.status }}</span>
            <span>Target: {{ notification.target_ref }}</span>
          </div>
        </li>
      </ul>

      <p v-else class="muted">No notifications are pending.</p>
    </article>
  </section>
</template>

<style scoped>
.notifications-layout {
  display: grid;
  gap: 1rem;
}

.surface-card {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  padding: 1.2rem;
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 1rem;
  background: rgba(15, 23, 42, 0.45);
  box-shadow: 0 20px 40px rgba(15, 23, 42, 0.18);
}

.hero {
  background:
    radial-gradient(circle at top right, rgba(103, 232, 249, 0.18), transparent 32%),
    rgba(15, 23, 42, 0.56);
}

.eyebrow {
  margin: 0;
  font-size: 0.72rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #67e8f9;
}

h1,
p {
  margin: 0;
}

.muted {
  color: #94a3b8;
}

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.notification-card {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  padding: 0.95rem;
  border-radius: 0.9rem;
  background: rgba(2, 6, 23, 0.6);
}

.meta-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  font-size: 0.92rem;
  color: #cbd5e1;
}
</style>
