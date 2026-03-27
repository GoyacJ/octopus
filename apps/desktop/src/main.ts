import { createDesktopApp, createWindowLocalHubClient } from "./app";

const { app, router } = createDesktopApp(createWindowLocalHubClient());

void router.isReady().finally(() => {
  app.mount("#app");
});
