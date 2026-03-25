# octopus

`octopus` 是一个面向个人、团队和企业的统一 Agent Runtime Platform 仓库。

当前仓库仍以文档作为治理事实源，但已进入 `Phase 1 skeleton` 阶段：根目录文档、`docs/`、`contracts/`、`.github/` 以及最小 `apps/`、`packages/`、`crates/` 骨架共同构成当前 tracked tree。

## 当前状态

- 当前跟踪树中，最重要的正式事实源是 `README.md`、`AGENTS.md`、`docs/`、`contracts/` 和 `.github/`。
- 当前仓库已经具备最小 `pnpm` + `cargo` workspace、Vue 控制面壳、共享契约包以及 Rust Hub/Server/Desktop adapter skeleton。
- 当前已存在两条最小本地 runtime 纵切片：`task -> waiting_approval -> paused/terminated -> completed` 与 `automation/trigger -> automation|watch run -> latest delivery/run view`，并由 Vue 控制面测试与 Rust HTTP smoke 共同覆盖。
- 当前仓库仍不应被描述为“功能完整平台”或“已完成目标态实现”。
- 当前只对已存在的 `pnpm` / `cargo` 验证链路作真实声明；`turbo`、`buf` 和端到端 UI 验证仍未成立。
- `node_modules/`、IDE 配置和本地缓存不属于仓库正式设计输入。

## 本地开发切片

- `cargo run -p octopus-server` 会在 `127.0.0.1:3000` 启动当前最小 HTTP runtime。
- 当前最小 HTTP runtime 已开放 `/api/v1/runs/task`、`/api/v1/runs/{run_id}`、`/api/v1/runs/{run_id}/resume`、`/api/v1/approvals/{approval_id}/resolve`、`/api/v1/automations` 与 `/api/v1/triggers/deliver`。
- `pnpm --filter @octopus/client dev` 会通过 Vite dev proxy 将同源 `/api/*` 请求转发到本地 runtime。
- 该联调方式只用于本地开发便利，不等同于仓库级 live E2E 能力声明；当前正式验证仍以组件测试、HTTP smoke 和 workspace build/test 为准。

## 正式文档入口

以下文件共同构成当前仓库的正式入口：

| 文件 | 作用 |
| --- | --- |
| [AGENTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/AGENTS.md) | 仓库级 Agent 协作约束与真实验证边界 |
| [docs/PRD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md) | 产品定位、用户模式、能力切片与核心对象 |
| [docs/SAD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md) | 目标态逻辑架构、运行平面、治理模型与恢复机制 |
| [docs/CONTRACTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/CONTRACTS.md) | 核心对象、共享枚举与事件骨架的正式契约冻结说明 |
| [docs/ENGINEERING_STANDARD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/ENGINEERING_STANDARD.md) | 工程实现约束、分层规则、交付与评审基线 |
| [docs/VIBECODING.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VIBECODING.md) | AI 主导实现模式下的执行边界与风控原则 |
| [docs/adr/README.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/adr/README.md) | 架构例外和正式决策的记录入口 |

配套治理入口：

- [.github/workflows/guardrails.yml](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/.github/workflows/guardrails.yml)
- [.github/pull_request_template.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/.github/pull_request_template.md)

配套契约源：

- [contracts/README.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/contracts/README.md)

## 建议阅读顺序

1. [docs/PRD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md)
2. [docs/SAD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md)
3. [docs/CONTRACTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/CONTRACTS.md)
4. [docs/ENGINEERING_STANDARD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/ENGINEERING_STANDARD.md)
5. [docs/VIBECODING.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VIBECODING.md)
6. [docs/adr/README.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/adr/README.md)
7. [AGENTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/AGENTS.md)

## 协作与验证边界

- 代码标识、契约字段、配置键和代码注释使用英文；仓库级设计文档默认使用中文。
- 若请求与 `PRD`、`SAD`、`ENGINEERING_STANDARD`、`VIBECODING` 或 ADR 冲突，先指出冲突，再继续变更。
- 不把目标态对象模型、目录结构、运行机制或治理能力误写成“当前已实现事实”。
- 当前真实可执行的默认验证集合至少包括：文档存在性检查、旧引用扫描、聚焦 diff 复核、`git diff --check`；若改动触达 `contracts/`、`packages/`、`apps/` 或 `crates/`，还应执行 `pnpm run typecheck:web`、`pnpm run test:web`、`cargo test --workspace` 与 `cargo build --workspace`。
