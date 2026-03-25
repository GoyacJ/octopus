# ADR 目录说明

`docs/adr/` 用于记录经批准的架构决策与例外。

## 何时新增 ADR

以下情况必须新增或更新 ADR：

1. 主技术栈调整。
2. Monorepo 目录边界调整。
3. 运行时模式调整。
4. 数据存储策略调整。
5. 组件体系、tokens 体系或契约源调整。
6. 插件、MCP 或宿主扩展机制的重大变化。
7. CapabilityCatalog、ToolSearch、SkillPack 注入机制或结构化交互模型调整。

## 命名规则

采用递增编号：

- `0001-<topic>.md`
- `0002-<topic>.md`

首个文件可从 `0001-` 开始，`0000-template.md` 仅作为模板，不参与编号。编号一旦使用即不回收；若历史 ADR 被删除或归档，后续文件继续递增。

## 当前 ADR

- `0002-capability-runtime-catalog-and-tool-search.md`：能力目录、ToolSearch、结构化交互、ArtifactSessionState 与 SkillPack 的正式架构决策
