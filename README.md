# octopus

`octopus` 是一个面向个人、团队和企业的统一 Agent Runtime Platform 仓库。

当前仓库处于 `doc-first rebuild` 阶段：根目录文档、`docs/`、`contracts/` 与 `.github/` 构成当前 tracked tree 的正式事实源。历史实现骨架已删除，不应被描述为当前已实现能力。

## 当前状态

- 当前跟踪树中，最重要的正式事实源是 `README.md`、`AGENTS.md`、`docs/`、`contracts/` 与 `.github/`。
- 当前仓库不包含可执行的 `apps/`、`packages/`、`crates/` 实现骨架，也不包含根级 `package.json`、`pnpm-workspace.yaml`、`Cargo.toml` 等 workspace manifests；任何运行时、API、UI 或测试能力都必须以后续实际引入的 tracked sources 为准。
- 当前重建设计的正式焦点是：`CapabilityCatalog`、`CapabilityResolver`、`ToolSearch`、结构化交互、分层记忆、`ArtifactSessionState` 与 `SkillPack` 注入机制。
- `contracts/v1/` 保存机器可读契约源，`contracts/templates/` 保存 capability card 模板。
- `docs/DEVELOPMENT_PLAN.md` 是仓库级 AI 开发执行主文档，`docs/DEVELOPMENT_CHANGELOG.md` 是累计式变更记录文档。
- 当前仓库仍不应被描述为“功能完整平台”或“已完成目标态实现”。
- 当前只对 tracked tree 实际支持的验证链路作真实声明；若未来重新引入 `pnpm` / `cargo` manifests，方可声明对应构建、测试或运行结果。
- `node_modules/`、IDE 配置和本地缓存不属于仓库正式设计输入。

## 正式文档入口

以下文件共同构成当前仓库的正式入口：

| 文件 | 作用 |
| --- | --- |
| [AGENTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/AGENTS.md) | 仓库级 Agent 协作约束与真实验证边界 |
| [docs/PRD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md) | 产品定位、用户模式、能力切片与核心对象 |
| [docs/SAD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md) | 目标态逻辑架构、运行平面、治理模型与恢复机制 |
| [docs/CONTRACTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/CONTRACTS.md) | 核心对象、共享枚举与事件骨架的正式契约冻结说明 |
| [contracts/README.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/contracts/README.md) | 机器可读契约源与 capability card 模板入口 |
| [docs/DEVELOPMENT_PLAN.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/DEVELOPMENT_PLAN.md) | 仓库级 AI 开发执行主文档，定义阶段、门禁与防跑偏规则 |
| [docs/DEVELOPMENT_CHANGELOG.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/DEVELOPMENT_CHANGELOG.md) | 仓库级累计变更记录，追踪阶段任务、验证与偏离情况 |
| [docs/ENGINEERING_STANDARD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/ENGINEERING_STANDARD.md) | 工程实现约束、分层规则、交付与评审基线 |
| [docs/VIBECODING.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VIBECODING.md) | AI 主导实现模式下的执行边界与风控原则 |
| [docs/adr/README.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/adr/README.md) | 架构例外和正式决策的记录入口 |

配套治理入口：

- [.github/workflows/guardrails.yml](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/.github/workflows/guardrails.yml)
- [.github/pull_request_template.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/.github/pull_request_template.md)

## 建议阅读顺序

1. [docs/PRD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md)
2. [docs/SAD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md)
3. [docs/CONTRACTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/CONTRACTS.md)
4. [contracts/README.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/contracts/README.md)
5. [docs/DEVELOPMENT_PLAN.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/DEVELOPMENT_PLAN.md)
6. [docs/DEVELOPMENT_CHANGELOG.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/DEVELOPMENT_CHANGELOG.md)
7. [docs/ENGINEERING_STANDARD.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/ENGINEERING_STANDARD.md)
8. [docs/VIBECODING.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VIBECODING.md)
9. [docs/adr/README.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/adr/README.md)
10. [AGENTS.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/AGENTS.md)

## 协作与验证边界

- 代码标识、契约字段、配置键和代码注释使用英文；仓库级设计文档默认使用中文。
- 若请求与 `PRD`、`SAD`、`ENGINEERING_STANDARD`、`VIBECODING` 或 ADR 冲突，先指出冲突，再继续变更。
- 不把目标态对象模型、目录结构、运行机制或治理能力误写成“当前已实现事实”。
- 当前真实可执行的默认验证集合至少包括：文档与契约存在性检查、旧引用扫描、聚焦 diff 复核、`git diff --check`；仅当对应 manifests 与源码实际存在时，才应执行 `pnpm run typecheck:web`、`pnpm run test:web`、`cargo test --workspace` 与 `cargo build --workspace`。
