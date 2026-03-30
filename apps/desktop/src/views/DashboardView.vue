<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import {
  buildProjectConversationRoute,
  buildWorkspaceProjectsRoute,
  buildWorkspaceInboxRoute
} from "../stores/connection";
import { useConversationStore } from "../stores/conversation";
import { usePreferencesStore } from "../stores/preferences";
import { useHubStore } from "../stores/hub";
import { useConnectionStore } from "../stores/connection";

const route = useRoute();
const router = useRouter();
const hub = useHubStore();
const preferences = usePreferencesStore();
const conversations = useConversationStore();
const connection = useConnectionStore();

preferences.initialize();
conversations.initialize();

const dashboardPrompt = ref("");

const workspaceId = computed(() => String(route.params.workspaceId));
const projectId = computed(() => String(route.params.projectId));
const conversationSummaries = computed(() =>
  conversations.summariesForProject(workspaceId.value, projectId.value)
);
const riskSignals = computed(() => {
  const signals = [];

  if (hub.authState !== "authenticated") {
    signals.push(preferences.t("shell.readOnly"));
  }
  if (hub.connectionStatus?.state === "disconnected") {
    signals.push(preferences.t("shell.degraded"));
  }

  return [
    ...signals,
    ...hub.notifications.map((notification) => notification.title)
  ];
});

const prioritizedRuns = computed(() =>
  [...hub.runs].sort((left, right) => {
    const priority = (status: string) => {
      switch (status) {
        case "failed":
          return 0;
        case "waiting_approval":
        case "blocked":
          return 1;
        case "running":
          return 2;
        default:
          return 3;
      }
    };

    return priority(left.status) - priority(right.status);
  })
);

async function loadDashboard(): Promise<void> {
  try {
    await Promise.all([
      hub.loadProjectContext(workspaceId.value, projectId.value),
      hub.loadRuns(workspaceId.value, projectId.value),
      hub.loadProjectKnowledge(workspaceId.value, projectId.value),
      hub.loadInboxItems(workspaceId.value),
      hub.loadNotifications(workspaceId.value)
    ]);
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

async function startConversation(): Promise<void> {
  const prompt = dashboardPrompt.value.trim();
  const threadId = prompt
    ? conversations.createThreadFromPrompt(workspaceId.value, projectId.value, prompt)
    : conversations.ensureThread(workspaceId.value, projectId.value).id;
  dashboardPrompt.value = "";
  await router.push({
    path: buildProjectConversationRoute(workspaceId.value, projectId.value),
    query: {
      thread: threadId
    }
  });
}

async function continueConversation(threadId?: string): Promise<void> {
  const ensuredThreadId =
    threadId ??
    conversations.ensureThread(workspaceId.value, projectId.value).id;

  await router.push({
    path: buildProjectConversationRoute(workspaceId.value, projectId.value),
    query: {
      thread: ensuredThreadId
    }
  });
}

async function openRun(runId: string): Promise<void> {
  await router.push(`/runs/${runId}`);
}

async function openInbox(): Promise<void> {
  await router.push(buildWorkspaceInboxRoute(workspaceId.value));
}

watch(
  () => [route.params.workspaceId, route.params.projectId],
  () => {
    void loadDashboard();
  }
);

onMounted(() => {
  void loadDashboard();
});
</script>

<template>
  <section class="page-grid">
    <article class="panel panel-hero">
      <p class="eyebrow">{{ preferences.t("nav.dashboard") }}</p>
      <h1 class="page-title">{{ preferences.t("dashboard.heroTitle") }}</h1>
      <p class="page-subtitle">{{ preferences.t("dashboard.heroSubtitle") }}</p>
      <div class="hero-input-row">
        <textarea
          v-model="dashboardPrompt"
          data-testid="dashboard-prompt"
          class="hero-textarea"
          :placeholder="preferences.t('dashboard.promptPlaceholder')"
          rows="3"
        />
        <div class="button-row">
          <button class="button-primary" type="button" @click="startConversation">
            {{ preferences.t("dashboard.startConversation") }}
          </button>
          <button
            class="button-secondary"
            type="button"
            @click="continueConversation(conversationSummaries[0]?.id)"
          >
            {{ preferences.t("dashboard.continuePrevious") }}
          </button>
        </div>
      </div>
    </article>

    <div class="summary-grid">
      <article class="panel">
        <p class="eyebrow">{{ preferences.t("dashboard.continueWork") }}</p>
        <h2 class="section-title">{{ preferences.t("dashboard.recentConversations") }}</h2>
        <ul v-if="conversationSummaries.length > 0" class="stack-list">
          <li v-for="thread in conversationSummaries.slice(0, 3)" :key="thread.id" class="list-card">
            <button class="text-button" type="button" @click="continueConversation(thread.id)">
              {{ thread.title }}
            </button>
            <p class="muted-copy">{{ thread.whyResume }}</p>
          </li>
        </ul>
        <p v-else class="muted-copy">{{ preferences.t("dashboard.emptyConversations") }}</p>
      </article>

      <article class="panel">
        <p class="eyebrow">{{ preferences.t("dashboard.continueWork") }}</p>
        <h2 class="section-title">{{ preferences.t("dashboard.recentRuns") }}</h2>
        <ul v-if="prioritizedRuns.length > 0" class="stack-list">
          <li v-for="run in prioritizedRuns.slice(0, 3)" :key="run.id" class="list-card">
            <button class="text-button" type="button" @click="openRun(run.id)">
              {{ run.title }}
            </button>
            <p class="muted-copy">
              {{
                run.status === "failed"
                  ? preferences.t("dashboard.whyRunFailed")
                  : run.status === "running"
                    ? preferences.t("dashboard.whyRunRunning")
                    : preferences.t("dashboard.whyRunWaiting")
              }}
            </p>
          </li>
        </ul>
        <p v-else class="muted-copy">{{ preferences.t("dashboard.emptyRuns") }}</p>
      </article>

      <article class="panel">
        <p class="eyebrow">{{ preferences.t("dashboard.continueWork") }}</p>
        <h2 class="section-title">{{ preferences.t("dashboard.pendingItems") }}</h2>
        <ul v-if="hub.inboxItems.length > 0" class="stack-list">
          <li
            v-for="item in hub.inboxItems.slice(0, 3)"
            :key="item.id"
            class="list-card"
          >
            <button class="text-button" type="button" @click="openInbox">
              {{ item.title }}
            </button>
            <p class="muted-copy">{{ preferences.t("dashboard.whyInbox") }}</p>
          </li>
        </ul>
        <p v-else class="muted-copy">{{ preferences.t("dashboard.emptyInbox") }}</p>
      </article>
    </div>

    <div class="summary-grid summary-grid-compact">
      <article class="panel">
        <p class="eyebrow">{{ preferences.t("dashboard.systemSummary") }}</p>
        <div class="metric-grid">
          <div class="metric-card">
            <span class="metric-label">{{ preferences.t("dashboard.summaryRuns") }}</span>
            <strong class="metric-value">
              {{ hub.runs.filter((run) => run.status !== "completed").length }}
            </strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">{{ preferences.t("dashboard.summaryApprovals") }}</span>
            <strong class="metric-value">{{ hub.inboxItems.length }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">{{ preferences.t("dashboard.summaryKnowledge") }}</span>
            <strong class="metric-value">
              {{ hub.projectKnowledgeIndex?.entries.length ?? 0 }}
            </strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">
              {{ preferences.t("dashboard.summaryNotifications") }}
            </span>
            <strong class="metric-value">{{ hub.notifications.length }}</strong>
          </div>
        </div>
      </article>

      <article class="panel">
        <p class="eyebrow">{{ preferences.t("dashboard.risks") }}</p>
        <ul v-if="riskSignals.length > 0" class="stack-list">
          <li v-for="signal in riskSignals" :key="signal" class="list-card tone-warning">
            <p>{{ signal }}</p>
          </li>
        </ul>
        <p v-else class="muted-copy">{{ preferences.t("shell.noReminders") }}</p>
      </article>
    </div>
  </section>
</template>
