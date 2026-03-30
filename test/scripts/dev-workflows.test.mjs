import assert from "node:assert/strict";
import path from "node:path";
import { describe, it } from "node:test";

import {
  REMOTE_HUB_DEV_BASE_URL,
  REMOTE_HUB_DEV_BOOTSTRAP_EMAIL,
  REMOTE_HUB_DEV_BOOTSTRAP_PASSWORD,
  REMOTE_HUB_DEV_DB_RELATIVE_PATH,
  REMOTE_HUB_DEV_PROJECT_ID,
  REMOTE_HUB_DEV_WORKSPACE_ID,
  createDesktopLocalDevSpec,
  createDesktopRemoteDevPlan,
  createRemoteHubDevSpec
} from "../../scripts/dev/workflows.mjs";

describe("desktop dev workflow specs", () => {
  it("builds the remote-hub dev command with isolated database and seed env", () => {
    const rootDir = "/tmp/octopus";
    const spec = createRemoteHubDevSpec({ rootDir });

    assert.equal(spec.label, "remote-hub");
    assert.equal(spec.command, "cargo");
    assert.deepEqual(spec.args, ["run", "-p", "octopus-remote-hub"]);
    assert.equal(spec.env.OCTOPUS_REMOTE_HUB_DEV_SEED, "1");
    assert.equal(
      spec.env.OCTOPUS_REMOTE_HUB_DB,
      path.join(rootDir, REMOTE_HUB_DEV_DB_RELATIVE_PATH)
    );
    assert.equal(spec.env.OCTOPUS_REMOTE_HUB_BIND, "127.0.0.1:4000");
  });

  it("describes the combined remote desktop workflow and manual login guidance", () => {
    const rootDir = "/tmp/octopus";
    const localSpec = createDesktopLocalDevSpec();
    const remotePlan = createDesktopRemoteDevPlan({ rootDir });

    assert.equal(localSpec.label, "desktop");
    assert.equal(localSpec.command, "pnpm");
    assert.deepEqual(localSpec.args, ["--filter", "@octopus/desktop", "dev"]);

    assert.deepEqual(
      remotePlan.children.map((child) => child.label),
      ["remote-hub", "desktop"]
    );
    assert.equal(remotePlan.readyProbeUrl, `${REMOTE_HUB_DEV_BASE_URL}/api/hub/connection`);
    assert.match(remotePlan.manualLoginMessage, new RegExp(REMOTE_HUB_DEV_WORKSPACE_ID));
    assert.match(remotePlan.manualLoginMessage, new RegExp(REMOTE_HUB_DEV_PROJECT_ID));
    assert.match(remotePlan.manualLoginMessage, new RegExp(REMOTE_HUB_DEV_BOOTSTRAP_EMAIL));
    assert.match(remotePlan.manualLoginMessage, new RegExp(REMOTE_HUB_DEV_BOOTSTRAP_PASSWORD));
  });
});
