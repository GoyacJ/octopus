<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import {
  buildProjectTasksRoute,
  buildWorkspaceProjectsRoute,
  useConnectionStore
} from "../stores/connection";
import { useConversationStore } from "../stores/conversation";
import { usePreferencesStore } from "../stores/preferences";
import { useHubStore } from "../stores/hub";

const route = useRoute();
const router = useRouter();
const hub = useHubStore();
const preferences = usePreferencesStore();
const conversations = useConversationStore();
const connection = useConnectionStore();

preferences.initialize();
conversations.initialize();

const composer = ref("");
const constraintDraft = ref("");

const workspaceId = computed(() => String(route.params.workspaceId));
const projectId = computed(() => String(route.params.projectId));
const threadId = computed(() =>
  typeof route.query.thread === "string" ? route.query.thread : null
);
const thread = computed(() =>
  conversations.ensureThread(workspaceId.value, projectId.value, threadId.value)
);

const executionReadinessLabel = computed(() => {
  switch (thread.value.proposal.executionReadiness) {
    case "ready":
      return preferences.t("conversation.readinessReady");
    case "reconfirm_required":
      return preferences.t("conversation.readinessReconfirm");
    default:
      return preferences.t("conversation.readinessDrafting");
  }
});

async function loadConversationContext(): Promise<void> {
  try {
    await hub.loadProjectContext(workspaceId.value, projectId.value);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    const rememberedProjectId = connection.profile.projectId ?? "";

    if (
      connection.remoteMode &&
      rememberedProjectId === projectId.value &&
      message.includes(`project \`${projectId.value}\` not found`)
    ) {
      connection.clearRememberedProject();
      await router.replace(buildWorkspaceProjectsRoute(workspaceId.value));
    }
  }
}

function sendTurn(): void {
  conversations.appendUserTurn(thread.value.id, composer.value);
  composer.value = "";
}

function addConstraint(): void {
  const trimmed = constraintDraft.value.trim();
  if (!trimmed) {
    return;
  }

  thread.value.draft.constraints = [...thread.value.draft.constraints, trimmed];
  constraintDraft.value = "";
  conversations.refreshThread(thread.value.id);
}

async function confirmExecution(): Promise<void> {
  if (thread.value.proposal.executionMode !== "now") {
    await router.push(buildProjectTasksRoute(workspaceId.value, projectId.value));
    return;
  }

  try {
    const runDetail = await hub.createAndStartTask({
      workspace_id: workspaceId.value,
      project_id: projectId.value,
      title: thread.value.draft.goal || thread.value.title,
      instruction:
        thread.value.draft.expectedResult ||
        thread.value.messages.at(-1)?.content ||
        thread.value.draft.goal,
      action: {
        kind: "emit_text",
        content:
          thread.value.draft.expectedResult ||
          thread.value.draft.goal ||
          thread.value.title
      },
      capability_id: "capability-write-note",
      estimated_cost: 1,
      idempotency_key: `${workspaceId.value}:${projectId.value}:${thread.value.id}`
    });

    conversations.markExecutionStarted(thread.value.id, runDetail.run.id);
    await router.push(`/runs/${runDetail.run.id}`);
  } catch {
    // The shared shell error state already surfaces execution failures.
  }
}

watch(
  () => [route.params.workspaceId, route.params.projectId],
  () => {
    void loadConversationContext();
  }
);

onMounted(() => {
  void loadConversationContext();
});
</script>

<template>
  <section class="conversation-layout">
    <article class="panel panel-hero">
      <p class="eyebrow">{{ preferences.t("nav.conversation") }}</p>
      <h1 class="page-title">{{ preferences.t("conversation.title") }}</h1>
      <p class="page-subtitle">{{ preferences.t("conversation.subtitle") }}</p>
      <div class="stage-chip">
        <span>{{ preferences.t("conversation.stage") }}</span>
        <strong>{{ thread.stage }}</strong>
      </div>
    </article>

    <div class="conversation-grid">
      <article class="panel">
        <ul v-if="thread.messages.length > 0" class="stack-list">
          <li
            v-for="message in thread.messages"
            :key="message.id"
            class="message-card"
            :class="message.role === 'user' ? 'message-user' : 'message-assistant'"
          >
            <span class="message-role">
              {{
                message.role === "user"
                  ? preferences.t("conversation.roleUser")
                  : preferences.t("conversation.roleSystem")
              }}
            </span>
            <p>{{ message.content }}</p>
          </li>
        </ul>
        <p v-else class="muted-copy">{{ preferences.t("conversation.empty") }}</p>

        <div class="field-stack">
          <textarea
            v-model="composer"
            data-testid="conversation-input"
            rows="4"
            :placeholder="preferences.t('conversation.placeholder')"
          />
          <div class="button-row">
            <button
              class="button-primary"
              data-testid="conversation-send"
              type="button"
              @click="sendTurn"
            >
              {{ preferences.t("conversation.send") }}
            </button>
          </div>
        </div>
      </article>

      <aside class="panel proposal-panel">
        <p class="eyebrow">{{ preferences.t("conversation.proposal") }}</p>
        <label class="field-stack">
          <span>{{ preferences.t("conversation.goal") }}</span>
          <textarea
            v-model="thread.draft.goal"
            rows="3"
            @input="conversations.refreshThread(thread.id)"
          />
        </label>

        <label class="field-stack">
          <span>{{ preferences.t("conversation.expected") }}</span>
          <textarea
            v-model="thread.draft.expectedResult"
            rows="3"
            @input="conversations.refreshThread(thread.id)"
          />
        </label>

        <label class="field-stack">
          <span>{{ preferences.t("conversation.mode") }}</span>
          <select
            v-model="thread.draft.executionMode"
            @change="conversations.refreshThread(thread.id)"
          >
            <option value="now">{{ preferences.t("conversation.executionNow") }}</option>
            <option value="scheduled">
              {{ preferences.t("conversation.executionScheduled") }}
            </option>
            <option value="event-driven">
              {{ preferences.t("conversation.executionEvent") }}
            </option>
          </select>
        </label>

        <div class="field-stack">
          <span>{{ preferences.t("conversation.constraints") }}</span>
          <ul v-if="thread.draft.constraints.length > 0" class="stack-list compact-stack">
            <li
              v-for="constraint in thread.draft.constraints"
              :key="constraint"
              class="tag-pill"
            >
              {{ constraint }}
            </li>
          </ul>
          <div class="inline-field">
            <input
              v-model="constraintDraft"
              type="text"
              :placeholder="preferences.t('conversation.constraints')"
            />
            <button class="button-secondary" type="button" @click="addConstraint">
              +
            </button>
          </div>
        </div>

        <div class="field-stack">
          <span>{{ preferences.t("conversation.risks") }}</span>
          <ul v-if="thread.proposal.riskNotes.length > 0" class="stack-list compact-stack">
            <li v-for="risk in thread.proposal.riskNotes" :key="risk" class="list-card tone-warning">
              {{ risk }}
            </li>
          </ul>
          <p v-else class="muted-copy">{{ preferences.t("common.empty") }}</p>
        </div>

        <div class="stage-chip stage-chip-strong">
          <span>{{ preferences.t("conversation.readiness") }}</span>
          <strong>{{ executionReadinessLabel }}</strong>
        </div>

        <button
          class="button-primary"
          data-testid="proposal-confirm"
          type="button"
          @click="confirmExecution"
        >
          {{
            thread.proposal.executionMode === "now"
              ? preferences.t("conversation.confirm")
              : preferences.t("conversation.openAdvanced")
          }}
        </button>

        <p v-if="thread.lastRunId" class="muted-copy">
          {{ preferences.t("conversation.linkedRun") }}: {{ thread.lastRunId }}
        </p>
      </aside>
    </div>
  </section>
</template>
