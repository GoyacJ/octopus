import { createLocalHubClient, type HubClient } from "@octopus/hub-client";

export function createWindowLocalHubClient(): HubClient {
  const transport = window.__OCTOPUS_LOCAL_HUB__;

  if (!transport) {
    throw new Error(
      "No local Hub transport bridge is registered on window.__OCTOPUS_LOCAL_HUB__."
    );
  }

  return createLocalHubClient(transport);
}
