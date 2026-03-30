import { defineStore } from "pinia";
import { ref } from "vue";

import { usePreferencesStore } from "./preferences";

export type ConversationStage =
  | "drafting"
  | "proposal_ready"
  | "execution_started"
  | "follow_up";

export type ConversationMessageRole = "assistant" | "user";
export type ConversationExecutionMode = "now" | "scheduled" | "event-driven";

export interface ConversationDraft {
  goal: string;
  constraints: string[];
  expectedResult: string;
  executionMode: ConversationExecutionMode;
}

export interface ExecutionProposal {
  currentGoal: string;
  scopeConstraints: string[];
  expectedResult: string;
  executionMode: ConversationExecutionMode;
  riskNotes: string[];
  executionReadiness: "drafting" | "ready" | "reconfirm_required";
}

export interface ConversationThreadSummary {
  id: string;
  title: string;
  stage: ConversationStage;
  updatedAt: string;
  lastMessage: string;
  whyResume: string;
  lastRunId: string | null;
}

export interface ConversationMessage {
  id: string;
  role: ConversationMessageRole;
  content: string;
  createdAt: string;
}

interface ConversationThread {
  id: string;
  workspaceId: string;
  projectId: string;
  title: string;
  stage: ConversationStage;
  updatedAt: string;
  lastRunId: string | null;
  draft: ConversationDraft;
  proposal: ExecutionProposal;
  messages: ConversationMessage[];
}

const CONVERSATION_STORAGE_KEY = "octopus.desktop.conversations";

function currentStorage(): Storage | null {
  if (typeof window === "undefined") {
    return null;
  }

  return window.localStorage;
}

function nowIsoString(): string {
  return new Date().toISOString();
}

function createId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

function blankDraft(): ConversationDraft {
  return {
    goal: "",
    constraints: [],
    expectedResult: "",
    executionMode: "now"
  };
}

function blankProposal(): ExecutionProposal {
  return {
    currentGoal: "",
    scopeConstraints: [],
    expectedResult: "",
    executionMode: "now",
    riskNotes: [],
    executionReadiness: "drafting"
  };
}

function parseThread(value: unknown): ConversationThread | null {
  if (!value || typeof value !== "object") {
    return null;
  }

  const raw = value as Partial<ConversationThread>;
  if (
    typeof raw.id !== "string" ||
    typeof raw.workspaceId !== "string" ||
    typeof raw.projectId !== "string"
  ) {
    return null;
  }

  return {
    id: raw.id,
    workspaceId: raw.workspaceId,
    projectId: raw.projectId,
    title: typeof raw.title === "string" ? raw.title : "Conversation",
    stage:
      raw.stage === "proposal_ready" ||
      raw.stage === "execution_started" ||
      raw.stage === "follow_up"
        ? raw.stage
        : "drafting",
    updatedAt: typeof raw.updatedAt === "string" ? raw.updatedAt : nowIsoString(),
    lastRunId: typeof raw.lastRunId === "string" ? raw.lastRunId : null,
    draft: {
      ...blankDraft(),
      ...(raw.draft ?? {})
    },
    proposal: {
      ...blankProposal(),
      ...(raw.proposal ?? {})
    },
    messages: Array.isArray(raw.messages)
      ? raw.messages
          .filter(
            (message): message is ConversationMessage =>
              Boolean(message) &&
              typeof message === "object" &&
              typeof (message as ConversationMessage).id === "string" &&
              typeof (message as ConversationMessage).content === "string"
          )
          .map((message) => ({
            ...message,
            role: message.role === "user" ? "user" : "assistant",
            createdAt:
              typeof message.createdAt === "string" ? message.createdAt : nowIsoString()
          }))
      : []
  };
}

function clampTitle(value: string): string {
  const trimmed = value.trim();
  if (trimmed.length <= 60) {
    return trimmed || "Conversation";
  }

  return `${trimmed.slice(0, 57)}...`;
}

export const useConversationStore = defineStore("conversation", () => {
  const initialized = ref(false);
  const threads = ref<ConversationThread[]>([]);

  function persist(): void {
    currentStorage()?.setItem(CONVERSATION_STORAGE_KEY, JSON.stringify(threads.value));
  }

  function initialize(): void {
    if (initialized.value) {
      return;
    }

    const raw = currentStorage()?.getItem(CONVERSATION_STORAGE_KEY);
    if (raw) {
      try {
        const parsed = JSON.parse(raw) as unknown[];
        threads.value = Array.isArray(parsed)
          ? parsed.map(parseThread).filter((thread): thread is ConversationThread => thread !== null)
          : [];
      } catch {
        currentStorage()?.removeItem(CONVERSATION_STORAGE_KEY);
      }
    }

    initialized.value = true;
  }

  function assistantCopyForThread(thread: ConversationThread): string {
    const preferences = usePreferencesStore();
    if (thread.stage === "follow_up") {
      return preferences.t("conversation.systemFollowUp");
    }

    if (!thread.draft.goal) {
      return preferences.t("conversation.systemIntro");
    }

    if (thread.stage === "proposal_ready") {
      return preferences.t("conversation.systemProposalReady");
    }

    return preferences.t("conversation.systemNeedResult");
  }

  function refreshThread(threadId: string): void {
    initialize();
    const thread = threads.value.find((item) => item.id === threadId);
    if (!thread) {
      return;
    }

    thread.title = clampTitle(thread.draft.goal || thread.messages.at(-1)?.content || thread.title);
    thread.proposal = {
      currentGoal: thread.draft.goal,
      scopeConstraints: [...thread.draft.constraints],
      expectedResult: thread.draft.expectedResult || thread.draft.goal,
      executionMode: thread.draft.executionMode,
      riskNotes:
        thread.stage === "follow_up"
          ? ["Follow-up changed the scope and should be reconfirmed before execution continues."]
          : thread.draft.executionMode === "now"
            ? []
            : ["This execution mode continues through the advanced direct-execution surface."],
      executionReadiness:
        thread.stage === "follow_up"
          ? "reconfirm_required"
          : thread.draft.goal && (thread.draft.expectedResult || thread.draft.goal)
            ? "ready"
            : "drafting"
    };

    if (thread.stage !== "execution_started" && thread.stage !== "follow_up") {
      thread.stage =
        thread.proposal.executionReadiness === "ready" ? "proposal_ready" : "drafting";
    }

    thread.updatedAt = nowIsoString();
    persist();
  }

  function createThread(workspaceId: string, projectId: string): ConversationThread {
    const createdAt = nowIsoString();
    const thread: ConversationThread = {
      id: createId("thread"),
      workspaceId,
      projectId,
      title: "Conversation",
      stage: "drafting",
      updatedAt: createdAt,
      lastRunId: null,
      draft: blankDraft(),
      proposal: blankProposal(),
      messages: [
        {
          id: createId("message"),
          role: "assistant",
          content: usePreferencesStore().t("conversation.systemIntro"),
          createdAt
        }
      ]
    };

    threads.value.unshift(thread);
    persist();
    return thread;
  }

  function ensureThread(
    workspaceId: string,
    projectId: string,
    threadId?: string | null
  ): ConversationThread {
    initialize();

    const existing =
      threads.value.find(
        (thread) =>
          thread.id === threadId &&
          thread.workspaceId === workspaceId &&
          thread.projectId === projectId
      ) ??
      threads.value.find(
        (thread) => thread.workspaceId === workspaceId && thread.projectId === projectId
      );

    return existing ?? createThread(workspaceId, projectId);
  }

  function appendUserTurn(threadId: string, content: string): void {
    initialize();
    const thread = threads.value.find((item) => item.id === threadId);
    if (!thread) {
      return;
    }

    const trimmed = content.trim();
    if (!trimmed) {
      return;
    }

    thread.messages.push({
      id: createId("message"),
      role: "user",
      content: trimmed,
      createdAt: nowIsoString()
    });

    if (thread.stage === "execution_started" || thread.stage === "follow_up") {
      thread.stage = "follow_up";
      thread.draft.constraints = [...thread.draft.constraints, trimmed];
    } else if (!thread.draft.goal) {
      thread.draft.goal = trimmed;
      thread.draft.expectedResult = trimmed;
    } else if (!thread.draft.expectedResult) {
      thread.draft.expectedResult = trimmed;
    } else {
      thread.draft.constraints = [...thread.draft.constraints, trimmed];
    }

    refreshThread(threadId);
    thread.messages.push({
      id: createId("message"),
      role: "assistant",
      content: assistantCopyForThread(thread),
      createdAt: nowIsoString()
    });
    persist();
  }

  function createThreadFromPrompt(
    workspaceId: string,
    projectId: string,
    prompt: string
  ): string {
    const thread = createThread(workspaceId, projectId);
    appendUserTurn(thread.id, prompt);
    return thread.id;
  }

  function markExecutionStarted(threadId: string, runId: string): void {
    initialize();
    const thread = threads.value.find((item) => item.id === threadId);
    if (!thread) {
      return;
    }

    thread.stage = "execution_started";
    thread.lastRunId = runId;
    thread.updatedAt = nowIsoString();
    thread.messages.push({
      id: createId("message"),
      role: "assistant",
      content: `Execution started in ${runId}.`,
      createdAt: thread.updatedAt
    });
    refreshThread(threadId);
    persist();
  }

  function threadForProject(
    workspaceId: string,
    projectId: string,
    threadId?: string | null
  ): ConversationThread | null {
    initialize();
    return (
      threads.value.find(
        (thread) =>
          thread.id === threadId &&
          thread.workspaceId === workspaceId &&
          thread.projectId === projectId
      ) ??
      threads.value.find(
        (thread) => thread.workspaceId === workspaceId && thread.projectId === projectId
      ) ??
      null
    );
  }

  function summariesForProject(
    workspaceId: string,
    projectId: string
  ): ConversationThreadSummary[] {
    initialize();

    return threads.value
      .filter(
        (thread) => thread.workspaceId === workspaceId && thread.projectId === projectId
      )
      .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
      .map((thread) => ({
        id: thread.id,
        title: thread.title,
        stage: thread.stage,
        updatedAt: thread.updatedAt,
        lastRunId: thread.lastRunId,
        lastMessage: thread.messages.at(-1)?.content ?? "",
        whyResume:
          thread.stage === "follow_up"
            ? usePreferencesStore().t("dashboard.whyConversationFollowUp")
            : thread.stage === "execution_started"
              ? usePreferencesStore().t("dashboard.whyConversationExecution")
              : usePreferencesStore().t("dashboard.whyConversationReady")
      }));
  }

  return {
    threads,
    initialize,
    ensureThread,
    threadForProject,
    createThreadFromPrompt,
    appendUserTurn,
    refreshThread,
    markExecutionStarted,
    summariesForProject
  };
});
