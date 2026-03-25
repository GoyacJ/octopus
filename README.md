# octopus

`octopus` 是一个面向个人、团队和企业的自托管 Agent OS 文档仓库。

当前仓库以文档为中心，目标是先锁定产品范围、架构边界、领域模型、数据模型、API 契约和视觉框架，再进入脚手架与实现阶段。

## 当前状态

- 当前跟踪的正式输入主要位于 `docs/` 与 `.github/`。
- 仓库已完成 `PRD / SAD / ARCHITECTURE / DOMAIN / DATA_MODEL / ENGINEERING_STANDARD / API / VISUAL_FRAMEWORK / AGENTS` 这一组文档基线。
- 本轮新增了模型管理方案：`ModelProvider`、`ModelCatalogItem`、`ModelProfile`、`TenantModelPolicy`。
- 当前仓库不应被描述为“已存在完整前后端脚手架”或“已验证通过 pnpm / cargo workspace 构建”。

## 一句话定位

用统一控制面管理 Agent、Team、Discussion、模型档案、扩展能力与治理策略的自托管 Agent OS。

## 模型管理新增范围

本轮文档明确采用“Hub 统一注册 + 模型目录 + 模型档案 + 租户可见性/默认值”的方案：

1. `ModelProvider` 负责 Provider 连接、端点、密钥引用与状态。
2. `ModelCatalogItem` 负责 Provider 下的模型目录。
3. `ModelProfile` 是业务可选的稳定模型档案。
4. `TenantModelPolicy` 控制租户允许使用的模型档案与默认绑定。
5. `Agent` 仅绑定 `model_profile_id`，不再长期持有内联模型配置。

## 文档导航

以下文档共同构成当前仓库的正式输入：

| 文档 | 作用 |
| --- | --- |
| [docs/PRD.md](docs/PRD.md) | 产品目标、用户场景、管理员能力与用户旅程 |
| [docs/SAD.md](docs/SAD.md) | 架构总览、关键决策与阅读索引 |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 技术架构、运行时、模块职责与关键流程 |
| [docs/DOMAIN.md](docs/DOMAIN.md) | 领域边界、聚合根、值对象、领域服务 |
| [docs/DATA_MODEL.md](docs/DATA_MODEL.md) | 关系型模型、向量存储、索引与迁移策略 |
| [docs/ENGINEERING_STANDARD.md](docs/ENGINEERING_STANDARD.md) | 工程规范、分层、技术基线、交付要求 |
| [docs/API/README.md](docs/API/README.md) | API 导航与跨资源约定 |
| [docs/VIBECODING.md](docs/VIBECODING.md) | AI 主导实现模式下的执行边界 |
| [docs/VISUAL_FRAMEWORK.md](docs/VISUAL_FRAMEWORK.md) | 视觉框架、信息架构、页面优先级 |
| [docs/plans/README.md](docs/plans/README.md) | 实施计划目录规则 |
| [docs/changes/README.md](docs/changes/README.md) | 变更记录规则 |
| [docs/plans/2026-03-25-product-development-master-plan.md](docs/plans/2026-03-25-product-development-master-plan.md) | 当前主计划、里程碑状态与 checklist 入口 |
| [docs/changes/2026-03-25-planning-governance-unification.md](docs/changes/2026-03-25-planning-governance-unification.md) | 当前规划治理变更记录、验证证据与风险回顾 |
| [AGENTS.md](AGENTS.md) | 仓库级 AI 协作约束 |

## 执行跟踪

- 任务级完成更新主计划：在 `docs/plans/2026-03-25-product-development-master-plan.md` 中勾选 checklist、更新 `Last Updated`，并补充简短 evidence/note。
- 里程碑或工作流状态更新变更文档：当对应记录进入 `In Progress / Blocked / Done` 时，同步更新相关 `docs/changes/` 文档。
- 当前治理变更入口为 `docs/changes/2026-03-25-planning-governance-unification.md`，后续记录按同一命名规则继续扩展。

建议阅读顺序：

1. `docs/PRD.md`
2. `docs/SAD.md`
3. `docs/ARCHITECTURE.md`
4. `docs/DOMAIN.md`
5. `docs/DATA_MODEL.md`
6. `docs/API/README.md`
7. `docs/ENGINEERING_STANDARD.md`
8. `docs/VIBECODING.md`
9. `docs/VISUAL_FRAMEWORK.md`
10. `AGENTS.md`

## 当前仓库结构

当前仓库应按文档优先的真实状态理解：

```text
octopus/
├─ .github/
├─ docs/
│  ├─ API/
│  ├─ adr/
│  ├─ changes/
│  ├─ plans/
│  ├─ PRD.md
│  ├─ SAD.md
│  ├─ ARCHITECTURE.md
│  ├─ DOMAIN.md
│  ├─ DATA_MODEL.md
│  ├─ ENGINEERING_STANDARD.md
│  ├─ VIBECODING.md
│  └─ VISUAL_FRAMEWORK.md
├─ AGENTS.md
└─ README.md
```

`node_modules/`、IDE 配置或本地缓存不属于仓库的正式设计输入。

## 协作约束

- 代码标识、schema 字段、配置键、提交类型和代码注释使用英文。
- 仓库级设计文档默认使用中文。
- 若请求与 `PRD / SAD / ARCHITECTURE / DOMAIN / DATA_MODEL / ENGINEERING_STANDARD / API` 冲突，先指出冲突再继续。
- 不把当前仓库的目标态文档误写成“现状已实现能力”。
- 不在当前 docs-only 状态下声称已完成 pnpm / cargo 级构建验证。

## 计划治理

- 统一交付顺序、退出条件与执行跟踪以 `docs/plans/2026-03-25-product-development-master-plan.md` 为唯一入口。
- 变更结果、验证证据和风险回顾以 `docs/changes/` 下的对应记录为准。
- 文档与脚手架仍需保持 `doc-first` 真实状态，不在缺少 manifest、源码或工具时虚构构建和运行结论。
