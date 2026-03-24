# Octopus V1 Development Roadmap

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this roadmap incrementally.

**Goal:** 将 `octopus` 从“文档优先 + 根级骨架已初始化”推进到“契约优先、前后端骨架可运行、首条 MVP 纵切片可验证、后续能力可按阶段解锁”的 v1 实施基线。

**Architecture:** 先固化计划与变更跟踪，再建立 `OpenAPI + Protobuf/Buf + Schema` 契约源骨架，随后初始化 Vue 控制面壳、共享包和 Rust workspace 骨架，最后按 `run -> interaction/approval -> resume -> timeline/audit` 单条纵切片推进。所有阶段必须遵守文档同步、状态跟踪、PR 证据和最小验证闭环。

**Tech Stack:** `pnpm` workspaces, `Turborepo`, Vue 3, TypeScript, Vite, Vue Router, Pinia, Vue I18n, UnoCSS, Rust stable, Cargo workspace, OpenAPI, Buf/Protobuf, JSON Schema

---

## 1. Status Model

统一状态字段：

- `Not Started`
- `In Progress`
- `Blocked`
- `Done`

状态更新规则：

1. 阶段开始时，将阶段状态更新为 `In Progress`，并填写 `Last Updated`。
2. 阶段阻塞时，将阶段状态更新为 `Blocked`，补充 `Blocking Reason` 和 `Next Action`。
3. 阶段完成时，将阶段状态更新为 `Done`，补充验证证据与对应阶段变更文档。
4. 未通过退出条件和验证的阶段不得标记为 `Done`。

## 2. Phase Summary

| Phase | Scope | Status | Last Updated | Change Record |
| --- | --- | --- | --- | --- |
| Phase 0 | 计划与变更跟踪基线 | `Done` | `2026-03-24` | `docs/changes/2026-03-24-phase-0-planning-and-tracking.md` |
| Phase 1 | 契约源骨架 | `Done` | `2026-03-24` | `docs/changes/2026-03-24-phase-1-contract-sources.md` |
| Phase 2 | Monorepo 工程脚手架 | `Done` | `2026-03-24` | `docs/changes/2026-03-24-phase-2-monorepo-scaffolding.md` |
| Phase 3 | 首条 MVP 纵切片 | `Done` | `2026-03-24` | `docs/changes/2026-03-24-phase-3-mvp-vertical-slice.md` |
| Phase 4 | v1 能力扩展 | `Not Started` | `-` | `TBD` |
| Phase 5 | Blueprint 与发布硬化 | `Not Started` | `-` | `TBD` |

## 3. Phase Details

### Phase 0: 计划与变更跟踪基线

- Status: `Done`
- Last Updated: `2026-03-24`
- Dependencies: none
- Change Record: `docs/changes/2026-03-24-phase-0-planning-and-tracking.md`

**Objectives**

1. 固化 v1 总路线图。
2. 建立阶段变更记录目录、模板与使用规范。
3. 把阶段状态、证据和文档同步要求写入正式文档，而不是只停留在聊天记录。

**Checklist**

- [x] 产出 v1 总路线图主文档。
- [x] 建立 `docs/changes/` 目录说明和模板。
- [x] 在主计划中定义统一状态字段：`Not Started / In Progress / Blocked / Done`。
- [x] 约定阶段完成后必须同步状态、日期、证据链接和对应变更文档。
- [x] 结合本次实际落地结果将阶段状态切换为 `Done`。

**Exit Criteria**

1. `docs/plans/` 中存在可执行路线图。
2. `docs/changes/` 中存在 `README.md`、`TEMPLATE.md` 和首条阶段记录。
3. `README.md`、hooks、CI 或相关入口能发现新文档基线。

**Verification**

1. `cargo metadata --no-deps --format-version 1`
2. `git diff --stat`

### Phase 1: 契约源骨架

- Status: `Done`
- Last Updated: `2026-03-24`
- Dependencies: Phase 0
- Change Record: `docs/changes/2026-03-24-phase-1-contract-sources.md`

**Objectives**

1. 在 `proto/` 下建立外部 API、内部 RPC 和插件/扩展 schema 的正式契约源。
2. 固化生成、lint、版本和文档同步规则。
3. 为前后端后续骨架开发提供不漂移的契约边界。

**Checklist**

- [x] 在 `proto/openapi/` 建立首批外部 API 契约骨架。
- [x] 在 `proto/grpc/` 建立内部节点协议骨架。
- [x] 在 `proto/schemas/` 建立扩展/插件基础 schema。
- [x] 明确生成代码、lint、版本和文档同步规则。

**Exit Criteria**

1. `proto/openapi/`、`proto/grpc/`、`proto/schemas/` 不再只有占位文件。
2. 契约命名、版本和目录结构与 `README` / `DEVELOPMENT_STANDARDS` 保持一致。
3. 已写清后续生成和 lint 的统一入口。

**Verification**

1. `cargo metadata --no-deps --format-version 1`
2. `git diff -- proto`
3. `buf lint` if available

### Phase 2: Monorepo 工程脚手架

- Status: `Done`
- Last Updated: `2026-03-24`
- Dependencies: Phase 1
- Change Record: `docs/changes/2026-03-24-phase-2-monorepo-scaffolding.md`

**Objectives**

1. 初始化 `apps/web` 最小控制面壳。
2. 初始化共享前端包与统一配置包。
3. 初始化 Rust crates 骨架并纳入 Cargo workspace。

**Checklist**

- [x] 初始化 `apps/web` 最小 Vue 3 + Vite + Router + Pinia + Vue I18n + UnoCSS 壳。
- [x] 初始化 `packages/design-tokens`、`ui`、`icons`、`i18n`、`api-client`、`tsconfig`、`eslint-config`。
- [x] 初始化 Rust crates 的 `Cargo.toml`、模块边界和 workspace 依赖。
- [x] 把各包/应用任务接入 `turbo.json`。

**Exit Criteria**

1. Node workspace 可以安装依赖并执行根级 `turbo run` 任务。
2. Cargo workspace 可解析且包含 crate 成员。
3. Web 壳遵守 tokens、i18n、自建 UI 和信息架构边界。

**Verification**

1. `pnpm install`
2. `pnpm lint`
3. `pnpm typecheck`
4. `pnpm test`
5. `pnpm build`
6. `cargo metadata --no-deps --format-version 1`
7. `cargo fmt --check`
8. `cargo test`

### Phase 3: 首条 MVP 纵切片

- Status: `Done`
- Last Updated: `2026-03-24`
- Dependencies: Phase 2
- Change Record: `docs/changes/2026-03-24-phase-3-mvp-vertical-slice.md`

**Objectives**

1. 统一事件模型落地到最小实现。
2. 打通 `run -> interaction/approval -> resume -> timeline/audit` 闭环。
3. 在控制面提供最小可演示页面与状态流。

**Checklist**

- [x] 后端打通统一事件模型：`Run`、`AskUserRequest`、`ApprovalRequest`、`resume`。
- [x] 建立最小状态投影、审计事件和时间线读取能力。
- [x] Web 控制面落地首批页面：应用壳、`Runs`、`Inbox`、基础 `Audit`。
- [x] 打通 `run -> ask-user/approval -> resume -> timeline/audit` 闭环。

**Exit Criteria**

1. 能创建 run 并进入等待输入或等待审批状态。
2. 能以 `resume_token` 和 `idempotency_key` 恢复。
3. 时间线与审计查询可用，重复恢复可安全去重。

**Verification**

1. `pnpm test`
2. `pnpm build`
3. `cargo test`
4. 手工验证最小闭环和审计回放
5. `pnpm check:generated`
6. `pnpm lint:openapi`

### Phase 4: v1 能力扩展

- Status: `Not Started`
- Last Updated: `-`
- Dependencies: Phase 3
- Change Record: `TBD`

**Checklist**

- [ ] 解锁 `Triggers`、`Nodes`、`Extensions` 的治理壳与最小后端能力。
- [ ] 按 bounded orchestration 落地多 agent 基础协作，不做自由 mesh。
- [ ] 建立 built-in tools、skills、MCP、plugins 的注册与治理最小闭环。

### Phase 5: Blueprint 与发布硬化

- Status: `Not Started`
- Last Updated: `-`
- Dependencies: Phase 4
- Change Record: `TBD`

**Checklist**

- [ ] 落地导入导出最小链路与依赖重绑/预检。
- [ ] 补齐 SQLite / PostgreSQL 双兼容验证。
- [ ] 补齐审计、恢复、幂等、审批、故障分类的测试与文档。
- [ ] 整理发布前验证、PR 证据、截图/录屏和文档同步清单。

## 4. Delivery Rules

1. 每个阶段完成时必须同步更新本路线图和对应 `docs/changes/` 文档。
2. 阶段变更记录必须覆盖 `Summary`、`Scope`、`Risks`、`Verification`、`Docs Sync`、`UI Evidence`、`Review Notes`。
3. 涉及 UI 的阶段必须补齐 light/dark 与 `zh-CN`/`en-US` 四组证据。
4. 涉及契约、tokens、组件 API、数据库策略或运行时边界的变更必须做文档同步；若超出既定边界，再补 ADR。
