# octopus

`octopus` 是一个面向个人、团队和企业的自托管 Agent OS 文档仓库。

当前仓库以文档为中心，目标是先锁定产品范围、架构边界、领域模型、数据模型、API 契约、视觉框架和执行治理，再进入脚手架与实现阶段。

## 当前状态

- 当前跟踪的正式输入主要位于 `docs/` 与 `.github/`。
- 当前正式计划体系采用“两层结构”：一个主计划导航入口，加上 `M0-M10` 对应的里程碑实施计划文件。
- 当前焦点是 `M0 · 文档真相与治理修复`，用于先收敛正式入口、required-doc 集合和会误导 AI 实现的文档表述。
- 当前仓库不应被描述为“已存在完整前后端脚手架”或“已验证通过 pnpm / cargo workspace 构建”。

## 一句话定位

用统一控制面管理 Agent、Team、Discussion、扩展能力与治理策略的自托管 Agent OS。

## 文档导航

以下文档共同构成当前仓库的正式输入：

| 文档 | 作用 |
| --- | --- |
| [docs/PRD.md](docs/PRD.md) | 产品目标、用户场景、管理员能力与用户旅程 |
| [docs/SAD.md](docs/SAD.md) | 架构总览、关键决策与阅读索引 |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 技术架构、运行时、模块职责与目标态蓝图 |
| [docs/DOMAIN.md](docs/DOMAIN.md) | 领域边界、聚合根、值对象、领域服务 |
| [docs/DATA_MODEL.md](docs/DATA_MODEL.md) | 关系型模型、向量存储、索引与迁移策略 |
| [docs/ENGINEERING_STANDARD.md](docs/ENGINEERING_STANDARD.md) | 工程规范、分层、技术基线、交付要求 |
| [docs/API/README.md](docs/API/README.md) | API 导航与跨资源约定 |
| [docs/VIBECODING.md](docs/VIBECODING.md) | AI 主导实现模式下的执行边界 |
| [docs/VISUAL_FRAMEWORK.md](docs/VISUAL_FRAMEWORK.md) | 视觉框架、信息架构、页面优先级 |
| [docs/plans/README.md](docs/plans/README.md) | 实施计划目录规则 |
| [docs/plans/2026-03-25-product-development-master-plan.md](docs/plans/2026-03-25-product-development-master-plan.md) | 当前主计划导航、依赖顺序与里程碑入口 |
| [docs/plans/2026-03-25-m00-doc-truth-and-governance-repair.md](docs/plans/2026-03-25-m00-doc-truth-and-governance-repair.md) | 当前在途实施计划 |
| [docs/changes/README.md](docs/changes/README.md) | 变更记录规则 |
| [docs/changes/2026-03-25-contract-and-repo-baseline.md](docs/changes/2026-03-25-contract-and-repo-baseline.md) | 当前在途变更记录与验证证据 |
| [docs/changes/2026-03-25-planning-governance-unification.md](docs/changes/2026-03-25-planning-governance-unification.md) | 旧治理统一工作的历史记录 |
| [AGENTS.md](AGENTS.md) | 仓库级 AI 协作约束 |

## 执行跟踪

- 主计划负责里程碑顺序、依赖关系、当前焦点和退出条件。
- 里程碑实施计划负责 AI 可直接执行的输入文档、合同冻结项、交付件、验证步骤和文档同步项。
- change 记录负责已经开始执行的里程碑或主题的结果、风险和验证证据。
- `docs/API/MODELS.md` 即使在本地工作区存在，也不自动成为当前正式 required-doc，相关合同要等 `M02` 冻结后再纳入正式基线。

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
10. `docs/plans/2026-03-25-product-development-master-plan.md`
11. `AGENTS.md`

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

`node_modules/`、IDE 配置和本地缓存不属于仓库的正式设计输入。

## 协作约束

- 代码标识、schema 字段、配置键、提交类型和代码注释使用英文。
- 仓库级设计文档默认使用中文。
- 若请求与 `PRD / SAD / ARCHITECTURE / DOMAIN / DATA_MODEL / ENGINEERING_STANDARD / API` 冲突，先指出冲突再继续。
- 不把当前仓库的目标态文档误写成“现状已实现能力”。
- 不在当前 docs-only 状态下声称已完成 `pnpm` / `cargo` 级构建验证。

## 计划治理

- 正式执行顺序只在 [docs/plans/2026-03-25-product-development-master-plan.md](docs/plans/2026-03-25-product-development-master-plan.md) 表达。
- 旧 `A-K` 编号不再作为正式执行入口，仅保留在主计划附录用于历史追踪。
- 变更结果、验证证据和风险回顾以 `docs/changes/` 下的对应记录为准。
- 文档与脚手架仍需保持 `doc-first` 真实状态，不在缺少 manifest、源码或工具时虚构构建和运行结论。
