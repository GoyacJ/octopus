<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { 
  Network, 
  Lock, 
  Server, 
  CheckCircle2, 
  AlertCircle,
  LogIn,
  LogOut,
  RefreshCcw
} from "lucide-vue-next";

import {
  DEFAULT_LOCAL_WORKBENCH_ROUTE,
  resolveDesktopEntryRoute,
  useConnectionStore
} from "../stores/connection";
import { useHubStore } from "../stores/hub";

// UI Components
import OButton from "../components/ui/OButton.vue";
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import OCardHeader from "../components/ui/OCardHeader.vue";
import OCardContent from "../components/ui/OCardContent.vue";
import OCardFooter from "../components/ui/OCardFooter.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";
import PageGrid from "../components/layout/PageGrid.vue";

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
  <PageContainer>
    <PageHeader
      eyebrow="Hub Connections"
      title="Connection Center"
      :subtitle="authMessage"
    >
      <template #stats>
        <OStatPill label="Mode" :value="hub.connectionStatus?.mode ?? selectedMode" highlight />
        <OStatPill label="Status" :value="stateLabel" />
      </template>
    </PageHeader>

    <PageGrid cols="1-side">
      <template #main>
        <OCard>
          <OCardHeader title="Connection Profile">
            <template #icon><Network :size="20" /></template>
          </OCardHeader>
          
          <OCardContent>
            <div class="form-grid">
              <div class="field">
                <label>Operating Mode</label>
                <select v-model="selectedMode" data-testid="connection-mode">
                  <option value="local">Local Mode (Tauri Bridge)</option>
                  <option value="remote">Remote Hub Mode</option>
                </select>
              </div>

              <div v-if="selectedMode === 'remote'" class="remote-fields">
                <div class="field">
                  <label>Base URL</label>
                  <input
                    v-model="remoteDraft.baseUrl"
                    :disabled="remoteFormDisabled"
                    type="url"
                    placeholder="http://127.0.0.1:4000"
                  />
                </div>
                <div class="field-row">
                  <div class="field">
                    <label>Workspace ID</label>
                    <input
                      v-model="remoteDraft.workspaceId"
                      :disabled="remoteFormDisabled"
                      type="text"
                      placeholder="workspace-alpha"
                    />
                  </div>
                  <div class="field">
                    <label>Admin Email</label>
                    <input
                      v-model="remoteDraft.email"
                      :disabled="remoteFormDisabled"
                      type="email"
                      placeholder="admin@octopus.local"
                    />
                  </div>
                </div>
              </div>
            </div>
          </OCardContent>

          <OCardFooter border>
            <OButton data-testid="connection-apply" @click="applyProfile">
              <template #icon-left><RefreshCcw :size="16" /></template>
              Apply Configuration
            </OButton>
          </OCardFooter>

          <div v-if="connection.authError" class="alert danger">
            <AlertCircle :size="16" />
            <span>{{ connection.authError }}</span>
          </div>
          <div v-if="connection.secureSessionWarning" class="alert warning">
            <Lock :size="16" />
            <span>{{ connection.secureSessionWarning }}</span>
          </div>
        </OCard>

        <OCard v-if="selectedMode === 'remote'">
          <OCardHeader title="Authentication">
            <template #icon><Lock :size="20" /></template>
          </OCardHeader>

          <OCardContent>
            <div v-if="connection.session" class="session-info">
              <div class="session-pill">
                <span class="label">User</span>
                <span class="val">{{ connection.session.email }}</span>
              </div>
              <div class="session-pill">
                <span class="label">Expires</span>
                <span class="val">{{ connection.session.expires_at }}</span>
              </div>
            </div>

            <div v-if="hub.authState !== 'authenticated'" class="form-grid">
              <div class="field">
                <label>Password</label>
                <input
                  v-model="remoteDraft.password"
                  data-testid="remote-password"
                  :disabled="remoteFormDisabled"
                  type="password"
                  placeholder="Enter bootstrap password"
                />
              </div>
            </div>
          </OCardContent>

          <OCardFooter border>
            <OButton
              v-if="hub.authState !== 'authenticated'"
              variant="primary"
              data-testid="remote-login"
              :disabled="loginDisabled"
              :loading="connection.authLoading"
              @click="handleLogin"
            >
              <template #icon-left><LogIn :size="16" /></template>
              Sign In to Hub
            </OButton>
            <OButton
              v-else
              variant="secondary"
              data-testid="remote-logout"
              :disabled="connection.authLoading"
              :loading="connection.authLoading"
              @click="handleLogout"
            >
              <template #icon-left><LogOut :size="16" /></template>
              Sign Out
            </OButton>
          </OCardFooter>
        </OCard>
      </template>

      <template #side>
        <OCard padding>
          <h3 class="side-title"><Server :size="16" class="inline-icon" /> Hub Connectors</h3>
          <div class="connector-stats">
            <OStatPill size="sm" label="Active" :value="hub.connectionStatus?.active_server_count ?? 0" />
            <OStatPill size="sm" label="Healthy" :value="hub.connectionStatus?.healthy_server_count ?? 0" />
          </div>

          <ul v-if="(hub.connectionStatus?.servers.length ?? 0) > 0" class="server-list">
            <li v-for="server in hub.connectionStatus?.servers ?? []" :key="server.id" class="server-item">
              <div class="server-header">
                <span class="server-ns">{{ server.namespace }}</span>
                <OPill size="sm" variant="success">{{ server.health_status }}</OPill>
              </div>
              <div class="server-meta">
                <span>{{ server.platform }}</span>
                <span class="divider"></span>
                <span>{{ server.trust_level }}</span>
              </div>
            </li>
          </ul>
          <p v-else class="empty-msg">No connector servers registered.</p>
        </OCard>
      </template>
    </PageGrid>
  </PageContainer>
</template>

<style scoped>
.form-grid {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.remote-fields {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding-top: 1.25rem;
  border-top: 1px solid var(--color-border);
}

.field-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field label { font-size: 0.8125rem; font-weight: 600; color: var(--text-muted); }

input, select {
  padding: 0.75rem 0.875rem;
  background-color: var(--bg-app);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  font-size: 0.9375rem;
  transition: var(--transition);
  width: 100%;
}

input:focus, select:focus {
  border-color: var(--color-accent);
  background-color: white;
  box-shadow: 0 0 0 3px var(--color-accent-soft);
  outline: none;
}

.alert {
  margin: 1rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-lg);
  font-size: 0.875rem;
  display: flex;
  gap: 0.75rem;
  align-items: center;
}

.alert.danger { background-color: var(--color-danger-soft); color: var(--color-danger); }
.alert.warning { background-color: var(--color-warning-soft); color: #92400e; }

.session-info {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}

.session-pill {
  background-color: var(--bg-app);
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-lg);
  display: flex;
  flex-direction: column;
}

.session-pill .label { font-size: 0.625rem; font-weight: 700; color: var(--text-subtle); text-transform: uppercase; }
.session-pill .val { font-size: 0.8125rem; font-weight: 600; }

.side-title {
  font-size: 0.875rem;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--text-subtle);
  margin-bottom: 1rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.connector-stats {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.server-list { list-style: none; padding: 0; margin: 0; }

.server-item {
  padding: 0.75rem;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
  margin-bottom: 0.75rem;
}

.server-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.375rem; }
.server-ns { font-size: 0.875rem; font-weight: 700; color: var(--text-primary); }

.server-meta {
  display: flex;
  gap: 0.5rem;
  font-size: 0.75rem;
  color: var(--text-subtle);
  align-items: center;
}

.server-meta .divider { width: 3px; height: 3px; border-radius: 50%; background: var(--color-border-hover); }

.empty-msg { font-size: 0.8125rem; color: var(--text-subtle); font-style: italic; }

.inline-icon { vertical-align: middle; margin-top: -2px; }
</style>
