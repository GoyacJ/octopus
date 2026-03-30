<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";

import { useHubStore } from "../stores/hub";
import { usePreferencesStore } from "../stores/preferences";

const route = useRoute();
const hub = useHubStore();
const preferences = usePreferencesStore();

preferences.initialize();

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
  <section class="run-layout">
    <article class="surface-card hero">
      <p class="eyebrow">{{ preferences.t("run.conclusion") }}</p>
      <h1>{{ runDetail?.task.title ?? "Run loading" }}</h1>
      <p class="muted">{{ preferences.t("run.subtitle") }}</p>
      <div class="meta-list">
        <span>Run: {{ runDetail?.run.id ?? "loading" }}</span>
        <span>Status: {{ runDetail?.run.status ?? "loading" }}</span>
        <span>Attempts: {{ runDetail?.run.attempt_count ?? 0 }}/{{ runDetail?.run.max_attempts ?? 0 }}</span>
      </div>
      <p class="muted">{{ runDetail?.task.instruction ?? "Loading task instruction..." }}</p>
      <div v-if="canRetryRun || canTerminateRun" class="action-row">
        <button
          v-if="canRetryRun"
          data-testid="retry-run"
          :disabled="hub.readOnlyMode || hub.runLoading || hub.runActionLoading"
          @click="handleRetryRun"
        >
          {{ runActionLabel("retry", "Retry Run", "Retrying...") }}
        </button>
        <button
          v-if="canTerminateRun"
          data-testid="terminate-run"
          class="secondary-button"
          :disabled="hub.readOnlyMode || hub.runLoading || hub.runActionLoading"
          @click="handleTerminateRun"
        >
          {{ runActionLabel("terminate", "Terminate Run", "Terminating...") }}
        </button>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">{{ preferences.t("run.result") }}</p>
      <h2>{{ artifacts[0]?.artifact_type ?? "No artifact yet" }}</h2>
      <p class="artifact-content">{{ artifacts[0]?.content ?? "No artifact content recorded." }}</p>
      <div v-if="artifacts[0]" class="meta-list">
        <span>Trust: {{ artifacts[0].trust_level }}</span>
        <span>Knowledge gate: {{ artifacts[0].knowledge_gate_status }}</span>
        <span>Provenance: {{ artifacts[0].provenance_source }}</span>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">{{ preferences.t("run.governance") }}</p>
      <h2>Model Selection Decision</h2>
      <template v-if="modelSelectionDecision">
        <p>{{ modelSelectionDecision.requested_intent }}</p>
        <div class="meta-list">
          <span>Outcome: {{ modelSelectionDecision.decision_outcome }}</span>
          <span v-if="modelSelectionDecision.model_profile_id">
            Profile: {{ modelSelectionDecision.model_profile_id }}
          </span>
          <span v-if="modelSelectionDecision.selected_model_key">
            Model: {{ modelSelectionDecision.selected_model_key }}
          </span>
          <span v-if="modelSelectionDecision.selected_provider_id">
            Provider: {{ modelSelectionDecision.selected_provider_id }}
          </span>
        </div>
        <p>{{ modelSelectionDecision.decision_reason }}</p>
        <div class="meta-list">
          <span>
            Required features:
            {{ modelSelectionDecision.required_feature_tags.join(", ") || "none" }}
          </span>
          <span>
            Missing features:
            {{ modelSelectionDecision.missing_feature_tags.join(", ") || "none" }}
          </span>
          <span>
            Approval:
            {{ modelSelectionDecision.requires_approval ? "required" : "not required" }}
          </span>
        </div>
      </template>
      <p v-else class="muted">
        No run-scoped model selection decision is recorded for this run yet.
      </p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">{{ preferences.t("run.governance") }}</p>
      <h2>{{ policyDecisions.length }} governance records</h2>
      <div v-if="policyDecisions.length > 0" class="stack-list">
        <div v-for="decision in policyDecisions" :key="decision.id">
          <strong>{{ decision.decision }}</strong>
          <p>{{ decision.reason }}</p>
          <div class="meta-list">
            <span>Capability: {{ decision.capability_id }}</span>
            <span>Estimated cost: {{ decision.estimated_cost }}</span>
            <span v-if="decision.approval_request_id">
              Approval: {{ decision.approval_request_id }}
            </span>
          </div>
        </div>
      </div>
      <p v-else class="muted">No policy decision chain is recorded for this run yet.</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">{{ preferences.t("run.governance") }}</p>
      <h2>Shared Knowledge</h2>
      <p>{{ knowledgeDetail?.knowledge_space.display_name ?? "Knowledge loading" }}</p>
      <div class="stack-list">
        <div v-for="candidate in knowledgeDetail?.candidates ?? []" :key="candidate.id">
          <strong>{{ candidate.id }}</strong>
          <p class="muted">Status: {{ candidate.status }}</p>
          <p>{{ candidate.content }}</p>
          <div v-if="approvalForCandidate(candidate.id)" class="meta-list">
            <span>{{ approvalForCandidate(candidate.id)?.approval_type }}</span>
            <span>{{ approvalForCandidate(candidate.id)?.status }}</span>
            <span>{{ approvalForCandidate(candidate.id)?.target_ref }}</span>
          </div>
          <button
            :data-testid="`request-promotion-${candidate.id}`"
            :disabled="hub.readOnlyMode || hub.governanceActionLoading"
            @click="handleRequestPromotion(candidate.id)"
          >
            {{
              governanceActionLabel(
                candidate.id,
                "Request Promotion",
                "Requesting..."
              )
            }}
          </button>
        </div>
      </div>
      <p v-if="(knowledgeDetail?.candidates.length ?? 0) === 0" class="muted">
        No candidates have been captured for this run.
      </p>
      <div v-if="(knowledgeDetail?.assets.length ?? 0) > 0" class="stack-list">
        <div v-for="asset in knowledgeDetail?.assets ?? []" :key="asset.id">
          <strong>{{ asset.id }}</strong>
          <p class="muted">{{ asset.status }}</p>
        </div>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">{{ preferences.t("run.governance") }}</p>
      <h2>Approval / Notification State</h2>
      <div class="meta-list">
        <span>Approvals: {{ runDetail?.approvals.length ?? 0 }}</span>
        <span>Inbox items: {{ runDetail?.inbox_items.length ?? 0 }}</span>
        <span>Notifications: {{ runDetail?.notifications.length ?? 0 }}</span>
      </div>
      <div v-if="(runDetail?.approvals.length ?? 0) > 0" class="stack-list">
        <div v-for="approval in runDetail?.approvals ?? []" :key="approval.id">
          <strong>{{ approval.approval_type }}</strong>
          <p>{{ approval.target_ref }}</p>
          <p class="muted">{{ approval.status }}</p>
          <div class="action-row">
            <button
              :data-testid="`run-approve-${approval.id}`"
              :disabled="hub.readOnlyMode || hub.governanceActionLoading"
              @click="handleResolveApproval(approval.id, 'approve')"
            >
              {{
                governanceActionLabel(
                  approval.id,
                  "Approve",
                  "Approving..."
                )
              }}
            </button>
            <button
              class="secondary-button"
              :data-testid="`run-reject-${approval.id}`"
              :disabled="hub.readOnlyMode || hub.governanceActionLoading"
              @click="handleResolveApproval(approval.id, 'reject')"
            >
              {{
                governanceActionLabel(
                  approval.id,
                  "Reject",
                  "Rejecting..."
                )
              }}
            </button>
          </div>
        </div>
      </div>
      <p class="muted">
        This minimum surface keeps the authoritative approval state visible from the run.
      </p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">{{ preferences.t("run.diagnosis") }}</p>
      <h2>Trace Replay</h2>
      <div v-if="(runDetail?.traces.length ?? 0) > 0" class="stack-list">
        <div v-for="trace in runDetail?.traces ?? []" :key="trace.id">
          <strong>{{ trace.stage }}</strong>
          <p>{{ trace.message }}</p>
        </div>
      </div>
      <p v-else class="muted">No trace replay is recorded for this run yet.</p>
    </article>
  </section>
</template>

<style scoped>
.run-layout {
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
h2,
p {
  margin: 0;
}

.artifact-content {
  white-space: pre-wrap;
  font-size: 1.02rem;
  color: #e2e8f0;
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
