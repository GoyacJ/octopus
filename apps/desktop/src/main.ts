import "./index.css";
import { createDesktopApp } from "./app";
import {
  configureDesktopConnectionRuntime,
  createConfiguredDesktopHubClient,
  initializeDesktopConnection,
  resolveDesktopEntryRoute
} from "./stores/connection";
import { registerTauriLocalHubTransport } from "./tauri-local-bridge";
import { createTauriRemoteSessionCacheRuntime } from "./tauri-remote-session-cache";

export async function bootstrap() {
  await registerTauriLocalHubTransport();
  configureDesktopConnectionRuntime(createTauriRemoteSessionCacheRuntime());
  await initializeDesktopConnection();

  const { app, router } = createDesktopApp(createConfiguredDesktopHubClient(), false, {
    defaultRoute: resolveDesktopEntryRoute()
  });

  await router.isReady();
  app.mount("#app");
  return { app, router };
}

if (!(globalThis as { __OCTOPUS_DISABLE_AUTO_BOOTSTRAP__?: boolean }).__OCTOPUS_DISABLE_AUTO_BOOTSTRAP__) {
  void bootstrap();
}
