<script setup lang="ts">
import { computed, watch } from "vue";
import { RouterLink, useRoute } from "vue-router";
import { 
  Brain, 
  Cpu, 
  ShieldCheck, 
  Zap, 
  ArrowRight,
  AlertTriangle
} from "lucide-vue-next";

import { useHubStore } from "../stores/hub";

// UI Components
import OButton from "../components/ui/OButton.vue";
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import OCardHeader from "../components/ui/OCardHeader.vue";
import OCardContent from "../components/ui/OCardContent.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";
import PageGrid from "../components/layout/PageGrid.vue";

const route = useRoute();
const hub = useHubStore();

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
  <PageContainer>
    <PageHeader
      eyebrow="Governance"
      title="Models & Intelligence"
      subtitle="Workspace model governance and provider configuration."
    >
      <template #stats>
        <OStatPill label="Providers" :value="providers.length" highlight />
        <OStatPill label="Profiles" :value="profiles.length" />
      </template>
    </PageHeader>

    <div v-if="hub.workspaceModelsLoading" class="loading-state">
      <div class="skeleton-banner"></div>
      <div class="skeleton-grid"></div>
    </div>

    <div v-else-if="stateMessage" class="alert warning">
      <AlertTriangle :size="20" />
      <span>{{ stateMessage }}</span>
    </div>

    <PageGrid v-else cols="1-side">
      <template #main>
        <OCard>
          <OCardHeader title="Providers">
            <template #icon><Brain :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <div v-if="providers.length > 0" class="entity-list">
              <div v-for="provider in providers" :key="provider.id" class="entity-item">
                <div class="entity-main">
                  <span class="entity-name">{{ provider.display_name }}</span>
                  <div class="entity-pills">
                    <OPill size="sm">{{ provider.provider_family }}</OPill>
                    <OPill size="sm" :variant="provider.status === 'active' ? 'success' : 'default'">
                      {{ provider.status }}
                    </OPill>
                  </div>
                </div>
                <div class="entity-meta">
                  URL: {{ provider.default_base_url ?? "Standard" }}
                </div>
              </div>
            </div>
            <p v-else class="empty-msg">No providers configured.</p>
          </OCardContent>
        </OCard>

        <OCard>
          <OCardHeader title="Model Catalog">
            <template #icon><Cpu :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <div v-if="catalogItems.length > 0" class="entity-list">
              <div v-for="item in catalogItems" :key="item.id" class="entity-item">
                <div class="entity-main">
                  <span class="entity-name mono">{{ item.model_key }}</span>
                  <OPill size="sm">{{ item.release_channel }}</OPill>
                </div>
                <div class="entity-meta">
                  <span>Provider: {{ item.provider_id }}</span>
                  <span class="divider"></span>
                  <span>Context: {{ item.context_window }}</span>
                </div>
              </div>
            </div>
            <p v-else class="empty-msg">Catalog is empty.</p>
          </OCardContent>
        </OCard>
      </template>

      <template #side>
        <OCard padding>
          <h3 class="side-title"><Zap :size="16" class="inline-icon" /> Active Profiles</h3>
          <div v-if="profiles.length > 0" class="profile-list">
            <div v-for="profile in profiles" :key="profile.id" class="profile-item">
              <div class="profile-name">{{ profile.display_name }}</div>
              <div class="profile-model">Primary: {{ profile.primary_model_key }}</div>
              <div v-if="profile.fallback_model_keys.length" class="profile-fallback">
                Fallback: {{ profile.fallback_model_keys.join(", ") }}
              </div>
            </div>
          </div>
          <p v-else class="empty-msg">No profiles found.</p>
        </OCard>

        <OCard padding variant="highlight">
          <h3 class="side-title"><ShieldCheck :size="16" class="inline-icon" /> Workspace Policy</h3>
          <div v-if="workspacePolicy" class="policy-info">
            <div class="policy-row">
              <span class="label">ID</span>
              <span class="val mono">{{ workspacePolicy.id.slice(0, 8) }}</span>
            </div>
            <div class="policy-row">
              <span class="label">Preview Approv.</span>
              <OPill size="sm" :variant="workspacePolicy.require_approval_for_preview ? 'danger' : 'success'">
                {{ workspacePolicy.require_approval_for_preview ? "Required" : "Not Required" }}
              </OPill>
            </div>
            <div class="policy-section">
              <span class="label">Allowed Models</span>
              <div class="tag-cloud">
                <span v-for="key in workspacePolicy.allowed_model_keys" :key="key" class="tag">{{ key }}</span>
              </div>
            </div>
          </div>
          <p v-else class="empty-msg">No active policy.</p>
        </OCard>

        <OCard v-if="currentRunRoute" padding>
          <h3 class="side-title">Active Run Intelligence</h3>
          <div v-if="modelSelectionDecision" class="decision-mini">
            <div class="decision-intent">{{ modelSelectionDecision.requested_intent }}</div>
            <OPill variant="info">{{ modelSelectionDecision.selected_model_key }}</OPill>
          </div>
          <OButton variant="primary" :to="currentRunRoute" class="full-width">
            View Current Run
            <template #icon-right><ArrowRight :size="16" /></template>
          </OButton>
        </OCard>
      </template>
    </PageGrid>
  </PageContainer>
</template>

<style scoped>
.entity-list { display: flex; flex-direction: column; gap: 1rem; }

.entity-item {
  padding: 1rem;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
}

.entity-main {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.entity-name { font-weight: 700; color: var(--text-primary); }
.entity-name.mono { font-family: monospace; font-size: 0.875rem; }

.entity-pills { display: flex; gap: 0.5rem; }

.entity-meta {
  font-size: 0.75rem;
  color: var(--text-subtle);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.entity-meta .divider { width: 3px; height: 3px; border-radius: 50%; background: var(--color-border-hover); }

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

.profile-item {
  padding: 0.75rem 0;
  border-bottom: 1px solid var(--color-border);
}

.profile-item:last-child { border: none; }

.profile-name { font-weight: 700; font-size: 0.875rem; margin-bottom: 0.25rem; color: var(--text-primary); }
.profile-model, .profile-fallback { font-size: 0.75rem; color: var(--text-muted); }

.policy-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; }
.policy-row .label { font-size: 0.75rem; color: var(--text-muted); }
.policy-row .val { font-size: 0.8125rem; font-weight: 600; }

.policy-section { margin-top: 1rem; }
.policy-section .label { font-size: 0.75rem; color: var(--text-muted); display: block; margin-bottom: 0.5rem; }

.tag-cloud { display: flex; flex-wrap: wrap; gap: 0.375rem; }
.tag { font-size: 0.625rem; font-weight: 700; background: var(--color-border); padding: 0.125rem 0.375rem; border-radius: 4px; color: var(--text-muted); }

.decision-mini { margin-bottom: 1rem; }
.decision-intent { font-size: 0.8125rem; font-weight: 600; margin-bottom: 0.5rem; color: var(--text-primary); }

.alert {
  padding: 1rem;
  margin-bottom: 2rem;
  border-radius: var(--radius-lg);
  font-size: 0.875rem;
  display: flex;
  gap: 0.75rem;
  align-items: center;
}

.alert.warning { background-color: var(--color-warning-soft); color: #92400e; border: 1px solid rgba(245, 158, 11, 0.2); }

.empty-msg { font-size: 0.8125rem; color: var(--text-subtle); font-style: italic; }

.inline-icon { vertical-align: middle; margin-top: -2px; }

.full-width { width: 100%; }
</style>
