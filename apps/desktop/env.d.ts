/// <reference types="vite/client" />

import type { LocalHubTransport } from "@octopus/hub-client";

declare module "*.vue" {
  import type { DefineComponent } from "vue";

  const component: DefineComponent<Record<string, never>, Record<string, never>, unknown>;
  export default component;
}

declare global {
  interface Window {
    __OCTOPUS_LOCAL_HUB__?: LocalHubTransport;
  }
}

export {};
