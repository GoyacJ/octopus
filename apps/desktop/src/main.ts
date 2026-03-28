import { createDesktopApp, createWindowLocalHubClient } from "./app";
import { registerTauriLocalHubTransport } from "./tauri-local-bridge";

export async function bootstrap() {
  await registerTauriLocalHubTransport();

  const { app, router } = createDesktopApp(createWindowLocalHubClient());

  await router.isReady();
  app.mount("#app");
  return { app, router };
}

if (!(globalThis as { __OCTOPUS_DISABLE_AUTO_BOOTSTRAP__?: boolean }).__OCTOPUS_DISABLE_AUTO_BOOTSTRAP__) {
  void bootstrap();
}
