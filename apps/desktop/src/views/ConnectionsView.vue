<script setup lang="ts">
import { onMounted } from "vue";

import { useHubStore } from "../stores/hub";

const hub = useHubStore();

onMounted(() => {
  void hub.loadConnectionStatus();
});
</script>

<template>
  <section class="connections-layout">
    <article class="surface-card hero">
      <p class="eyebrow">Hub Connections</p>
      <h1>
        {{ hub.connectionStatus?.mode ?? "unknown" }} /
        {{ hub.connectionStatus?.state ?? "unknown" }}
      </h1>
      <p class="muted">
        Auth state: {{ hub.connectionStatus?.auth_state ?? "unknown" }}
      </p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Connection State</p>
      <div class="meta-list">
        <span>Active servers: {{ hub.connectionStatus?.active_server_count ?? 0 }}</span>
        <span>Healthy servers: {{ hub.connectionStatus?.healthy_server_count ?? 0 }}</span>
        <span>Last refresh: {{ hub.connectionStatus?.last_refreshed_at ?? "n/a" }}</span>
      </div>

      <ul v-if="(hub.connectionStatus?.servers.length ?? 0) > 0" class="stack-list">
        <li
          v-for="server in hub.connectionStatus?.servers ?? []"
          :key="server.id"
          class="server-card"
        >
          <strong>{{ server.namespace }}</strong>
          <div class="meta-list">
            <span>Capability: {{ server.capability_id }}</span>
            <span>Platform: {{ server.platform }}</span>
            <span>Trust: {{ server.trust_level }}</span>
            <span>Health: {{ server.health_status }}</span>
          </div>
        </li>
      </ul>

      <p v-else class="muted">No connector servers are currently registered in this surface.</p>
    </article>
  </section>
</template>

<style scoped>
.connections-layout {
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
    radial-gradient(circle at top right, rgba(34, 197, 94, 0.18), transparent 32%),
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

.meta-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  font-size: 0.92rem;
  color: #cbd5e1;
}

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.server-card {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  padding: 0.95rem;
  border-radius: 0.9rem;
  background: rgba(2, 6, 23, 0.6);
}
</style>
