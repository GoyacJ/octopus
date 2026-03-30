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
  refreshToken: string;
  refreshTokenExpiresAt: string;
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

export type DesktopConnectionStateKind =
  | "authenticated"
  | "auth_required"
  | "token_expired"
  | "restored_but_disconnected"
  | "memory_only_storage";

export interface DesktopConnectionBanner {
  kind: Exclude<DesktopConnectionStateKind, "authenticated">;
  title: string;
  message: string;
  tone: "warning" | "danger";
}

const CONNECTION_PROFILE_STORAGE_KEY = "octopus.desktop.connection-profile.v1";
const DEFAULT_REMOTE_BASE_URL = "http://127.0.0.1:4000";
const DEFAULT_SECURE_SESSION_WARNING =
  "Secure session storage is unavailable. Remote sign-in will stay memory-only.";

export const DEFAULT_LOCAL_WORKBENCH_ROUTE = "/workspaces/demo/projects/demo/tasks";

let runtimeOptions: DesktopConnectionRuntimeOptions = {};
const remoteAccessToken = ref<string | null>(null);
const remoteRefreshToken = ref<string | null>(null);
const remoteRefreshTokenExpiresAt = ref<string | null>(null);
const remoteSessionState = ref<HubSession | null>(null);
const remoteSessionOriginState = ref<"none" | "live" | "restored">("none");
const secureSessionWarningState = ref<string | null>(null);

function encodePathSegment(value: string): string {
  return encodeURIComponent(value);
}

function buildRemoteClientOptions(
  profile: DesktopConnectionProfile
): RemoteHubClientOptions {
  return {
    baseUrl: profile.baseUrl,
    getAccessToken: () => remoteAccessToken.value,
    getRefreshToken: () => remoteRefreshToken.value,
    async onRefreshTokens(response) {
      const storedProfile = persistDesktopConnectionProfile({
        ...loadDesktopConnectionProfile(),
        mode: "remote",
        baseUrl: profile.baseUrl,
        workspaceId: response.session.workspace_id,
        email: response.session.email,
        projectId:
          loadDesktopConnectionProfile().workspaceId === response.session.workspace_id
            ? loadDesktopConnectionProfile().projectId
            : undefined
      });
      setRemoteSession(
        response.access_token,
        response.refresh_token,
        response.refresh_expires_at,
        response.session,
        "live"
      );
      await savePersistedRemoteSession(
        toPersistedRemoteSession(
          storedProfile,
          response.access_token,
          response.refresh_token,
          response.refresh_expires_at,
          response.session
        )
      );
    },
    async clearSessionTokens() {
      clearRemoteSession();
      await clearPersistedRemoteSession();
    }
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

function refreshTokenExpired(expiresAt: string | null | undefined): boolean {
  const parsed = Date.parse(expiresAt ?? "");
  return !Number.isFinite(parsed) || parsed <= Date.now();
}

function hasRefreshRecoveryState(): boolean {
  return (
    Boolean(remoteRefreshToken.value) &&
    !refreshTokenExpired(remoteRefreshTokenExpiresAt.value)
  );
}

function setRemoteSession(
  accessToken: string,
  refreshToken: string,
  refreshTokenExpiresAt: string,
  session: HubSession,
  origin: "live" | "restored" = "live"
): void {
  remoteAccessToken.value = accessToken;
  remoteRefreshToken.value = refreshToken;
  remoteRefreshTokenExpiresAt.value = refreshTokenExpiresAt;
  remoteSessionState.value = session;
  remoteSessionOriginState.value = origin;
}

function clearRemoteSession(): void {
  remoteAccessToken.value = null;
  remoteRefreshToken.value = null;
  remoteRefreshTokenExpiresAt.value = null;
  remoteSessionState.value = null;
  remoteSessionOriginState.value = "none";
}

function toPersistedRemoteSession(
  profile: DesktopConnectionProfile,
  accessToken: string,
  refreshToken: string,
  refreshTokenExpiresAt: string,
  session: HubSession
): PersistedRemoteSession {
  return {
    baseUrl: profile.baseUrl,
    workspaceId: session.workspace_id,
    email: session.email,
    accessToken,
    refreshToken,
    refreshTokenExpiresAt,
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

function rememberableProjectId(
  profile: DesktopConnectionProfile,
  session: HubSession
): string | undefined {
  return profile.workspaceId === session.workspace_id ? profile.projectId : undefined;
}

function profileForSession(
  profile: DesktopConnectionProfile,
  session: HubSession
): DesktopConnectionProfile {
  return normalizeDesktopConnectionProfile({
    ...profile,
    mode: "remote",
    workspaceId: session.workspace_id,
    email: session.email,
    projectId: rememberableProjectId(profile, session)
  });
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
  if (
    profile.mode !== "remote" ||
    !remoteAccessToken.value ||
    !remoteSessionState.value
  ) {
    return false;
  }

  const workspaceMatches =
    profile.workspaceId.length === 0 ||
    remoteSessionState.value.workspace_id === profile.workspaceId;
  if (!workspaceMatches) {
    return false;
  }

  if (!sessionExpired(remoteSessionState.value)) {
    return true;
  }

  return hasRefreshRecoveryState();
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

  setRemoteSession(
    persistedSession.accessToken,
    persistedSession.refreshToken,
    persistedSession.refreshTokenExpiresAt,
    persistedSession.session,
    "restored"
  );
  const restoredProfile = persistDesktopConnectionProfile(
    profileForSession(profile, persistedSession.session)
  );
  const authClient = createRemoteAuthClientForProfile(restoredProfile);

  try {
    if (sessionExpired(persistedSession.session) && hasRefreshRecoveryState()) {
      const refreshed = await authClient.refreshSession();
      setRemoteSession(
        refreshed.access_token,
        refreshed.refresh_token,
        refreshed.refresh_expires_at,
        refreshed.session,
        "live"
      );
      const refreshedProfile = persistDesktopConnectionProfile(
        profileForSession(restoredProfile, refreshed.session)
      );
      await savePersistedRemoteSession(
        toPersistedRemoteSession(
          refreshedProfile,
          refreshed.access_token,
          refreshed.refresh_token,
          refreshed.refresh_expires_at,
          refreshed.session
        )
      );
      return refreshedProfile;
    }

    const verifiedSession = await authClient.getCurrentSession();
    setRemoteSession(
      persistedSession.accessToken,
      persistedSession.refreshToken,
      persistedSession.refreshTokenExpiresAt,
      verifiedSession,
      "live"
    );
    const verifiedProfile = persistDesktopConnectionProfile(
      profileForSession(restoredProfile, verifiedSession)
    );
    await savePersistedRemoteSession(
      toPersistedRemoteSession(
        verifiedProfile,
        persistedSession.accessToken,
        persistedSession.refreshToken,
        persistedSession.refreshTokenExpiresAt,
        verifiedSession
      )
    );
    return verifiedProfile;
  } catch (error) {
    if (authStateRequiresReauthentication(error)) {
      clearRemoteSession();
      await clearPersistedRemoteSession();
      return loadDesktopConnectionProfile();
    }

    return restoredProfile;
  }
}

export const useConnectionStore = defineStore("connection", () => {
  const hub = useHubStore();
  const profile = ref(loadDesktopConnectionProfile());
  const authLoading = ref(false);
  const authError = ref<string | null>(null);

  const remoteMode = computed(() => profile.value.mode === "remote");
  const session = computed(() => remoteSessionState.value);
  const hasRemoteAccessToken = computed(() => remoteAccessToken.value !== null);
  const sessionActive = computed(() => hasActiveRemoteSession(profile.value));
  const sessionOrigin = computed(() => remoteSessionOriginState.value);
  const secureSessionWarning = computed(() => secureSessionWarningState.value);

  const connectionState = computed<DesktopConnectionStateKind>(() => {
    if (!remoteMode.value) {
      return "authenticated";
    }

    if (hub.authState === "token_expired") {
      return "token_expired";
    }

    if (
      session.value &&
      sessionOrigin.value === "restored" &&
      hub.connectionStatus?.state === "disconnected"
    ) {
      return "restored_but_disconnected";
    }

    if (hub.authState === "auth_required") {
      return "auth_required";
    }

    if (secureSessionWarning.value) {
      return "memory_only_storage";
    }

    return "authenticated";
  });

  const connectionBanner = computed<DesktopConnectionBanner | null>(() => {
    switch (connectionState.value) {
      case "restored_but_disconnected":
        return {
          kind: "restored_but_disconnected",
          tone: "warning",
          title: "Remote session restored in degraded mode",
          message:
            "The cached remote session was restored, but the hub is currently disconnected. The workbench stays read-only until connectivity recovers."
        };
      case "memory_only_storage":
        return {
          kind: "memory_only_storage",
          tone: "warning",
          title: "Remote session storage is memory-only",
          message:
            secureSessionWarning.value ??
            DEFAULT_SECURE_SESSION_WARNING
        };
      case "token_expired":
        return {
          kind: "token_expired",
          tone: "danger",
          title: "Remote session expired",
          message: "Sign in again to restore read/write access across the workbench."
        };
      case "auth_required":
        return {
          kind: "auth_required",
          tone: "danger",
          title: "Remote sign-in required",
          message: "This workbench is read-only until the current remote profile is authenticated."
        };
      default:
        return null;
    }
  });

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

  async function refreshConnectionStatus(options: { resetWorkbench?: boolean } = {}): Promise<void> {
    if (options.resetWorkbench) {
      hub.resetWorkbenchState();
    }

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
    await refreshConnectionStatus({
      resetWorkbench: true
    });
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
      setRemoteSession(
        response.access_token,
        response.refresh_token,
        response.refresh_expires_at,
        response.session,
        "live"
      );
      profile.value = rememberProfile(profileForSession(profile.value, response.session));
      await savePersistedRemoteSession(
        toPersistedRemoteSession(
          profile.value,
          response.access_token,
          response.refresh_token,
          response.refresh_expires_at,
          response.session
        )
      );
      syncHubClient(profile.value);
      await refreshConnectionStatus({
        resetWorkbench: true
      });
      return response.session;
    } catch (error) {
      authError.value = error instanceof Error ? error.message : String(error);
      throw error;
    } finally {
      authLoading.value = false;
    }
  }

  async function refreshCurrentSession(): Promise<HubSession | null> {
    if (!remoteMode.value || !remoteAccessToken.value || !remoteSessionState.value) {
      clearRemoteSession();
      return null;
    }

    authLoading.value = true;
    authError.value = null;

    try {
      const authClient = createRemoteAuthClientForProfile(profile.value);
      let nextSession: HubSession;
      let accessToken = remoteAccessToken.value;
      let refreshToken = remoteRefreshToken.value;
      let refreshTokenExpiresAt = remoteRefreshTokenExpiresAt.value;

      if (sessionExpired(remoteSessionState.value)) {
        if (!hasRefreshRecoveryState()) {
          clearRemoteSession();
          await clearPersistedRemoteSession();
          return null;
        }

        const refreshed = await authClient.refreshSession();
        accessToken = refreshed.access_token;
        refreshToken = refreshed.refresh_token;
        refreshTokenExpiresAt = refreshed.refresh_expires_at;
        nextSession = refreshed.session;
      } else {
        nextSession = await authClient.getCurrentSession();
      }

      if (!accessToken || !refreshToken || !refreshTokenExpiresAt) {
        clearRemoteSession();
        await clearPersistedRemoteSession();
        return null;
      }

      setRemoteSession(
        accessToken,
        refreshToken,
        refreshTokenExpiresAt,
        nextSession,
        "live"
      );
      profile.value = rememberProfile(profileForSession(profile.value, nextSession));
      await savePersistedRemoteSession(
        toPersistedRemoteSession(
          profile.value,
          accessToken,
          refreshToken,
          refreshTokenExpiresAt,
          nextSession
        )
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
      await refreshConnectionStatus({
        resetWorkbench: true
      });
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
    sessionOrigin,
    connectionState,
    connectionBanner,
    secureSessionWarning,
    applyProfile,
    login,
    refreshCurrentSession,
    refreshConnectionStatus,
    logout,
    clearAuthError,
    rememberProject,
    clearRememberedProject
  };
});
