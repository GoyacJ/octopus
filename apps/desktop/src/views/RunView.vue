<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { 
  FileText, 
  Database, 
  Activity, 
  Cpu, 
  ShieldAlert, 
  CheckCircle2,
  RefreshCw,
  StopCircle,
  ArrowUpRight
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

const runDetail = computed(() => hub.runDetail);
const artifacts = computed(() => hub.artifacts);
const knowledgeDetail = computed(() => hub.knowledgeDetail);
const currentRun = computed(() => runDetail.value?.run ?? null);
const modelSelectionDecision = computed(
  () => runDetail.value?.model_selection_decision ?? null
);
const runApprovals = computed(() => runDetail.value?.approvals ?? []);
const policyDecisions = computed(() => runDetail.value?.policy_decisions ?? []);
const canRetryRun = computed(() => {
  const run = currentRun.value;
  return Boolean(
    run &&
      run.status === "failed" &&
      run.resume_token &&
      run.attempt_count < run.max_attempts
  );
});
const canTerminateRun = computed(() => {
  const status = currentRun.value?.status;
  return (
    status === "created" ||
    status === "running" ||
    status === "failed" ||
    status === "resuming" ||
    status === "blocked" ||
    status === "waiting_approval"
  );
});

function approvalForCandidate(candidateId: string) {
  return runApprovals.value.find(
    (approval) => approval.target_ref === `knowledge_candidate:${candidateId}`
  );
}

function governanceActionLabel(
  target: string,
  idleLabel: string,
  loadingLabel: string
): string {
  if (hub.governanceActionLoading && hub.governanceActionTarget === target) {
    return loadingLabel;
  }

  if (hub.readOnlyMode) {
    return "Read-only";
  }

  return idleLabel;
}

function runActionLabel(
  kind: "retry" | "terminate",
  idleLabel: string,
  loadingLabel: string
): string {
  if (hub.runActionLoading && hub.runActionKind === kind) {
    return loadingLabel;
  }

  if (hub.readOnlyMode) {
    return "Read-only";
  }

  return idleLabel;
}

async function loadRunSurface(): Promise<void> {
  await hub.loadRun(String(route.params.runId));
}

async function handleRetryRun(): Promise<void> {
  if (!currentRun.value) {
    return;
  }

  try {
    await hub.retryRun(currentRun.value.id);
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handleTerminateRun(): Promise<void> {
  if (!currentRun.value) {
    return;
  }

  try {
    await hub.terminateRun(currentRun.value.id);
  } catch {
    // The store already exposes the error banner for the shell.
  }
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

async function handleRequestPromotion(candidateId: string): Promise<void> {
  try {
    await hub.requestKnowledgePromotion(candidateId, "request promotion");
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

watch(
  () => route.params.runId,
  () => {
    void loadRunSurface();
  }
);

onMounted(() => {
  void loadRunSurface();
});
</script>

<template>
  <PageContainer>
    <PageHeader
      eyebrow="Run Detail"
      :title="runDetail?.task.title ?? 'Run Detail'"
      :subtitle="runDetail?.task.instruction"
    >
      <template #stats>
        <OStatPill label="Run ID" :value="runDetail?.run.id ?? '...'" mono />
        <OStatPill label="Status" highlight>
          <span class="status-dot" :class="runDetail?.run.status"></span>
          {{ runDetail?.run.status ?? "..." }}
        </OStatPill>
        <OStatPill label="Attempts" :value="`${runDetail?.run.attempt_count ?? 0} / ${runDetail?.run.max_attempts ?? 0}`" />
      </template>
      <template #actions>
        <OButton
          v-if="canRetryRun"
          variant="primary"
          :disabled="hub.readOnlyMode || hub.runLoading || hub.runActionLoading"
          :loading="hub.runActionLoading && hub.runActionKind === 'retry'"
          @click="handleRetryRun"
        >
          <template #icon-left><RefreshCw :size="16" /></template>
          Retry Run
        </OButton>
        <OButton
          v-if="canTerminateRun"
          variant="danger"
          :disabled="hub.readOnlyMode || hub.runLoading || hub.runActionLoading"
          :loading="hub.runActionLoading && hub.runActionKind === 'terminate'"
          @click="handleTerminateRun"
        >
          <template #icon-left><StopCircle :size="16" /></template>
          Terminate
        </OButton>
      </template>
    </PageHeader>

    <PageGrid cols="1-side">
      <template #main>
        <OCard>
          <OCardHeader title="Artifacts">
            <template #icon><FileText :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <div v-if="artifacts.length > 0" class="artifact-list">
              <div v-for="art in artifacts" :key="art.id" class="artifact-item">
                <div class="artifact-header">
                  <span class="artifact-type">{{ art.artifact_type }}</span>
                  <OPill size="sm" :variant="art.trust_level === 'trusted' ? 'success' : 'default'">
                    {{ art.trust_level }}
                  </OPill>
                </div>
                <pre class="artifact-content">{{ art.content }}</pre>
                <div class="artifact-meta">
                  <span>Gate: {{ art.knowledge_gate_status }}</span>
                  <span>Source: {{ art.provenance_source }}</span>
                </div>
              </div>
            </div>
            <p v-else class="empty-msg">No artifacts generated yet.</p>
          </OCardContent>
        </OCard>

        <OCard>
          <OCardHeader title="Shared Knowledge">
            <template #icon><Database :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <div class="knowledge-section">
              <div v-if="knowledgeDetail?.candidates.length" class="candidate-list">
                <div v-for="cand in knowledgeDetail.candidates" :key="cand.id" class="candidate-item">
                  <div class="item-header">
                    <span class="item-id">{{ cand.id }}</span>
                    <OPill size="sm" variant="warning">{{ cand.status }}</OPill>
                  </div>
                  <p class="item-content">{{ cand.content }}</p>
                  <div class="item-actions">
                    <OButton
                      size="sm"
                      variant="secondary"
                      :disabled="hub.readOnlyMode || hub.governanceActionLoading"
                      @click="handleRequestPromotion(cand.id)"
                    >
                      <template #icon-left><ArrowUpRight :size="14" /></template>
                      Promote
                    </OButton>
                  </div>
                </div>
              </div>
              <p v-else class="empty-msg">No candidates captured.</p>
            </div>
          </OCardContent>
        </OCard>

        <OCard>
          <OCardHeader title="Trace Replay">
            <template #icon><Activity :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <div v-if="runDetail?.traces.length" class="timeline">
              <div v-for="trace in runDetail.traces" :key="trace.id" class="timeline-item">
                <div class="timeline-point"></div>
                <div class="timeline-content">
                  <span class="timeline-stage">{{ trace.stage }}</span>
                  <p class="timeline-msg">{{ trace.message }}</p>
                </div>
              </div>
            </div>
            <p v-else class="empty-msg">No trace recorded.</p>
          </OCardContent>
        </OCard>
      </template>

      <template #side>
        <OCard padding>
          <h3 class="side-title"><Cpu :size="16" class="inline-icon" /> Model Selection</h3>
          <div v-if="modelSelectionDecision" class="model-info">
            <div class="info-row">
              <span class="info-label">Outcome</span>
              <OPill variant="info">{{ modelSelectionDecision.decision_outcome }}</OPill>
            </div>
            <div class="info-row">
              <span class="info-label">Provider</span>
              <span class="info-val">{{ modelSelectionDecision.selected_provider_id }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">Model</span>
              <span class="info-val">{{ modelSelectionDecision.selected_model_key }}</span>
            </div>
            <p class="info-reason">{{ modelSelectionDecision.decision_reason }}</p>
          </div>
          <p v-else class="empty-msg">No model decision.</p>
        </OCard>

        <OCard padding>
          <h3 class="side-title"><ShieldAlert :size="16" class="inline-icon" /> Governance</h3>
          <div v-if="policyDecisions.length" class="policy-list">
            <div v-for="dec in policyDecisions" :key="dec.id" class="policy-item">
              <div class="policy-header">
                <OPill size="sm" :variant="dec.decision === 'allowed' ? 'success' : 'danger'">
                  {{ dec.decision }}
                </OPill>
                <span class="policy-cap">{{ dec.capability_id }}</span>
              </div>
              <p class="policy-reason">{{ dec.reason }}</p>
            </div>
          </div>
          <p v-else class="empty-msg">No policy records.</p>
        </OCard>

        <OCard padding variant="highlight">
          <h3 class="side-title"><CheckCircle2 :size="16" class="inline-icon" /> Approvals</h3>
          <div v-if="runApprovals.length" class="approval-mini-list">
            <div v-for="app in runApprovals" :key="app.id" class="app-mini-item">
              <div class="app-info">
                <span class="app-type">{{ app.approval_type }}</span>
                <OPill size="sm" :variant="app.status === 'pending' ? 'warning' : 'success'">
                  {{ app.status }}
                </OPill>
              </div>
              <div v-if="app.status === 'pending'" class="app-actions">
                <OButton size="sm" variant="primary" @click="handleResolveApproval(app.id, 'approve')">Approve</OButton>
                <OButton size="sm" variant="danger" @click="handleResolveApproval(app.id, 'reject')">Reject</OButton>
              </div>
            </div>
          </div>
          <p v-else class="empty-msg">No active approvals.</p>
        </OCard>
      </template>
    </PageGrid>
  </PageContainer>
</template>

<style scoped>
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--text-subtle);
}
.status-dot.running { background-color: var(--color-accent); animation: pulse 2s infinite; }
.status-dot.completed { background-color: var(--color-success); }
.status-dot.failed { background-color: var(--color-danger); }

@keyframes pulse { 0% { opacity: 1; } 50% { opacity: 0.5; } 100% { opacity: 1; } }

.artifact-item {
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
  padding: 1rem;
  margin-bottom: 1rem;
}

.artifact-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}

.artifact-type { font-weight: 700; font-size: 0.875rem; color: var(--text-primary); }

.artifact-content {
  background-color: #1e293b;
  color: #e2e8f0;
  padding: 1rem;
  border-radius: var(--radius-lg);
  font-family: monospace;
  font-size: 0.875rem;
  overflow-x: auto;
  white-space: pre-wrap;
}

.artifact-meta {
  margin-top: 0.75rem;
  display: flex;
  gap: 1rem;
  font-size: 0.75rem;
  color: var(--text-subtle);
}

.candidate-item {
  border-left: 3px solid var(--color-warning);
  background-color: var(--bg-app);
  padding: 1rem;
  border-radius: 0 var(--radius-lg) var(--radius-lg) 0;
  margin-bottom: 1rem;
}

.item-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}

.item-id { font-family: monospace; font-size: 0.8125rem; font-weight: 600; }
.item-content { font-size: 0.9375rem; color: var(--text-primary); margin-bottom: 0.75rem; }

.timeline {
  padding-left: 0.5rem;
}

.timeline-item {
  position: relative;
  padding-left: 1.5rem;
  padding-bottom: 1.5rem;
  border-left: 1px solid var(--color-border);
}

.timeline-item:last-child { border-left: none; padding-bottom: 0; }

.timeline-point {
  position: absolute;
  left: -4.5px;
  top: 0;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--color-border-hover);
  border: 2px solid white;
}

.timeline-stage { font-size: 0.75rem; font-weight: 700; color: var(--color-accent); text-transform: uppercase; }
.timeline-msg { font-size: 0.875rem; color: var(--text-muted); margin-top: 0.25rem; }

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

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.info-label { font-size: 0.75rem; color: var(--text-muted); }
.info-val { font-size: 0.8125rem; font-weight: 600; }
.info-reason { font-size: 0.75rem; color: var(--text-muted); line-height: 1.4; margin-top: 0.75rem; }

.policy-item {
  padding: 0.75rem 0;
  border-bottom: 1px solid var(--color-border);
}

.policy-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.375rem; }
.policy-cap { font-size: 0.75rem; font-weight: 600; }
.policy-reason { font-size: 0.75rem; color: var(--text-muted); }

.app-mini-item {
  padding: 0.75rem;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
  margin-bottom: 0.75rem;
}

.app-info { display: flex; justify-content: space-between; margin-bottom: 0.75rem; }
.app-type { font-size: 0.8125rem; font-weight: 600; }
.app-actions { display: flex; gap: 0.5rem; }

.empty-msg { font-size: 0.8125rem; color: var(--text-subtle); font-style: italic; }

.inline-icon { vertical-align: middle; margin-top: -2px; }
</style>
