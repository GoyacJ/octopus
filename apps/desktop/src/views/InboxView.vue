<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute } from "vue-router";

import { useHubStore } from "../stores/hub";
import { usePreferencesStore } from "../stores/preferences";

const route = useRoute();
const hub = useHubStore();
const preferences = usePreferencesStore();

preferences.initialize();

type InboxItem = ReturnType<typeof useHubStore>["inboxItems"][number];

function approvalForItem(item: InboxItem) {
  return hub.approvalDetails[item.approval_request_id] ?? null;
}

function governanceActionLabel(
  approvalId: string,
  decision: "approve" | "reject"
): string {
  if (hub.governanceActionLoading && hub.governanceActionTarget === approvalId) {
    return decision === "approve" ? "Approving..." : "Rejecting...";
  }

  if (hub.readOnlyMode) {
    return "Read-only";
  }

  return decision === "approve" ? "Approve" : "Reject";
}

async function loadInboxSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);

  await hub.loadInboxItems(workspaceId);
}

async function handleResolveApproval(
  approvalId: string,
  decision: "approve" | "reject"
): Promise<void> {
  try {
    await hub.resolveGovernanceApproval(approvalId, decision, decision);
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

watch(
  () => route.params.workspaceId,
  () => {
    void loadInboxSurface();
  }
);

onMounted(() => {
  void loadInboxSurface();
});
</script>

<template>
  <section class="inbox-layout">
    <article class="surface-card hero">
      <p class="eyebrow">{{ preferences.t("nav.inbox") }}</p>
      <h1>{{ preferences.t("inbox.title") }}</h1>
      <p class="muted">{{ preferences.t("inbox.subtitle") }}</p>
      <p class="muted">{{ hub.inboxItems.length }} open items</p>
    </article>

    <article class="surface-card">
      <ul v-if="hub.inboxItems.length > 0" class="stack-list">
        <li v-for="item in hub.inboxItems" :key="item.id" class="inbox-card">
          <strong>{{ item.title }}</strong>
          <p>{{ item.message }}</p>
          <div v-if="approvalForItem(item)" class="meta-list">
            <span>{{ approvalForItem(item)?.approval_type }}</span>
            <span>{{ approvalForItem(item)?.status }}</span>
            <span>{{ approvalForItem(item)?.target_ref }}</span>
          </div>
          <div class="action-row">
            <button
              :data-testid="`workspace-approve-${item.approval_request_id}`"
              :disabled="hub.readOnlyMode || hub.governanceActionLoading"
              @click="handleResolveApproval(item.approval_request_id, 'approve')"
            >
              {{ governanceActionLabel(item.approval_request_id, 'approve') }}
            </button>
            <button
              class="secondary-button"
              :data-testid="`workspace-reject-${item.approval_request_id}`"
              :disabled="hub.readOnlyMode || hub.governanceActionLoading"
              @click="handleResolveApproval(item.approval_request_id, 'reject')"
            >
              {{ governanceActionLabel(item.approval_request_id, 'reject') }}
            </button>
          </div>
        </li>
      </ul>

      <p v-else class="muted">{{ preferences.t("inbox.empty") }}</p>
    </article>
  </section>
</template>

<style scoped>
.inbox-layout {
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
    radial-gradient(circle at top right, rgba(250, 204, 21, 0.16), transparent 32%),
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

.inbox-card {
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

.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}

button {
  border: none;
  border-radius: 999px;
  padding: 0.85rem 1.05rem;
  font: inherit;
  font-weight: 600;
  color: #082f49;
  background: linear-gradient(135deg, #67e8f9, #facc15);
  cursor: pointer;
}

button:disabled {
  cursor: progress;
  opacity: 0.75;
}

.secondary-button {
  color: #e2e8f0;
  background: rgba(15, 23, 42, 0.72);
  border: 1px solid rgba(125, 211, 252, 0.25);
}
</style>
