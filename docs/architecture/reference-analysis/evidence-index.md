# Evidence Index

## Hermes Agent

| ID | Topic | Claim | Evidence | Confidence |
|---|---|---|---|---|


## OpenClaw

| ID | Topic | Claim | Evidence | Confidence |
|---|---|---|---|---|
| OC-001 | Project positioning | OpenClaw is a local-first personal assistant; Gateway is the control plane, not the product itself. | `docs/references/openclaw-main/README.md`; `docs/references/openclaw-main/VISION.md`; `docs/references/openclaw-main/docs/gateway/index.md` | High |
| OC-002 | Tech stack | OpenClaw is a Node.js/TypeScript ESM project with Pi agent runtime, WS/HTTP Gateway, TypeBox/Zod schemas, sqlite-vec memory support, and Lit/Vite Control UI. | `docs/references/openclaw-main/package.json`; `docs/references/openclaw-main/docs/web/control-ui.md`; `docs/references/openclaw-main/VISION.md` | High |
| OC-003 | Entrypoints | The CLI starts from `openclaw.mjs`, routes through `src/index.ts` and `src/cli/run-main.ts`; Gateway startup is centered on `startGatewayServer`. | `docs/references/openclaw-main/openclaw.mjs`; `docs/references/openclaw-main/src/index.ts`; `docs/references/openclaw-main/src/cli/run-main.ts`; `docs/references/openclaw-main/src/gateway/server.impl.ts` | High |
| OC-004 | Gateway protocol | Gateway is a single WebSocket control plane and node transport with mandatory `connect`, role/scope claims, device identity, idempotent side-effecting methods, and scope-gated broadcasts. | `docs/references/openclaw-main/docs/concepts/architecture.md`; `docs/references/openclaw-main/docs/gateway/protocol.md`; `docs/references/openclaw-main/src/gateway/server-methods-list.ts` | High |
| OC-005 | Channel adapters | Channel plugins expose a broad `ChannelPlugin` contract; core owns lifecycle/routing/shared message tool while plugins own channel-specific semantics and actions. | `docs/references/openclaw-main/docs/channels/index.md`; `docs/references/openclaw-main/src/channels/plugins/types.plugin.ts`; `docs/references/openclaw-main/src/gateway/server-channels.ts`; `docs/references/openclaw-main/docs/plugins/architecture.md` | High |
| OC-006 | Messaging trust | Unknown DM senders are paired before processing, allowlists are stored locally, group access is separate, and routing back to origin is deterministic. | `docs/references/openclaw-main/README.md`; `docs/references/openclaw-main/docs/channels/pairing.md`; `docs/references/openclaw-main/docs/concepts/session.md`; `docs/references/openclaw-main/docs/channels/channel-routing.md`; `docs/references/openclaw-main/src/security/dm-policy-shared.ts` | High |
| OC-007 | Agent runtime | OpenClaw owns sessions, workspace/bootstrap files, skills, tool wiring, queues, and channel delivery above Pi agent core. | `docs/references/openclaw-main/docs/concepts/agent.md`; `docs/references/openclaw-main/docs/concepts/agent-workspace.md`; `docs/references/openclaw-main/docs/concepts/agent-loop.md`; `docs/references/openclaw-main/docs/concepts/queue.md`; `docs/references/openclaw-main/src/gateway/server-methods/agent.ts`; `docs/references/openclaw-main/src/gateway/server-methods/chat.ts` | High |
| OC-008 | UI and Canvas | Gateway serves Control UI and canvas/A2UI surfaces; WebChat and TUI use Gateway WS, with TUI also offering local embedded mode. | `docs/references/openclaw-main/docs/web/control-ui.md`; `docs/references/openclaw-main/docs/web/webchat.md`; `docs/references/openclaw-main/docs/web/tui.md`; `docs/references/openclaw-main/docs/platforms/mac/canvas.md`; `docs/references/openclaw-main/src/gateway/server-http.ts`; `docs/references/openclaw-main/src/canvas-host/server.ts` | High |
| OC-009 | Tools and plugins | Tools are typed functions, plugins register capabilities through a manifest-first and registry-consumed model, and native plugins run trusted in-process. | `docs/references/openclaw-main/docs/tools/index.md`; `docs/references/openclaw-main/docs/plugins/architecture.md`; `docs/references/openclaw-main/docs/plugins/building-plugins.md`; `docs/references/openclaw-main/src/plugins/registry.ts`; `docs/references/openclaw-main/src/plugins/runtime/types.ts` | High |
| OC-010 | Memory and persona | Memory/persona are primarily workspace files plus plugin-backed retrieval; the built-in memory backend indexes Markdown into a per-agent SQLite database. | `docs/references/openclaw-main/docs/concepts/memory.md`; `docs/references/openclaw-main/docs/concepts/memory-builtin.md`; `docs/references/openclaw-main/docs/concepts/active-memory.md`; `docs/references/openclaw-main/docs/concepts/agent-workspace.md`; `docs/references/openclaw-main/SECURITY.md` | High |
| OC-011 | Security boundary | OpenClaw assumes one trusted operator boundary per Gateway; session keys are routing controls, plugins are TCB, nodes are execution extensions, and inbound content is untrusted. | `docs/references/openclaw-main/SECURITY.md`; `docs/references/openclaw-main/docs/gateway/security/index.md`; `docs/references/openclaw-main/docs/gateway/protocol.md`; `docs/references/openclaw-main/docs/channels/pairing.md` | High |


## Claude Code Sourcemap

| ID | Topic | Claim | Evidence | Confidence |
|---|---|---|---|---|
