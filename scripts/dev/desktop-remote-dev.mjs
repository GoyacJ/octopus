import { createDesktopRemoteDevPlan } from "./workflows.mjs";
import {
  ensureParentDirectory,
  runProcessGroup,
  waitForHttpReady
} from "./supervisor.mjs";

const plan = createDesktopRemoteDevPlan();
const remoteHubSpec = plan.children.find((child) => child.label === "remote-hub");
if (remoteHubSpec) {
  ensureParentDirectory(remoteHubSpec.env.OCTOPUS_REMOTE_HUB_DB);
}

const exitCode = await runProcessGroup(plan.children, {
  onReady: async () => {
    await waitForHttpReady(plan.readyProbeUrl);
    process.stdout.write(plan.manualLoginMessage);
  }
});

process.exit(exitCode);
