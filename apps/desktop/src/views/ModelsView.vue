<script setup lang="ts">
import { computed, watch } from "vue";
import { RouterLink, useRoute } from "vue-router";

import { useHubStore } from "../stores/hub";
import { usePreferencesStore } from "../stores/preferences";

const route = useRoute();
const hub = useHubStore();
const preferences = usePreferencesStore();

preferences.initialize();

const providers = computed(() => hub.workspaceModelProviders);
const catalogItems = computed(() => hub.workspaceModelCatalogItems);
const profiles = computed(() => hub.workspaceModelProfiles);
const workspacePolicy = computed(() => hub.workspaceModelPolicy);
const surfaceState = computed(() => hub.workspaceModelsState);
const modelSelectionDecision = computed(
  () => hub.runDetail?.model_selection_decision ?? null
);
const currentRunRoute = computed(() => {
  if (!hub.currentRunId) {
    return null;
  }

  return `/runs/${hub.currentRunId}`;
});

const stateMessage = computed(() => {
  switch (surfaceState.value) {
    case "offline":
      return "Remote hub is offline";
    case "forbidden":
      return "You do not have permission to read workspace model governance";
    case "auth_required":
      return "Sign in again to load workspace model governance";
    default:
      return null;
  }
});

function loadWorkspaceModelsForRoute(): void {
  const workspaceId =
    typeof route.params.workspaceId === "string" ? route.params.workspaceId : "";
  if (!workspaceId) {
    return;
  }

  void hub.loadWorkspaceModels(workspaceId).catch(() => undefined);
}

watch(
  () => route.params.workspaceId,
  () => {
    loadWorkspaceModelsForRoute();
  },
  { immediate: true }
);
</script>

<template>
  <section class="models-layout">
    <article class="surface-card hero">
      <p class="eyebrow">{{ preferences.t("nav.models") }}</p>
      <h1>{{ preferences.t("models.title") }}</h1>
      <p class="read-only">Read-only</p>
      <p class="muted">{{ preferences.t("models.subtitle") }}</p>
      <p v-if="hub.workspaceModelsLoading" class="muted">
        Loading workspace model governance...
      </p>
      <p v-else-if="stateMessage" class="status-message">{{ stateMessage }}</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Providers</p>
      <div v-if="providers.length > 0" class="stack-list">
        <div v-for="provider in providers" :key="provider.id">
          <strong>{{ provider.display_name }}</strong>
          <p class="muted">{{ provider.provider_family }} / {{ provider.status }}</p>
          <p class="muted">
            Base URL: {{ provider.default_base_url ?? "not recorded" }}
          </p>
        </div>
      </div>
      <p v-else class="muted">No model providers are recorded</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Catalog</p>
      <div v-if="catalogItems.length > 0" class="stack-list">
        <div v-for="item in catalogItems" :key="item.id">
          <strong>{{ item.model_key }}</strong>
          <div class="meta-list">
            <span>Provider: {{ item.provider_id }}</span>
            <span>Channel: {{ item.release_channel }}</span>
            <span>Context window: {{ item.context_window }}</span>
          </div>
        </div>
      </div>
      <p v-else class="muted">No model catalog items are recorded</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Profiles</p>
      <div v-if="profiles.length > 0" class="stack-list">
        <div v-for="profile in profiles" :key="profile.id">
          <strong>{{ profile.display_name }}</strong>
          <p class="muted">Primary model: {{ profile.primary_model_key }}</p>
          <p class="muted">
            Fallbacks:
            {{ profile.fallback_model_keys.join(", ") || "none recorded" }}
          </p>
        </div>
      </div>
      <p v-else class="muted">No model profiles are recorded</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Workspace Policy</p>
      <template v-if="workspacePolicy">
        <h2>{{ workspacePolicy.id }}</h2>
        <div class="meta-list">
          <span>Tenant: {{ workspacePolicy.tenant_id }}</span>
          <span>
            Preview models require approval:
            {{ workspacePolicy.require_approval_for_preview ? "yes" : "no" }}
          </span>
        </div>
        <p v-if="workspacePolicy.require_approval_for_preview">
          Preview models require approval
        </p>
        <div class="meta-list">
          <span>
            Allowed models:
            {{ workspacePolicy.allowed_model_keys.join(", ") || "none recorded" }}
          </span>
          <span>
            Allowed providers:
            {{ workspacePolicy.allowed_provider_ids.join(", ") || "none recorded" }}
          </span>
        </div>
      </template>
      <p v-else class="muted">No workspace model policy is recorded</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Run Link</p>
      <template v-if="modelSelectionDecision">
        <strong>{{ modelSelectionDecision.requested_intent }}</strong>
        <p>{{ modelSelectionDecision.decision_reason }}</p>
        <p class="muted">
          Selected model:
          {{ modelSelectionDecision.selected_model_key ?? "not recorded" }}
        </p>
      </template>
      <p v-else class="muted">
        Run-scoped model selection remains authoritative on the run detail surface.
      </p>
      <RouterLink
        v-if="currentRunRoute"
        :to="currentRunRoute"
        data-testid="models-open-current-run"
      >
        Open current run
      </RouterLink>
    </article>
  </section>
</template>

<style scoped>
.models-layout {
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
    radial-gradient(circle at top right, rgba(56, 189, 248, 0.18), transparent 32%),
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
h2,
p {
  margin: 0;
}

.read-only {
  font-weight: 600;
  color: #facc15;
}

.muted {
  color: #94a3b8;
}

.status-message {
  color: #f8fafc;
}

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.meta-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  font-size: 0.92rem;
  color: #cbd5e1;
}

a {
  width: fit-content;
  border: 1px solid rgba(103, 232, 249, 0.25);
  border-radius: 999px;
  padding: 0.75rem 1rem;
  color: inherit;
  text-decoration: none;
  background: rgba(15, 23, 42, 0.72);
}
</style>
