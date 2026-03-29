<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useRouter } from "vue-router";

import {
  DEFAULT_LOCAL_WORKBENCH_ROUTE,
  resolveDesktopEntryRoute,
  useConnectionStore
} from "../stores/connection";
import { useHubStore } from "../stores/hub";

const hub = useHubStore();
const connection = useConnectionStore();
const router = useRouter();

const selectedMode = ref(connection.profile.mode);
const remoteDraft = reactive({
  baseUrl: connection.profile.baseUrl,
  workspaceId: connection.profile.workspaceId,
  email: connection.profile.email,
  password: ""
});

const stateLabel = computed(() => hub.connectionStatus?.state ?? "connecting");
const authStateLabel = computed(() => hub.connectionStatus?.auth_state ?? "auth_required");
const remoteFormDisabled = computed(() => connection.authLoading);
const loginDisabled = computed(
  () =>
    !remoteDraft.baseUrl.trim() ||
    !remoteDraft.workspaceId.trim() ||
    !remoteDraft.email.trim() ||
    !remoteDraft.password.trim() ||
    connection.authLoading
);
const authMessage = computed(() => {
  if (selectedMode.value === "local") {
    return "Local host stays available through the existing Tauri bridge and demo workspace seed.";
  }

  if (hub.connectionStatus?.auth_state === "token_expired") {
    return "Session expired. Sign in again to restore read/write access.";
  }

  if (connection.session && hub.authState !== "authenticated") {
    return `Cached session restored for ${connection.session.email} in ${connection.session.workspace_id}, but the remote hub is read-only until the connection recovers.`;
  }

  if (hub.connectionStatus?.state === "disconnected") {
    return "Remote hub is disconnected. Check the base URL and retry once the host is reachable.";
  }

  if (hub.connectionStatus?.auth_state === "authenticated") {
    return connection.session
      ? `Remote session active for ${connection.session.email} in ${connection.session.workspace_id}.`
      : "Remote session is authenticated.";
  }

  return "Connect Remote Hub to enter the workspace-scoped remote workbench.";
});

function syncDraftFromProfile(): void {
  selectedMode.value = connection.profile.mode;
  remoteDraft.baseUrl = connection.profile.baseUrl;
  remoteDraft.workspaceId = connection.profile.workspaceId;
  remoteDraft.email = connection.profile.email;
  remoteDraft.password = "";
}

async function loadConnectionSurface(): Promise<void> {
  try {
    await hub.loadConnectionStatus();
  } catch {
    // The shell already surfaces connection errors.
  }
}

async function applyProfile(): Promise<void> {
  connection.clearAuthError();

  if (selectedMode.value === "local") {
    await connection.applyProfile({
      ...connection.profile,
      mode: "local"
    });
    syncDraftFromProfile();
    await router.push(DEFAULT_LOCAL_WORKBENCH_ROUTE);
    return;
  }

  await connection.applyProfile({
    mode: "remote",
    baseUrl: remoteDraft.baseUrl,
    workspaceId: remoteDraft.workspaceId,
    email: remoteDraft.email
  });
  syncDraftFromProfile();
  await router.push("/connections");
}

async function handleLogin(): Promise<void> {
  try {
    const password = remoteDraft.password;
    await connection.applyProfile({
      mode: "remote",
      baseUrl: remoteDraft.baseUrl,
      workspaceId: remoteDraft.workspaceId,
      email: remoteDraft.email
    });
    await connection.login(password);
    syncDraftFromProfile();
    await router.push(resolveDesktopEntryRoute(connection.profile));
  } catch {
    // The store already exposes the auth error for the view.
  }
}

async function handleLogout(): Promise<void> {
  try {
    await connection.logout();
    syncDraftFromProfile();
    await router.push("/connections");
  } catch {
    // The store already exposes the auth error for the view.
  }
}

watch(
  () => connection.profile,
  () => {
    syncDraftFromProfile();
  },
  { deep: true }
);

onMounted(() => {
  syncDraftFromProfile();
  void loadConnectionSurface();
  if (
    connection.remoteMode &&
    connection.hasRemoteAccessToken &&
    !connection.sessionActive &&
    !connection.session
  ) {
    void connection.refreshCurrentSession();
  }
});
</script>

<template>
  <section class="connections-layout">
    <article class="surface-card hero">
      <p class="eyebrow">Hub Connections</p>
      <h1>
        {{ hub.connectionStatus?.mode ?? selectedMode }} /
        {{ stateLabel }}
      </h1>
      <p class="muted">
        Auth state: {{ authStateLabel }}
      </p>
      <p class="muted">{{ authMessage }}</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Connection Profile</p>
      <label class="field-stack">
        <span>Mode</span>
        <select v-model="selectedMode" data-testid="connection-mode">
          <option value="local">local</option>
          <option value="remote">remote</option>
        </select>
      </label>

      <div v-if="selectedMode === 'remote'" class="field-grid">
        <label class="field-stack">
          <span>Base URL</span>
          <input
            v-model="remoteDraft.baseUrl"
            :disabled="remoteFormDisabled"
            type="url"
            placeholder="http://127.0.0.1:4000"
          />
        </label>
        <label class="field-stack">
          <span>Workspace</span>
          <input
            v-model="remoteDraft.workspaceId"
            :disabled="remoteFormDisabled"
            type="text"
            placeholder="workspace-alpha"
          />
        </label>
        <label class="field-stack">
          <span>Email</span>
          <input
            v-model="remoteDraft.email"
            :disabled="remoteFormDisabled"
            type="email"
            placeholder="admin@octopus.local"
          />
        </label>
      </div>

      <div class="action-row">
        <button data-testid="connection-apply" @click="applyProfile">Apply Mode</button>
      </div>

      <p v-if="connection.authError" class="error-copy">{{ connection.authError }}</p>
      <p v-if="connection.secureSessionWarning" class="warning-copy">
        {{ connection.secureSessionWarning }}
      </p>
    </article>

    <article v-if="selectedMode === 'remote'" class="surface-card">
      <p class="eyebrow">Connect Remote Hub</p>
      <div v-if="connection.session" class="meta-list">
        <span>Session: {{ connection.session.session_id }}</span>
        <span>User: {{ connection.session.email }}</span>
        <span>Workspace: {{ connection.session.workspace_id }}</span>
        <span>Expires: {{ connection.session.expires_at }}</span>
      </div>

      <label v-if="hub.authState !== 'authenticated'" class="field-stack">
        <span>Password</span>
        <input
          v-model="remoteDraft.password"
          data-testid="remote-password"
          :disabled="remoteFormDisabled"
          type="password"
          placeholder="octopus-bootstrap-password"
        />
      </label>

      <div class="action-row">
        <button
          v-if="hub.authState !== 'authenticated'"
          data-testid="remote-login"
          :disabled="loginDisabled"
          @click="handleLogin"
        >
          {{ connection.authLoading ? "Signing In..." : "Sign In" }}
        </button>
        <button
          v-else
          data-testid="remote-logout"
          class="secondary-button"
          :disabled="connection.authLoading"
          @click="handleLogout"
        >
          {{ connection.authLoading ? "Signing Out..." : "Sign Out" }}
        </button>
      </div>
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

.field-grid {
  display: grid;
  gap: 0.85rem;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
}

.field-stack {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  color: #cbd5e1;
}

input,
select {
  border: 1px solid rgba(148, 163, 184, 0.26);
  border-radius: 0.9rem;
  padding: 0.8rem 0.9rem;
  font: inherit;
  color: #e2e8f0;
  background: rgba(2, 6, 23, 0.72);
}

.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}

button {
  border: none;
  border-radius: 999px;
  padding: 0.75rem 1rem;
  font: inherit;
  font-weight: 600;
  color: #082f49;
  background: linear-gradient(135deg, #67e8f9, #facc15);
  cursor: pointer;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.secondary-button {
  color: #e2e8f0;
  background: rgba(30, 41, 59, 0.9);
}

.error-copy {
  margin: 0;
  color: #fecaca;
}

.warning-copy {
  margin: 0;
  color: #fbbf24;
}
</style>
