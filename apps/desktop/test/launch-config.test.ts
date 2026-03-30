import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import packageJson from "../package.json";
import tauriConfig from "../src-tauri/tauri.conf.json";

const rootPackageJson = JSON.parse(
  readFileSync(resolve(process.cwd(), "../../package.json"), "utf8")
) as { scripts?: Record<string, string> };

describe("desktop launch configuration", () => {
  it("loads built frontend assets and wires tauri dev/build commands through the desktop package", () => {
    expect(tauriConfig.build?.frontendDist).toBe("../dist");
    expect(tauriConfig.build?.beforeDevCommand).toBe("pnpm run ui:dev");
    expect(tauriConfig.build?.beforeBuildCommand).toBe("pnpm run ui:build");
    expect(tauriConfig.build?.devUrl).toBe("http://127.0.0.1:5173");

    expect(packageJson.scripts?.["ui:dev"]).toBe(
      "vite --host 127.0.0.1 --port 5173 --strictPort"
    );
    expect(packageJson.scripts?.["ui:build"]).toBe("vite build");
    expect(packageJson.scripts?.tauri).toBe("tauri");
    expect(packageJson.scripts?.dev).toBe("pnpm tauri dev");
    expect(packageJson.scripts?.open).toBe("pnpm build && cargo run --manifest-path src-tauri/Cargo.toml");
  });

  it("exposes explicit root-level local and remote desktop dev commands", () => {
    expect(rootPackageJson.scripts?.["desktop:dev:local"]).toBe("pnpm --filter @octopus/desktop dev");
    expect(rootPackageJson.scripts?.["remote-hub:dev"]).toBe("node scripts/dev/remote-hub-dev.mjs");
    expect(rootPackageJson.scripts?.["desktop:dev:remote"]).toBe(
      "node scripts/dev/desktop-remote-dev.mjs"
    );
    expect(rootPackageJson.scripts?.["desktop:open"]).toBe("pnpm --filter @octopus/desktop open");
    expect(rootPackageJson.scripts?.["remote-hub:start"]).toBe("cargo run -p octopus-remote-hub");
  });
});
