# octopus

`octopus` 是一个面向个人、团队和企业的单租户自托管 Agent OS。

它不是“聊天界面 + 工具调用”的简单封装，而是一个围绕多 agent 编排、节点执行、权限治理、扩展体系、跨端控制面和 blueprint 迁移构建的可治理平台。当前仓库仍处于文档优先阶段，核心目标是先锁定产品边界、架构方向和工程基线，再进入脚手架与代码实现。

## 当前状态

- 当前仓库已完成治理文档固化，并进入基础脚手架初始化阶段。
- 已完成首版产品需求、目标态架构、开发规范、VibeCoding 执行基线、视觉框架和仓库级 AI 协作约束。
- 已初始化 Monorepo 根配置、目录骨架、PR 模板、hooks 和最小 CI 入口，但尚未进入核心业务功能实现阶段。

## 一句话定位

面向个人、团队和企业的自托管 Agent OS，用统一控制平面管理 agent、节点、Built-in Tools、插件、技能、MCP、权限和可移植 blueprint。

## v1 核心能力

`octopus` 首版聚焦以下 6 条能力主线：

1. 多 agent 编排
2. 节点执行
3. 认证、权限与审批
4. 扩展体系
5. 跨端控制面
6. 导入导出

进一步说，v1 重点解决的是：

- 让 agent 成为工作区内长期存在、可治理、可迁移的资产。
- 让多 agent 协作成为运行时能力，而不是 prompt 拼接技巧。
- 让 `Prompt Packs`、`Skills`、`Built-in Tools`、`MCP Servers`、`Plugins` 进入统一治理面。
- 让 `Trigger / Loop / Wait / Ask User / Approval / Resume` 成为正式运行时能力。
- 让平台同时支持个人、团队、企业三种治理强度。

## 首版边界

以下内容不属于 `octopus` v1 正式交付范围：

- 不以 Slack、Telegram、Discord、企业微信等第三方聊天渠道作为首版重点。
- 不做多租户 SaaS 优先控制平面。
- 不把移动端定义为完整重型执行节点。
- 不把 artifact 定义为通用、长生命周期、自持久化应用运行时。
- 不承诺自由 agent mesh、无上限自治协作或无代码工作流搭建器。
- 不导出对话过程、推理过程、执行日志、结果物和凭据。

## 技术基线

当前项目已经锁定以下系统级技术方向：

| 领域 | 基线 |
| --- | --- |
| 后端内核 | Rust Core、Tokio、Axum、Tonic、SQLx、Serde、tracing/OpenTelemetry |
| 前端控制面 | Vue 3、TypeScript、Vite、Vue Router、Pinia、VueUse、UnoCSS、Vue I18n |
| 桌面 / 移动 | Tauri 2、Tauri Mobile |
| UI 体系 | 自建 design tokens、自建组件库、统一主题与国际化 |
| 数据库 | `SQLite` 默认，`PostgreSQL` 为团队/生产推荐 |
| 存储与适配 | 文件系统为默认对象存储基线，`Redis` / `S3` 仅为可选适配层 |
| 协议 | 外部 `HTTPS JSON API + WebSocket/SSE`，内部 `mTLS gRPC`，MCP 支持 `stdio` 与 `Streamable HTTP/SSE` |
| 工程组织 | 单仓 Monorepo，Node 侧 `pnpm workspace + Turborepo`，Rust 侧 Cargo workspace |

## 设计与工程原则

- 平台优先做“治理、隔离、审批、审计、恢复”，而不是先做“更多入口”。
- 硬约束必须由系统实现，不能只依赖 prompt。
- tokens 是唯一视觉事实来源，前端必须支持 `system`、`light`、`dark` 三种主题，以及 `zh-CN`、`en-US` 国际化。
- 后端采用模块化单体 + Ports and Adapters + 事件驱动运行时。
- SQLite 与 PostgreSQL 必须保持核心路径双兼容。
- 所有接口、schema、生成类型和扩展契约必须以正式契约源为准，禁止漂移。

## 文档导航

以下文档是当前仓库的正式输入：

| 文档 | 作用 |
| --- | --- |
| [docs/PRD.md](docs/PRD.md) | 产品目标、范围、核心场景、约束与非目标 |
| [docs/SAD.md](docs/SAD.md) | 目标态软件架构、核心模块、运行时机制、信任边界 |
| [docs/DEVELOPMENT_STANDARDS.md](docs/DEVELOPMENT_STANDARDS.md) | 技术栈、目录结构、设计模式、代码规范、提交规范 |
| [docs/VIBECODING.md](docs/VIBECODING.md) | AI 主导实现模式下的执行规则、边界与错误处置流程 |
| [docs/VISUAL_FRAMEWORK.md](docs/VISUAL_FRAMEWORK.md) | 首版视觉框架、表面职责、导航结构和页面优先级 |
| [AGENTS.md](AGENTS.md) | 面向 Codex/AI agent 的仓库级协作约束 |

建议阅读顺序：

1. `docs/PRD.md`
2. `docs/SAD.md`
3. `docs/DEVELOPMENT_STANDARDS.md`
4. `docs/VIBECODING.md`
5. `docs/VISUAL_FRAMEWORK.md`
6. `AGENTS.md`

## 当前仓库结构基线

当前仓库已经初始化以下目录骨架，后续工程落地必须在此边界内继续演进：

```text
octopus/
├─ apps/
│  ├─ web/
│  ├─ desktop/
│  └─ mobile/
├─ packages/
│  ├─ design-tokens/
│  ├─ ui/
│  ├─ icons/
│  ├─ i18n/
│  ├─ composables/
│  ├─ api-client/
│  ├─ eslint-config/
│  ├─ tsconfig/
│  └─ shared/
├─ crates/
│  ├─ octopus-domain/
│  ├─ octopus-application/
│  ├─ octopus-runtime/
│  ├─ octopus-api-http/
│  ├─ octopus-api-grpc/
│  ├─ octopus-infra-sqlite/
│  ├─ octopus-infra-postgres/
│  ├─ octopus-node-runtime/
│  ├─ octopus-plugin-host/
│  └─ octopus-shared/
├─ proto/
├─ deploy/
├─ scripts/
├─ docs/
└─ README.md
```

## 仓库协作约束

- 代码标识、schema 字段、配置键、提交类型和代码注释使用英文。
- 架构文档、规范文档和仓库级说明默认使用中文。
- 协作流采用 `GitHub Flow + Conventional Commits`。
- 主技术栈、目录边界、契约源、tokens 体系、数据库兼容策略等变更必须先走 ADR 或架构评审。
- 工作区可能存在未提交的用户修改；不得覆盖或回退无关变更。

## 对 AI Agent 的约束

本仓库已经提供根级 [AGENTS.md](AGENTS.md)。任何使用 Codex 或其他仓库内 agent 的协作者，都应先读取该文件，再开始脚手架、代码或架构相关工作。

对 AI 协作者的最低要求：

- 先读 `docs/PRD.md`、`docs/SAD.md`、`docs/DEVELOPMENT_STANDARDS.md`。
- 若请求与现有文档冲突，先指出冲突，再继续。
- 不把当前仓库的“目标态设计”误写成“现状已实现能力”。
- 在验证链路尚未搭起前，不夸大测试、构建和运行状态。

## 下一步

当前最合理的仓库演进顺序是：

1. 在 `proto/` 下建立 OpenAPI / Protobuf / Schema 的首批契约源。
2. 初始化 Vue 控制面壳、design tokens、组件库和 i18n 基线。
3. 初始化 Rust Core、运行时 crate、SQLite/PostgreSQL 适配层和基础观测。
4. 先交付 `run -> interaction/approval -> resume -> timeline/audit` 的最小纵切片。
5. 在首条纵切片稳定后，再解锁 trigger、built-in tools、MCP、plugin 和多 agent 扩展。
