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
  loadPersistedRemoteSession?: (
    profile: DesktopConnectionProfile
  ) => Promise<PersistedRemoteSessionLoadResult>;
  savePersistedRemoteSession?: (
    session: PersistedRemoteSession
  ) => Promise<PersistedRemoteSessionOperationResult>;
  clearPersistedRemoteSession?: () => Promise<PersistedRemoteSessionOperationResult>;
}

export interface PersistedRemoteSession {
  baseUrl: string;
  workspaceId: string;
  email: string;
  accessToken: string;
  session: HubSession;
}

export interface PersistedRemoteSessionLoadResult {
  session: PersistedRemoteSession | null;
  storageAvailable: boolean;
  warning?: string;
}

export interface PersistedRemoteSessionOperationResult {
  storageAvailable: boolean;
  warning?: string;
}

const CONNECTION_PROFILE_STORAGE_KEY = "octopus.desktop.connection-profile.v1";
const DEFAULT_REMOTE_BASE_URL = "http://127.0.0.1:4000";
const DEFAULT_SECURE_SESSION_WARNING =
  "Secure session storage is unavailable. Remote sign-in will stay memory-only.";

export const DEFAULT_LOCAL_WORKBENCH_ROUTE = "/workspaces/demo/projects/demo/tasks";

let runtimeOptions: DesktopConnectionRuntimeOptions = {};
const remoteAccessToken = ref<string | null>(null);
const remoteSessionState = ref<HubSession | null>(null);
const secureSessionWarningState = ref<string | null>(null);

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

function toPersistedRemoteSession(
  profile: DesktopConnectionProfile,
  accessToken: string,
  session: HubSession
): PersistedRemoteSession {
  return {
    baseUrl: profile.baseUrl,
    workspaceId: session.workspace_id,
    email: session.email,
    accessToken,
    session
  };
}

function normalizeWarning(value: string | null | undefined): string | null {
  const warning = typeof value === "string" ? value.trim() : "";
  return warning.length > 0 ? warning : null;
}

function warningFromError(error: unknown): string {
  return normalizeWarning(error instanceof Error ? error.message : String(error))
    ?? DEFAULT_SECURE_SESSION_WARNING;
}

function applySecureSessionOperationResult(
  result: PersistedRemoteSessionLoadResult | PersistedRemoteSessionOperationResult | null | undefined
): void {
  if (!result) {
    secureSessionWarningState.value = null;
    return;
  }

  secureSessionWarningState.value =
    normalizeWarning(result.warning) ??
    (result.storageAvailable ? null : DEFAULT_SECURE_SESSION_WARNING);
}

async function loadPersistedRemoteSession(
  profile: DesktopConnectionProfile
): Promise<PersistedRemoteSession | null> {
  const load = runtimeOptions.loadPersistedRemoteSession;
  if (!load) {
    secureSessionWarningState.value = null;
    return null;
  }

  try {
    const result = await load(profile);
    applySecureSessionOperationResult(result);
    return result.session;
  } catch (error) {
    secureSessionWarningState.value = warningFromError(error);
    return null;
  }
}

async function savePersistedRemoteSession(session: PersistedRemoteSession): Promise<void> {
  const save = runtimeOptions.savePersistedRemoteSession;
  if (!save) {
    secureSessionWarningState.value = null;
    return;
  }

  try {
    applySecureSessionOperationResult(await save(session));
  } catch (error) {
    secureSessionWarningState.value = warningFromError(error);
  }
}

async function clearPersistedRemoteSession(): Promise<void> {
  const clear = runtimeOptions.clearPersistedRemoteSession;
  if (!clear) {
    secureSessionWarningState.value = null;
    return;
  }

  try {
    applySecureSessionOperationResult(await clear());
  } catch (error) {
    secureSessionWarningState.value = warningFromError(error);
  }
}

function authStateRequiresReauthentication(error: unknown): boolean {
  return (
    error instanceof HubClientAuthError &&
    (error.authState === "auth_required" || error.authState === "token_expired")
  );
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
  secureSessionWarningState.value = null;
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

export async function initializeDesktopConnection(): Promise<DesktopConnectionProfile> {
  const profile = loadDesktopConnectionProfile();

  if (profile.mode !== "remote") {
    clearRemoteSession();
    secureSessionWarningState.value = null;
    return profile;
  }

  const persistedSession = await loadPersistedRemoteSession(profile);
  if (!persistedSession) {
    clearRemoteSession();
    return loadDesktopConnectionProfile();
  }

  setRemoteSession(persistedSession.accessToken, persistedSession.session);
  const restoredProfile = persistDesktopConnectionProfile({
    ...profile,
    workspaceId: persistedSession.session.workspace_id,
    email: persistedSession.session.email,
    projectId:
      profile.workspaceId === persistedSession.session.workspace_id
        ? profile.projectId
        : undefined
  });

  try {
    const verifiedSession = await createRemoteAuthClientForProfile(restoredProfile).getCurrentSession();
    setRemoteSession(persistedSession.accessToken, verifiedSession);
    const verifiedProfile = persistDesktopConnectionProfile({
      ...restoredProfile,
      workspaceId: verifiedSession.workspace_id,
      email: verifiedSession.email,
      projectId:
        restoredProfile.workspaceId === verifiedSession.workspace_id
          ? restoredProfile.projectId
          : undefined
    });
    await savePersistedRemoteSession(
      toPersistedRemoteSession(verifiedProfile, persistedSession.accessToken, verifiedSession)
    );
    return verifiedProfile;
  } catch (error) {
    if (authStateRequiresReauthentication(error)) {
      clearRemoteSession();
      await clearPersistedRemoteSession();
    }

    return loadDesktopConnectionProfile();
  }
}

export const useConnectionStore = defineStore("connection", () => {
  const profile = ref(loadDesktopConnectionProfile());
  const authLoading = ref(false);
  const authError = ref<string | null>(null);

  const remoteMode = computed(() => profile.value.mode === "remote");
  const session = computed(() => remoteSessionState.value);
  const hasRemoteAccessToken = computed(() => remoteAccessToken.value !== null);
  const sessionActive = computed(() => hasActiveRemoteSession(profile.value));
  const secureSessionWarning = computed(() => secureSessionWarningState.value);

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
    await clearPersistedRemoteSession();
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
      await savePersistedRemoteSession(
        toPersistedRemoteSession(profile.value, response.access_token, response.session)
      );
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
      await savePersistedRemoteSession(
        toPersistedRemoteSession(profile.value, remoteAccessToken.value, nextSession)
      );
      return nextSession;
    } catch (error) {
      if (authStateRequiresReauthentication(error)) {
        clearRemoteSession();
        await clearPersistedRemoteSession();
      }
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
      await clearPersistedRemoteSession();
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
    secureSessionWarning,
    applyProfile,
    login,
    refreshCurrentSession,
    logout,
    clearAuthError,
    rememberProject,
    clearRememberedProject
  };
});
