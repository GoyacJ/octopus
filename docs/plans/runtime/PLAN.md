# Phase 4 100% Complete 收口实施计划

## 摘要
本计划按“单一收口”执行，目标是在一次完整实施中把 Phase 4 从“transport/projection 已落地”推进到“runtime-native team/workflow orchestration 已闭环且可判定完成”。  
默认策略已锁定为：**严格移除 legacy debug JSON 与旧 tool-side 主路径依赖**；若保留任何过渡兼容，只允许短期只读兼容，且必须在本计划退出条件内清零。

## 核心实施变更

### 1. 把 team worker 从“投影/回退执行”改成真正的 durable subrun orchestration
- 在 `crates/octopus-runtime-adapter` 内建立 runtime-owned subrun dispatch 模型，subrun 的创建、排队、启动、挂起、恢复、取消、完成全部由 runtime orchestrator 驱动，不再由父 turn 文本或 tool-local 约定驱动。
- 冻结 `TeamManifest` 后生成一次性 dispatch plan，必须显式包含：leader、member、delegation edges、mailbox policy、artifact handoff policy、workflow affordance、worker concurrency ceiling。
- subrun checkpoint 必须保存完整恢复输入，而不是只存 `parentRunId + actorRef`。最少要持久化：`parentRunId`、`delegatedByToolCallId/dispatchKey`、目标 actor、worker 输入内容或 artifact/input ref、workflow node ref、mailbox/handoff refs、manifest snapshot ref、session policy snapshot ref、capability state ref、当前迭代和 pending mediation 状态。
- 删除 subrun 对父 `input.content` 的 fallback 恢复逻辑；恢复只能来自 subrun 自己的 checkpoint / runtime artifact refs / JSONL events / SQLite projection。
- worker suspend/resume/cancel 必须针对单个 subrun 生效，不允许通过重跑 leader turn 间接恢复。

### 2. 把 worker concurrency 从“数量裁剪 + 串行 loop”改成 runtime scheduler
- 引入 runtime-level worker scheduler/queue，按 manifest policy 控制最大并发，而不是 `take(limit)` 后由调用方顺序执行。
- scheduler 需要支持：排队、获取可运行 worker、完成后释放 slot、失败/取消后回收 slot、重启后从 SQLite projection 恢复队列和运行态。
- leader run 与 worker subrun 的 capability mediation、approval、auth、trace、usage accounting 必须继续走同一 runtime trunk；并发调度只改变执行次序，不改变共享 runtime 治理边界。
- mixed-domain actor（coding / non-coding）必须共用这一套 scheduler 与 lineage 模型，不能有单独旁路。

### 3. 把 mailbox / handoff / artifact lineage 从投影改成 runtime first-class state
- runtime-owned mailbox body 与 handoff envelope 落盘到 `runtime/`，SQLite 仅存 summary、ref、hash、state、actor/run lineage；artifact body 继续只放 `data/artifacts`。
- mailbox/handoff 记录必须包含并可查询：`sessionId`、`runId`、`parentRunId`、`delegatedByToolCallId`、`senderActorRef`、`receiverActorRef`、`mailboxRef`、`artifactRefs`、`handoffState`。
- mailbox channel、delivery rule、ack rule、artifact handoff rule 必须来自 manifest policy；删除 `team-mailbox` 之类的硬编码行为。
- leader / workflow engine 收集 worker 输出时，只能通过 mailbox summary + handoff envelope + artifact lineage 读取结果，不再通过“把 worker 文本拼回 prompt”实现。
- mailbox delivery 与 acknowledgment 必须可由 **SQLite projection + JSONL events + runtime artifact refs** 复原和解释。

### 4. 把 workflow/background 从简化 summary 改成 runtime state machine
- workflow runtime 需要有明确 node/step 状态机，而不是只产出 `leader-plan` / `workflow-complete` 两个摘要值。
- `workflowAffordance` 必须编译成 runtime-owned execution input，workflow step 与 delegated worker 使用同一 subrun substrate 和同一 lineage 模型。
- approval/auth pause 必须挂在具体 workflow node 或具体 subrun 上，恢复时只恢复该节点，不得重放整轮 session turn。
- background continuation 必须建立成 runtime-managed continuation path：脱离前台连接后仍能继续，且保留 trace、approval、artifact、mailbox lineage。
- background completion 需要有 declared runtime events 和可查询 projection，而不是仅镜像 run status。

### 5. 收口持久化、恢复和消费者切换
- SQLite projection + `runtime/events/*.jsonl` + runtime checkpoint/mailbox/handoff artifacts 成为唯一恢复链路；删除 debug session JSON 作为恢复依赖。
- 移除 `persist_session()` / `load_runtime_events()` / session delete 中对 legacy debug session/events 文件的正式依赖；如有临时读兼容，必须只读且在本阶段结束前删净。
- `/api/v1/runtime/*` 继续作为唯一公开 runtime surface；OpenAPI、`packages/schema`、桌面端 store、browser/tauri host adapter 全部对齐同一 contract。
- `apps/desktop` 继续只消费 typed runtime contract；允许显示 workflow/mailbox/background 状态，但不得依赖本地 patching、opaque JSON fragment 或 tool-layer 推导。
- fence/remove 旧 tool-side orchestration 主路径，确保 team/workflow/background 的 primary route 只能进入 runtime-native path。

## 公共接口与类型变更
- 保持公开 API 仍只在 `/api/v1/runtime/*`，不新增旁路 API。
- 补全并固定 runtime contract：`RuntimeSubrunSummary`、`RuntimeMailboxSummary`、`RuntimeHandoffSummary`、`RuntimeWorkflowSummary`、`RuntimeWorkflowRunDetail`、`RuntimeBackgroundRunSummary`。
- 在 session/run/detail/event 层补齐 Phase 4 所需字段与一致命名，至少覆盖：
  - session/detail：workflow summary、pending mailbox summary、background run summary、worker/workflow counts
  - run snapshot：worker dispatch summary、workflow run ref、mailbox/handoff refs、background state
  - event taxonomy：`subrun.*`、`workflow.started`、`workflow.step.started`、`workflow.step.completed`、`workflow.completed`、`workflow.failed`
  - workflow/subrun event fields：`sessionId`、`runId`、`parentRunId`、`iteration`、`workflowRunId`、`workflowStepId`、`actorRef`、`toolUseId/dispatchKey`、`outcome`

## 测试与验证计划
- 合同层：执行 `pnpm openapi:bundle`、`pnpm schema:generate`、`pnpm schema:check`，并补强 route/adapters parity，使其不再是 `0 normalized routes` 的空验证。
- Adapter/runtime 单测：覆盖 subrun 创建/恢复/取消、scheduler 并发上限、mailbox ack、handoff lineage、workflow node pause/resume、background continuation、restart recovery。
- 持久化回归：构造“运行中 leader + 多 worker + mailbox/handoff + workflow node pause”的中间态，重启 adapter 后验证全部状态由 SQLite + JSONL + runtime artifacts 恢复，且不依赖任何 debug session JSON。
- Mixed-domain 场景：至少各做一组 coding actor 与 non-coding actor 的 workflow/subrun 编排测试，证明同一 substrate 生效。
- Desktop/server parity：继续运行 runtime adapter、platform、server、desktop store/client tests，并补充一组端到端 fixture，验证桌面端无需本地修补即可展示 team/workflow/mailbox/background 状态。

## 退出条件
只有同时满足以下条件，Phase 4 才能标记为 100% complete：

1. **无 prompt-centric 主路径残留**
- team session 进入 runtime trunk 后，worker 输入、恢复、输出均不再依赖父 prompt fallback。
- 代码中不存在 subrun 从父 `input.content` 回退恢复的主路径逻辑。

2. **worker lifecycle 是真实 subrun state**
- worker 可独立创建、排队、运行、挂起、恢复、取消、完成。
- 恢复针对具体 subrun，不需要重放 leader turn。

3. **并发由 runtime policy enforcement 管理**
- 不再通过 `take(limit)` + 调用方 `for` loop 代表并发控制。
- 重启后仍能恢复排队/运行中的 worker 并继续受 manifest concurrency ceiling 约束。

4. **mailbox / handoff / artifact lineage 可查询、可审计、可回放**
- handoff 规则与 mailbox 行为由 manifest policy 驱动。
- mailbox delivery/ack 与 artifact lineage 可从 SQLite + JSONL + runtime artifacts 完整解释。
- leader/workflow engine 收集 worker 输出不再依赖把文本拼回 prompt。

5. **workflow/background 与 worker 共用同一 durable substrate**
- workflow step 与 worker subrun 共用同一 lineage model。
- approval/auth pause 恢复到具体 workflow node/subrun。
- background continuation 在前台断开后仍能完成，并保留 trace/approval/artifact/mailbox lineage。

6. **恢复链路完全收口**
- 恢复只依赖 SQLite projection、`runtime/events/*.jsonl`、runtime artifacts、`data/artifacts`。
- `runtime/sessions/*.json` 与 `*-events.json` 不再是正式恢复输入，也不再由正式持久化路径写出。

7. **消费者和 contract 完整切换**
- desktop、browser host、server 仅通过同一 typed runtime contract 读取 team/workflow/mailbox/background 状态。
- 无 opaque JSON fragment、本地 patching、tool-layer shape 映射作为必需逻辑。

8. **删除门禁全部通过**
- `rg -n "team_runtime_not_enabled" crates/octopus-runtime-adapter crates/runtime` 无结果。
- 旧 tool-side orchestration 相关符号不再作为 primary runtime route；如仍保留低层 primitive，其调用边界必须明确且不再直接承担 team/workflow orchestration。
- 与 legacy debug session/events 正式持久化相关的读写主路径代码清零。

9. **整体验证通过**
- 通过文档要求的全套命令：`pnpm openapi:bundle`、`pnpm schema:generate`、`pnpm schema:check`、`cargo test -p runtime`、`cargo test -p tools`、`cargo test -p octopus-infra`、`cargo test -p octopus-runtime-adapter`、`cargo test -p octopus-platform`、`cargo test -p octopus-server`、`pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts`。
- 额外新增的 Phase 4 restart/mixed-domain/orchestration cases 全部通过。

## 假设与默认值
- 默认按“严格移除 legacy fallback/旧主路径”执行，不接受长期并存。
- 若某些低层 helper 仍有复用价值，只能保留为 runtime orchestration 之后的内部 primitive，不能再暴露为 team/workflow 的编排主入口。
- 本计划以 Phase 4 文档和仓库治理文档为最终验收标准；任何实现若无法满足上述退出条件，即视为 Phase 4 未完成。
