<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { CheckCircle2, Inbox } from "lucide-vue-next";

import { useHubStore } from "../stores/hub";

// UI Components
import OButton from "../components/ui/OButton.vue";
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";

const route = useRoute();
const hub = useHubStore();

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
  <PageContainer narrow>
    <PageHeader
      eyebrow="Approval Inbox"
      :title="`${hub.inboxItems.length} Pending Items`"
      subtitle="Actionable approval requests requiring your attention."
    />

    <div class="inbox-content">
      <div v-if="hub.inboxItems.length > 0" class="inbox-list">
        <OCard v-for="item in hub.inboxItems" :key="item.id" hover>
          <div class="card-body">
            <div class="card-main">
              <h2 class="item-title">{{ item.title }}</h2>
              <p class="item-message">{{ item.message }}</p>
              
              <div v-if="approvalForItem(item)" class="item-meta">
                <OPill>{{ approvalForItem(item)?.approval_type }}</OPill>
                <OPill :variant="approvalForItem(item)?.status === 'pending' ? 'warning' : 'info'">
                  {{ approvalForItem(item)?.status }}
                </OPill>
                <OPill size="sm" outline>Target: {{ approvalForItem(item)?.target_ref }}</OPill>
              </div>
            </div>

            <div class="card-actions">
              <OButton
                variant="primary"
                :disabled="hub.readOnlyMode || hub.governanceActionLoading"
                :loading="hub.governanceActionLoading && hub.governanceActionTarget === item.approval_request_id"
                @click="handleResolveApproval(item.approval_request_id, 'approve')"
              >
                Approve
              </OButton>
              <OButton
                variant="secondary"
                :disabled="hub.readOnlyMode || hub.governanceActionLoading"
                @click="handleResolveApproval(item.approval_request_id, 'reject')"
              >
                Reject
              </OButton>
            </div>
          </div>
        </OCard>
      </div>

      <div v-else class="empty-state">
        <div class="empty-icon"><CheckCircle2 :size="32" /></div>
        <h2 class="empty-title">All Caught Up</h2>
        <p class="empty-text">No approval requests are waiting for your review.</p>
      </div>
    </div>
  </PageContainer>
</template>

<style scoped>
.inbox-list {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.card-body {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 2rem;
  padding: 1.5rem;
}

.card-main {
  flex: 1;
}

.item-title {
  font-size: 1.125rem;
  font-weight: 700;
  margin: 0 0 0.5rem;
}

.item-message {
  font-size: 0.9375rem;
  color: var(--text-muted);
  line-height: 1.5;
  margin-bottom: 1.25rem;
}

.item-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.card-actions {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 140px;
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
  background-color: var(--color-success-soft);
  color: var(--color-success);
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

@media (max-width: 640px) {
  .card-body { flex-direction: column; gap: 1.5rem; }
  .card-actions { flex-direction: row; width: 100%; }
  .card-actions > * { flex: 1; }
}
</style>
