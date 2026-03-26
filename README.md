# octopus

`octopus` 是一个面向个人、团队与企业的统一 Agent Runtime Platform。

## 当前仓库状态

当前仓库处于 `doc-first rebuild` 阶段。

- 当前 tracked tree 仅包含根目录文档与 `docs/`。
- 历史运行时骨架，例如 `apps/`、`packages/`、`crates/` 与 workspace manifests，不在当前 tracked tree 中。
- 在未来实际源码进入 tracked tree 之前，不得把产品与架构目标态描述成“当前已实现能力”。

## 正式文档入口

以下文件是当前面向人的正式入口。

### 核心真相文档

- [`docs/PRD.md`](./docs/PRD.md)：产品范围、发版切片、核心对象与验收意图
- [`docs/SAD.md`](./docs/SAD.md)：架构边界、运行模型、治理约束、恢复机制与技术方向
- [`AGENTS.md`](./AGENTS.md)：仓库级 coding agent 协作约束

### 实施与治理规范

- [`docs/ENGINEERING_STANDARD.md`](./docs/ENGINEERING_STANDARD.md)：工程开发规范与实现约束
- [`docs/CODING_STANDARD.md`](./docs/CODING_STANDARD.md)：实现级代码风格、分层模式、状态与 review 规则
- [`docs/AI_ENGINEERING_PLAYBOOK.md`](./docs/AI_ENGINEERING_PLAYBOOK.md)：AI 开发行为手册与停机条件
- [`docs/AI_DEVELOPMENT_PROTOCOL.md`](./docs/AI_DEVELOPMENT_PROTOCOL.md)：AI 每次进入仓库时应遵循的标准开发协议
- [`docs/AI_PROMPT_HANDBOOK.md`](./docs/AI_PROMPT_HANDBOOK.md)：与 AI 协作推进项目时可直接复用的提示词手册
- [`docs/VISUAL_FRAMEWORK.md`](./docs/VISUAL_FRAMEWORK.md)：首版 GA 核心交互面的视觉框架与页面语法
- [`docs/DELIVERY_GOVERNANCE.md`](./docs/DELIVERY_GOVERNANCE.md)：文档类型、ADR/contract/plan/review 流程与 Done 门禁
- [`docs/templates/README.md`](./docs/templates/README.md)：任务切片卡、contract、implementation plan 模板入口

### 建议阅读顺序

如果你是工程师或 agent，建议按以下顺序进入：

1. `README.md`
2. `AGENTS.md`
3. `docs/PRD.md`
4. `docs/SAD.md`
5. 按任务类型继续阅读：
   - 实现与命名规范：`docs/ENGINEERING_STANDARD.md`
   - 代码风格与分层模式：`docs/CODING_STANDARD.md`
   - AI 执行原则：`docs/AI_ENGINEERING_PLAYBOOK.md`
   - AI 固定 SOP：`docs/AI_DEVELOPMENT_PROTOCOL.md`
   - AI 提示词手册：`docs/AI_PROMPT_HANDBOOK.md`
   - GA 交互与视觉：`docs/VISUAL_FRAMEWORK.md`
   - 交付与门禁：`docs/DELIVERY_GOVERNANCE.md`
   - 模板资产：`docs/templates/README.md`

## README 与 AGENTS 的分工

- `README.md` 面向人，说明仓库是什么、当前处于什么状态，以及正式文档在哪里。
- `AGENTS.md` 面向 agent，定义在本仓库中如何工作、如何验证，以及哪些事实不能被夸大或臆造。
- `docs/` 下的补充规范文档负责把产品与架构真相源转换成实现、AI 协作、视觉和交付治理规则。
