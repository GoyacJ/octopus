import { invoke } from "@tauri-apps/api/core";

import type {
  DesktopConnectionProfile,
  DesktopConnectionRuntimeOptions,
  PersistedRemoteSession,
  PersistedRemoteSessionLoadResult,
  PersistedRemoteSessionOperationResult
} from "./stores/connection";

export function createTauriRemoteSessionCacheRuntime(): Pick<
  DesktopConnectionRuntimeOptions,
  | "loadPersistedRemoteSession"
  | "savePersistedRemoteSession"
  | "clearPersistedRemoteSession"
> {
  return {
    async loadPersistedRemoteSession(profile: DesktopConnectionProfile) {
      return invoke<PersistedRemoteSessionLoadResult>("load_remote_session_cache", {
        baseUrl: profile.baseUrl,
        workspaceId: profile.workspaceId,
        email: profile.email
      });
    },
    async savePersistedRemoteSession(session: PersistedRemoteSession) {
      return invoke<PersistedRemoteSessionOperationResult>("save_remote_session_cache", {
        session
      });
    },
    async clearPersistedRemoteSession() {
      return invoke<PersistedRemoteSessionOperationResult>("clear_remote_session_cache");
    }
  };
}
