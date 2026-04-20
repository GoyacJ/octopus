# Octopus SDK 重构计划 · 索引

> 本目录（`docs/plans/sdk/`）是 **Octopus Agent Harness SDK 重构**的唯一控制面。
> 规范层真相源在 `docs/sdk/01–14`；本目录将规范落地为**可执行的任务、checklist 与门禁**。
>
> 阅读顺序：`00-overview.md → 01-ai-execution-protocol.md → 02-crate-topology.md → 03-legacy-retirement.md → 04-week-*-*.md`。

## 文档索引

| 文档 | 范围 | 状态 |
|---|---|---|
| `00-overview.md` | 总控：目标、10 项取舍结论、8 周路线、全局 checkpoint 与退出条件 | `draft` |
| `01-ai-execution-protocol.md` | AI 推进规约：checklist 机制、stop conditions、每周门禁、批次报告格式 | `draft` |
| `02-crate-topology.md` | SDK crate 分包、对外 trait/struct 公共面、依赖方向、契约差异清单、UI IR 登记表 | `draft` |
| `03-legacy-retirement.md` | 旧 crate 退役清单（逐文件/逐符号）+ 替代映射 + 守护扫描 | `draft` |
| `04-week-1-contracts-session.md` | W1：`octopus-sdk-contracts` + `octopus-sdk-session` | `draft` |
| `05-week-2-model.md` | W2：`octopus-sdk-model`（Provider / Surface / Model 三层） | `pending` |
| `06-week-3-tools-mcp.md` | W3：`octopus-sdk-tools` + `octopus-sdk-mcp`（删 Capability Planner） | `pending` |
| `07-week-4-permissions-hooks-sandbox-context.md` | W4：权限 / 钩子 / 沙箱 / 上下文工程 | `pending` |
| `08-week-5-subagent-plugin.md` | W5：子代理编排 + 插件体系 | `pending` |
| `09-week-6-core-loop.md` | W6：`octopus-sdk-core`（Brain Loop）整合 + 端到端最小链路 | `pending` |
| `10-week-7-business-cutover.md` | W7：业务侧切换到 SDK + 删除 `octopus-runtime-adapter` | `pending` |
| `11-week-8-cleanup-and-split.md` | W8：文件级拆分、`octopus-persistence` 上线、legacy 文件退场 | `pending` |

> 本索引随周计划产出逐步更新 `状态`。`pending → in_progress → done`。

## 基础规约

- **规范源**：`docs/sdk/01–14`（不改动 `docs/sdk/*` 除非发现与实现冲突）。
- **治理源**：`/AGENTS.md`、`docs/AGENTS.md`、`docs/plans/AGENTS.md`、`docs/api-openapi-governance.md`、`contracts/openapi/AGENTS.md`。
- **本目录本地覆盖**：`docs/plans/sdk/AGENTS.md` 定义命名约定（`NN-<topic>.md` 顺序编号，不使用日期前缀）、状态流转、模板要求与守护扫描。命名违规视为 `01-ai-execution-protocol.md §5` Stop Condition #11。
- **推进模型**：所有子 Plan 使用 `docs/plans/PLAN_TEMPLATE.md`，执行/汇报使用 `docs/plans/EXECUTION_TEMPLATE.md`。
- **checklist 入口**：`01-ai-execution-protocol.md` 定义"任务启动前 / 批次结束后 / 每周门禁"三类 checklist。
- **禁止**：旁路 HTTP API、手改生成物（`contracts/openapi/octopus.openapi.yaml` / `packages/schema/src/generated.ts`）、跳过 schema 生成链条、把业务域塞入 SDK。

## 关键不变量（全周期守护）

1. **Prompt Cache 稳定**：工具顺序、系统 prompt 分段、历史前缀，三者不可在 session 内重排。
2. **凭据零暴露**：SDK 日志/事件流不可序列化明文凭据；`SecretVault` 为唯一出入口。
3. **Config Snapshot**：`start_session()` 必须写入 `config_snapshot_id` + `effective_config_hash` 为事件日志首条记录。
4. **Session 持久化双通道**：SQLite projection + `runtime/events/*.jsonl`；`runtime/sessions/*.json` 退为可选 debug 导出，不作恢复源。
5. **UI 意图 IR**：SDK 通过事件流输出 `RenderBlock` / `AskPrompt` / `ArtifactRef` / `RenderLifecycle`；业务层只消费 IR，不反向约束 SDK。
6. **窄接口**：SDK 对业务仅暴露 4 类 trait：`AgentRuntime` / `SessionStore` / `ModelProvider` / `SecretVault`。

---

**最后更新**：2026-04-20
