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

- [`README.md`](./README.md)：仓库当前状态与文档入口
- [`AGENTS.md`](./AGENTS.md)：仓库级 coding agent 协作约束
- [`docs/PRD.md`](./docs/PRD.md)：产品范围、发版切片、核心对象与验收意图
- [`docs/SAD.md`](./docs/SAD.md)：架构边界、运行模型、治理约束、恢复机制与技术方向

### 建议阅读顺序

如果你是工程师或 agent，建议按以下顺序进入：

1. `README.md`
2. `AGENTS.md`
3. `docs/PRD.md`
4. `docs/SAD.md`

## README 与 AGENTS 的分工

- `README.md` 面向人，说明仓库是什么、当前处于什么状态，以及正式文档在哪里。
- `AGENTS.md` 面向 agent，定义在本仓库中如何工作、如何验证，以及哪些事实不能被夸大或臆造。
- `docs/` 下的补充规范文档负责把产品与架构真相源转换成实现、AI 协作、视觉和交付治理规则。
