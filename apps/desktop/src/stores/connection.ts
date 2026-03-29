import {
  HubClientAuthError,
  createRemoteHubAuthClient,
  createRemoteHubClient,
  type HubClient,
  type HubSession,
  type RemoteHubAuthClient,
  type RemoteHubClientOptions
} from "@octopus/hub-client";
import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { createWindowLocalHubClient } from "../hub-client-runtime";
import { configureHubClient, useHubStore } from "./hub";

export type DesktopConnectionMode = "local" | "remote";

export interface DesktopConnectionProfile {
  mode: DesktopConnectionMode;
  baseUrl: string;
  workspaceId: string;
  email: string;
  projectId?: string;
}

export interface DesktopConnectionStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface DesktopConnectionRuntimeOptions {
  storage?: DesktopConnectionStorage;
  createLocalClient?: () => HubClient;
  createRemoteClient?: (options: RemoteHubClientOptions) => HubClient;
  createRemoteAuthClient?: (options: RemoteHubClientOptions) => RemoteHubAuthClient;
}

const CONNECTION_PROFILE_STORAGE_KEY = "octopus.desktop.connection-profile.v1";
const DEFAULT_REMOTE_BASE_URL = "http://127.0.0.1:4000";

export const DEFAULT_LOCAL_WORKBENCH_ROUTE = "/workspaces/demo/projects/demo/tasks";

let runtimeOptions: DesktopConnectionRuntimeOptions = {};
const remoteAccessToken = ref<string | null>(null);
const remoteSessionState = ref<HubSession | null>(null);

function encodePathSegment(value: string): string {
  return encodeURIComponent(value);
}

function buildRemoteClientOptions(
  profile: DesktopConnectionProfile
): RemoteHubClientOptions {
  return {
    baseUrl: profile.baseUrl,
    getAccessToken: () => remoteAccessToken.value
  };
}

function currentStorage(): DesktopConnectionStorage | null {
  if (runtimeOptions.storage) {
    return runtimeOptions.storage;
  }

  if (typeof window === "undefined") {
    return null;
  }

  return window.localStorage;
}

function isConnectionMode(value: unknown): value is DesktopConnectionMode {
  return value === "local" || value === "remote";
}

function normalizeText(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeDesktopConnectionProfile(
  value: Partial<DesktopConnectionProfile> | null | undefined
): DesktopConnectionProfile {
  const projectId = normalizeText(value?.projectId);

  return {
    mode: isConnectionMode(value?.mode) ? value.mode : "local",
    baseUrl: normalizeText(value?.baseUrl) || DEFAULT_REMOTE_BASE_URL,
    workspaceId: normalizeText(value?.workspaceId),
    email: normalizeText(value?.email),
    projectId: projectId || undefined
  };
}

function sessionExpired(session: HubSession): boolean {
  const expiresAt = Date.parse(session.expires_at);
  return Number.isFinite(expiresAt) && expiresAt <= Date.now();
}

function setRemoteSession(accessToken: string, session: HubSession): void {
  remoteAccessToken.value = accessToken;
  remoteSessionState.value = session;
}

function clearRemoteSession(): void {
  remoteAccessToken.value = null;
  remoteSessionState.value = null;
}

function createRemoteAuthClientForProfile(
  profile: DesktopConnectionProfile
): RemoteHubAuthClient {
  const createAuthClient =
    runtimeOptions.createRemoteAuthClient ?? createRemoteHubAuthClient;
  return createAuthClient(buildRemoteClientOptions(profile));
}

function syncHubClient(profile: DesktopConnectionProfile): void {
  configureHubClient(createConfiguredDesktopHubClient(profile));
}

function syncStoredProfile(
  profile: Partial<DesktopConnectionProfile>
): DesktopConnectionProfile {
  const normalized = normalizeDesktopConnectionProfile(profile);
  const storage = currentStorage();
  if (storage) {
    storage.setItem(CONNECTION_PROFILE_STORAGE_KEY, JSON.stringify(normalized));
  }
  return normalized;
}

export function buildWorkspaceInboxRoute(workspaceId: string): string {
  return `/workspaces/${encodePathSegment(workspaceId)}/inbox`;
}

export function buildWorkspaceProjectsRoute(workspaceId: string): string {
  return `/workspaces/${encodePathSegment(workspaceId)}/projects`;
}

export function buildProjectTasksRoute(
  workspaceId: string,
  projectId: string
): string {
  return `/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/tasks`;
}

export function configureDesktopConnectionRuntime(
  options: DesktopConnectionRuntimeOptions
): void {
  runtimeOptions = {
    ...runtimeOptions,
    ...options
  };
}

export function resetDesktopConnectionRuntime(): void {
  runtimeOptions = {};
  clearRemoteSession();
}

export function loadDesktopConnectionProfile(): DesktopConnectionProfile {
  const storage = currentStorage();
  if (!storage) {
    return normalizeDesktopConnectionProfile(null);
  }

  const rawValue = storage.getItem(CONNECTION_PROFILE_STORAGE_KEY);
  if (!rawValue) {
    return normalizeDesktopConnectionProfile(null);
  }

  try {
    return normalizeDesktopConnectionProfile(
      JSON.parse(rawValue) as Partial<DesktopConnectionProfile>
    );
  } catch {
    storage.removeItem(CONNECTION_PROFILE_STORAGE_KEY);
    return normalizeDesktopConnectionProfile(null);
  }
}

export function persistDesktopConnectionProfile(
  profile: Partial<DesktopConnectionProfile>
): DesktopConnectionProfile {
  return syncStoredProfile(profile);
}

export function hasActiveRemoteSession(
  profile: DesktopConnectionProfile = loadDesktopConnectionProfile()
): boolean {
  if (profile.mode !== "remote" || !remoteAccessToken.value || !remoteSessionState.value) {
    return false;
  }

  if (sessionExpired(remoteSessionState.value)) {
    return false;
  }

  return (
    profile.workspaceId.length === 0 ||
    remoteSessionState.value.workspace_id === profile.workspaceId
  );
}

export function resolveDesktopEntryRoute(
  profile: DesktopConnectionProfile = loadDesktopConnectionProfile()
): string {
  if (profile.mode !== "remote") {
    return DEFAULT_LOCAL_WORKBENCH_ROUTE;
  }

  if (!hasActiveRemoteSession(profile)) {
    return "/connections";
  }

  const workspaceId = remoteSessionState.value?.workspace_id || profile.workspaceId;
  const rememberedProjectId = profile.projectId?.trim() ?? "";

  if (rememberedProjectId) {
    return buildProjectTasksRoute(workspaceId, rememberedProjectId);
  }

  return buildWorkspaceProjectsRoute(workspaceId);
}

export function createConfiguredDesktopHubClient(
  profile: DesktopConnectionProfile = loadDesktopConnectionProfile()
): HubClient {
  if (profile.mode === "remote") {
    const createRemoteClient = runtimeOptions.createRemoteClient ?? createRemoteHubClient;
    return createRemoteClient(buildRemoteClientOptions(profile));
  }

  const createLocalClient = runtimeOptions.createLocalClient ?? createWindowLocalHubClient;
  return createLocalClient();
}

export const useConnectionStore = defineStore("connection", () => {
  const profile = ref(loadDesktopConnectionProfile());
  const authLoading = ref(false);
  const authError = ref<string | null>(null);

  const remoteMode = computed(() => profile.value.mode === "remote");
  const session = computed(() => remoteSessionState.value);
  const hasRemoteAccessToken = computed(() => remoteAccessToken.value !== null);
  const sessionActive = computed(() => hasActiveRemoteSession(profile.value));

  function rememberProfile(nextProfile: Partial<DesktopConnectionProfile>): DesktopConnectionProfile {
    const storedProfile = persistDesktopConnectionProfile(nextProfile);
    profile.value = storedProfile;
    return storedProfile;
  }

  function rememberProject(projectId: string): DesktopConnectionProfile {
    return rememberProfile({
      ...profile.value,
      projectId
    });
  }

  function clearRememberedProject(): DesktopConnectionProfile {
    return rememberProject("");
  }

  async function reloadConnectionStatus(): Promise<void> {
    const hub = useHubStore();
    hub.resetWorkbenchState();
    try {
      await hub.loadConnectionStatus();
    } catch {
      // The connection surface and shell already expose the error banner.
    }
  }

  async function applyProfile(nextProfile: Partial<DesktopConnectionProfile>): Promise<void> {
    authError.value = null;
    const normalized = normalizeDesktopConnectionProfile(nextProfile);
    const preserveProjectId =
      profile.value.mode === "remote" &&
      normalized.mode === "remote" &&
      profile.value.baseUrl === normalized.baseUrl &&
      profile.value.workspaceId === normalized.workspaceId;
    const rememberedProjectId = preserveProjectId
      ? normalized.projectId ?? profile.value.projectId
      : normalized.projectId;
    const storedProfile = rememberProfile({
      ...normalized,
      projectId: rememberedProjectId
    });
    clearRemoteSession();
    syncHubClient(storedProfile);
    await reloadConnectionStatus();
  }

  async function login(password: string): Promise<HubSession> {
    if (!remoteMode.value) {
      throw new Error("Remote login requires remote mode.");
    }

    const workspaceId = profile.value.workspaceId.trim();
    const email = profile.value.email.trim();
    if (!workspaceId || !email || !password.trim()) {
      throw new Error("Base URL, workspace, email, and password are required.");
    }

    authLoading.value = true;
    authError.value = null;

    try {
      const response = await createRemoteAuthClientForProfile(profile.value).login({
        workspace_id: workspaceId,
        email,
        password
      });
      setRemoteSession(response.access_token, response.session);
      const rememberedProjectId =
        profile.value.workspaceId === response.session.workspace_id
          ? profile.value.projectId
          : undefined;
      profile.value = rememberProfile({
        ...profile.value,
        mode: "remote",
        workspaceId: response.session.workspace_id,
        email: response.session.email,
        projectId: rememberedProjectId
      });
      syncHubClient(profile.value);
      await reloadConnectionStatus();
      return response.session;
    } catch (error) {
      authError.value = error instanceof Error ? error.message : String(error);
      throw error;
    } finally {
      authLoading.value = false;
    }
  }

  async function refreshCurrentSession(): Promise<HubSession | null> {
    if (!remoteMode.value || !remoteAccessToken.value) {
      remoteSessionState.value = null;
      return null;
    }

    authLoading.value = true;
    authError.value = null;

    try {
      const nextSession = await createRemoteAuthClientForProfile(profile.value).getCurrentSession();
      remoteSessionState.value = nextSession;
      const rememberedProjectId =
        profile.value.workspaceId === nextSession.workspace_id
          ? profile.value.projectId
          : undefined;
      profile.value = rememberProfile({
        ...profile.value,
        workspaceId: nextSession.workspace_id,
        email: nextSession.email,
        projectId: rememberedProjectId
      });
      return nextSession;
    } catch (error) {
      authError.value = error instanceof Error ? error.message : String(error);
      throw error;
    } finally {
      authLoading.value = false;
    }
  }

  async function logout(): Promise<void> {
    authLoading.value = true;
    authError.value = null;

    try {
      if (remoteMode.value && remoteAccessToken.value) {
        try {
          await createRemoteAuthClientForProfile(profile.value).logout();
        } catch (error) {
          if (!(error instanceof HubClientAuthError)) {
            throw error;
          }
        }
      }
      clearRemoteSession();
      syncHubClient(profile.value);
      await reloadConnectionStatus();
    } catch (error) {
      authError.value = error instanceof Error ? error.message : String(error);
      throw error;
    } finally {
      authLoading.value = false;
    }
  }

  function clearAuthError(): void {
    authError.value = null;
  }

  return {
    profile,
    authLoading,
    authError,
    remoteMode,
    session,
    hasRemoteAccessToken,
    sessionActive,
    applyProfile,
    login,
    refreshCurrentSession,
    logout,
    clearAuthError,
    rememberProject,
    clearRememberedProject
  };
});
