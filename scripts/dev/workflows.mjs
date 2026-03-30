import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = resolve(SCRIPT_DIR, "../..");

export const REMOTE_HUB_DEV_HOST = "127.0.0.1";
export const REMOTE_HUB_DEV_PORT = 4000;
export const REMOTE_HUB_DEV_BASE_URL = `http://${REMOTE_HUB_DEV_HOST}:${REMOTE_HUB_DEV_PORT}`;
export const REMOTE_HUB_DEV_DB_RELATIVE_PATH = "target/dev/remote-hub.sqlite";
export const REMOTE_HUB_DEV_WORKSPACE_ID = "workspace-alpha";
export const REMOTE_HUB_DEV_PROJECT_ID = "project-remote-demo";
export const REMOTE_HUB_DEV_BOOTSTRAP_EMAIL = "admin@octopus.local";
export const REMOTE_HUB_DEV_BOOTSTRAP_PASSWORD = "octopus-bootstrap-password";

function toRemoteHubDevEnv(rootDir) {
  return {
    OCTOPUS_REMOTE_HUB_BIND: `${REMOTE_HUB_DEV_HOST}:${REMOTE_HUB_DEV_PORT}`,
    OCTOPUS_REMOTE_HUB_DB: resolve(rootDir, REMOTE_HUB_DEV_DB_RELATIVE_PATH),
    OCTOPUS_REMOTE_HUB_DEV_SEED: "1"
  };
}

export function createRemoteHubDevSpec({ rootDir = REPO_ROOT } = {}) {
  return {
    label: "remote-hub",
    command: "cargo",
    args: ["run", "-p", "octopus-remote-hub"],
    cwd: rootDir,
    env: toRemoteHubDevEnv(rootDir)
  };
}

export function createDesktopLocalDevSpec({ rootDir = REPO_ROOT } = {}) {
  return {
    label: "desktop",
    command: "pnpm",
    args: ["--filter", "@octopus/desktop", "dev"],
    cwd: rootDir,
    env: {}
  };
}

export function formatRemoteManualLoginMessage() {
  return [
    "",
    "remote-hub dev is ready.",
    "Use ConnectionsView with the seeded defaults:",
    `- Mode: remote`,
    `- Base URL: ${REMOTE_HUB_DEV_BASE_URL}`,
    `- Workspace ID: ${REMOTE_HUB_DEV_WORKSPACE_ID}`,
    `- Email: ${REMOTE_HUB_DEV_BOOTSTRAP_EMAIL}`,
    `- Password: ${REMOTE_HUB_DEV_BOOTSTRAP_PASSWORD}`,
    `- Expected project: ${REMOTE_HUB_DEV_PROJECT_ID}`,
    ""
  ].join("\n");
}

export function createDesktopRemoteDevPlan({ rootDir = REPO_ROOT } = {}) {
  return {
    children: [createRemoteHubDevSpec({ rootDir }), createDesktopLocalDevSpec({ rootDir })],
    readyProbeUrl: `${REMOTE_HUB_DEV_BASE_URL}/api/hub/connection`,
    manualLoginMessage: formatRemoteManualLoginMessage()
  };
}
