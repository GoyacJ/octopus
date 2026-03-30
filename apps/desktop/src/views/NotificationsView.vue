<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { Bell, Info } from "lucide-vue-next";

import { useHubStore } from "../stores/hub";

// UI Components
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";

const route = useRoute();
const hub = useHubStore();

async function loadNotificationSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);

  await hub.loadNotifications(workspaceId);
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
  <PageContainer narrow>
    <PageHeader
      eyebrow="Signals"
      title="Notifications"
      subtitle="System events and status reminders."
    >
      <template #stats>
        <OStatPill label="Pending" :value="hub.notifications.length" highlight />
      </template>
    </PageHeader>

    <div class="notifications-content">
      <div v-if="hub.notifications.length > 0" class="notifications-list">
        <OCard
          v-for="notification in hub.notifications"
          :key="notification.id"
          hover
        >
          <div class="notification-card-inner">
            <div class="card-indicator"></div>
            <div class="card-body">
              <h2 class="notification-title">{{ notification.title }}</h2>
              <p class="notification-message">{{ notification.message }}</p>
              <div class="notification-meta">
                <OPill size="sm" :variant="notification.status === 'pending' ? 'warning' : 'default'">
                  {{ notification.status }}
                </OPill>
                <span class="meta-item">Target: {{ notification.target_ref }}</span>
              </div>
            </div>
          </div>
        </OCard>
      </div>

      <div v-else class="empty-state">
        <div class="empty-icon"><Bell :size="32" /></div>
        <h2 class="empty-title">All Clear</h2>
        <p class="empty-text">No pending notifications at the moment.</p>
      </div>
    </div>
  </PageContainer>
</template>

<style scoped>
.notifications-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.notification-card-inner {
  display: flex;
  overflow: hidden;
}

.card-indicator {
  width: 4px;
  background-color: var(--color-accent);
  flex-shrink: 0;
}

.card-body {
  padding: 1.25rem 1.5rem;
  flex: 1;
}

.notification-title {
  font-size: 1rem;
  font-weight: 700;
  margin: 0 0 0.375rem;
  color: var(--text-primary);
}

.notification-message {
  font-size: 0.9375rem;
  color: var(--text-muted);
  line-height: 1.5;
  margin-bottom: 1rem;
}

.notification-meta {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.meta-item {
  font-size: 0.75rem;
  color: var(--text-subtle);
}

.empty-state {
  text-align: center;
  padding: 4rem 2rem;
  background-color: var(--bg-surface);
  border: 2px dashed var(--color-border);
  border-radius: var(--radius-2xl);
}

.empty-icon {
  width: 3.5rem;
  height: 3.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 1.5rem;
  background-color: var(--bg-app);
  color: var(--text-subtle);
  border-radius: 50%;
}

.empty-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.empty-text {
  color: var(--text-muted);
  font-size: 0.9375rem;
}
</style>
