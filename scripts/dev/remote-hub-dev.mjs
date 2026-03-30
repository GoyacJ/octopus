import { createRemoteHubDevSpec } from "./workflows.mjs";
import { ensureParentDirectory, runProcessGroup } from "./supervisor.mjs";

const spec = createRemoteHubDevSpec();
ensureParentDirectory(spec.env.OCTOPUS_REMOTE_HUB_DB);
process.stdout.write(
  `[remote-hub:dev] using isolated database ${spec.env.OCTOPUS_REMOTE_HUB_DB}\n`
);

const exitCode = await runProcessGroup([spec]);
process.exit(exitCode);
