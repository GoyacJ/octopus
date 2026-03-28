import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { LocalHubTransport } from "@octopus/hub-client";

export function toTauriInvokeCommand(command: string): string {
  return command.replace(/[^a-zA-Z0-9_]/g, "_");
}

export function createTauriLocalHubTransport(): LocalHubTransport {
  return {
    async invoke(command, payload) {
      const invokeArgs =
        payload === null || payload === undefined
          ? undefined
          : (payload as InvokeArgs);
      return invoke(toTauriInvokeCommand(command), invokeArgs);
    },
    async listen(channel, handler) {
      const unlisten = await listen(channel, (event) => {
        handler(event.payload);
      });

      return async () => {
        unlisten();
      };
    }
  };
}

export async function registerTauriLocalHubTransport(): Promise<LocalHubTransport> {
  if (window.__OCTOPUS_LOCAL_HUB__) {
    return window.__OCTOPUS_LOCAL_HUB__;
  }

  const transport = createTauriLocalHubTransport();
  window.__OCTOPUS_LOCAL_HUB__ = transport;
  return transport;
}

export function clearWindowLocalHubTransport(): void {
  delete window.__OCTOPUS_LOCAL_HUB__;
}
