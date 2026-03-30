import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname } from "node:path";

function prefixLines(stream, label, writer) {
  if (!stream) {
    return;
  }

  stream.setEncoding("utf8");
  let buffer = "";
  stream.on("data", (chunk) => {
    buffer += chunk;
    const lines = buffer.split(/\r?\n/);
    buffer = lines.pop() ?? "";
    for (const line of lines) {
      writer(`[${label}] ${line}\n`);
    }
  });
  stream.on("end", () => {
    if (buffer.length > 0) {
      writer(`[${label}] ${buffer}\n`);
    }
  });
}

export function ensureParentDirectory(filePath) {
  mkdirSync(dirname(filePath), { recursive: true });
}

export function spawnManagedChild(spec) {
  const child = spawn(spec.command, spec.args, {
    cwd: spec.cwd,
    env: {
      ...process.env,
      ...spec.env
    },
    stdio: ["inherit", "pipe", "pipe"]
  });

  prefixLines(child.stdout, spec.label, (line) => process.stdout.write(line));
  prefixLines(child.stderr, spec.label, (line) => process.stderr.write(line));

  child.on("error", (error) => {
    process.stderr.write(`[${spec.label}] failed to start: ${error.message}\n`);
  });

  return child;
}

export async function waitForHttpReady(
  url,
  { timeoutMs = 60_000, intervalMs = 500 } = {}
) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      // Keep polling until timeout.
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${url}`);
}

export async function runProcessGroup(specs, { onReady } = {}) {
  const children = specs.map(spawnManagedChild);
  const exitStates = new Map();
  let finalCode = 0;
  let settling = false;
  let settleReason = null;

  const killChildren = (signal) => {
    for (const child of children) {
      if (!child.killed && !exitStates.has(child.pid)) {
        child.kill(signal);
        setTimeout(() => {
          if (!exitStates.has(child.pid)) {
            child.kill("SIGKILL");
          }
        }, 3_000).unref();
      }
    }
  };

  const settle = (code, signal = "SIGTERM") => {
    if (settling) {
      return;
    }
    settling = true;
    finalCode = code;
    settleReason = signal;
    killChildren(signal);
  };

  const signalHandlers = new Map([
    [
      "SIGINT",
      () => {
        settle(0, "SIGINT");
      }
    ],
    [
      "SIGTERM",
      () => {
        settle(0, "SIGTERM");
      }
    ]
  ]);

  for (const [signal, handler] of signalHandlers) {
    process.once(signal, handler);
  }

  if (onReady) {
    void onReady()
      .then(() => undefined)
      .catch((error) => {
        if (!settling) {
          process.stderr.write(`[dev] readiness check failed: ${error.message}\n`);
          settle(1, "SIGTERM");
        }
      });
  }

  const exitCode = await new Promise((resolve) => {
    for (const child of children) {
      child.on("exit", (code, signal) => {
        exitStates.set(child.pid, { code, signal });

        if (!settling) {
          const normalizedCode = code ?? (signal ? 1 : 0);
          settle(normalizedCode, "SIGTERM");
        }

        if (exitStates.size === children.length) {
          resolve(finalCode);
        }
      });
    }
  });

  for (const [signal, handler] of signalHandlers) {
    process.removeListener(signal, handler);
  }

  if (settleReason === "SIGINT") {
    process.stdout.write("[dev] shutting down after SIGINT\n");
  } else if (settleReason === "SIGTERM" && exitCode === 0) {
    process.stdout.write("[dev] shutting down\n");
  }

  return exitCode;
}
