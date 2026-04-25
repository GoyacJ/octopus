# M9 · POC + Acceptance · 三大 POC + 端到端验收

> 状态：待启动 · 依赖：M8 完成
> 关键交付：3 份 POC 报告 + 1 份端到端验收报告 + v1.0 Release Tag
> 预计任务卡：8 张 · 累计工时：AI 12 小时 + 人类评审 16 小时
> 并行度：1（串行；POC 之间有依赖）

---

## 0. 里程碑级注意事项

1. **POC 是最后一道闸门**：架构评审报告（`audit/2026-04-25-architecture-review.md` §4.4）建议的三个高风险设计点必须在此验证
2. **POC 失败的处理**：
   - POC-1（Prompt Cache 命中率）失败 → ADR-003 重设计 → 部分推倒重做
   - POC-2（Steering Queue）失败 → ADR-0017 调整
   - POC-3（Hook 多 transport）失败 → ADR / harness-hook 调整
3. **POC 不是一次性单元测试**：是产线场景下的"假设验证"，必须包含真实环境跑数据
4. **验收报告必须归档**：`docs/architecture/harness/audit/2026-XX-implementation-acceptance.md`

---

## 1. 任务卡总览

| ID | 任务 | 输入 | 输出 |
|---|---|---|---|
| **M9-P01** | POC-1 · Prompt Cache 命中率实测 | M8 完成的 Anthropic provider | 命中率报告 |
| **M9-P02** | POC-2 · Steering Queue 长 turn 语义 | M5 完成的 steering | 行为正确性报告 |
| **M9-P03** | POC-3 · Hook 多 transport 失败模式 | M3 完成的 hook | failure_mode + replay 报告 |
| **M9-T01** | E2E 验收：Desktop 完整闭环 | M8 完成 | 录屏 + 截图 |
| **M9-T02** | E2E 验收：Server 完整闭环 | M8 完成 | API 调用日志 |
| **M9-T03** | E2E 验收：CLI 完整闭环 | M8 完成 | terminal 输出录屏 |
| **M9-T04** | 实施验收报告归档 | T01-T03 + P01-P03 | audit 文档 |
| **M9-T05** | v1.0 Release Tag + 文档发布 | T04 | Git tag + cargo doc |

---

## 2. POC 详情

### M9-P01 · POC-1 · Prompt Cache 命中率实测

**评审报告锚点**：
- `docs/architecture/harness/audit/2026-04-25-architecture-review.md` §4.4 第 1 项

**目标**：验证 ADR-003 的核心假设——Anthropic `system_and_3` cache 在多轮对话 + reload_with(AppliedInPlace, OneShotInvalidation) 场景下确实"一次性代价后续恢复命中"。

**测试场景**：

| 场景 | 输入 | 期望 |
|---|---|---|
| S1 · 基线 | 创建 Session（system 1k tokens + tools 5 个）→ 跑 5 轮对话 | 第 2 轮起 cache_read_tokens > 0；命中率 ≥ 95% |
| S2 · 加工具 | S1 后 reload_with(添加 1 工具) → 再跑 3 轮 | 第 1 轮 cache miss；第 2-3 轮 cache_read_tokens > 0 |
| S3 · 改 system prompt | S1 后 reload_with(改 system prompt) → 再跑 3 轮 | ForkedNewSession 返回；新 Session 第 1 轮 cache miss |
| S4 · 加 Skill | S1 后 reload_with(添加 1 skill) → 再跑 3 轮 | 同 S2（OneShotInvalidation） |

**验收路径分级**（实施前评估 P1-4，对应 Anthropic API 不稳定的 fallback）：

| 级别 | 数据源 | 必跑性 | 通过判据 |
|---|---|:---:|---|
| **L1 · 必跑** | mock SSE 回放 + 内置 cache_read_tokens 字段验证 | 必跑 | 验证 SDK 内部 cache breakpoint 注入 / 字段记账 / reload_with 三档路由正确 |
| **L2 · 应跑** | 真实 Anthropic API + 3 次重试取均值 | 应跑（API 可用时） | S1-S4 按表中"期望"列通过；如 API 限流则记 partial-pass 并转 L3 |
| **L3 · 理想** | 连续 7 天 nightly job 收集 | 理想（不阻塞 M9 Gate） | 趋势报告归档；用于发版后真值校准 |

**预期产物**：
- `crates/octopus-harness-sdk/tests/poc_prompt_cache.rs`：分两组用例
  - `mod l1_mock { ... }`：默认运行，无环境变量要求
  - `mod l2_live { ... }`：`#[ignore] live`，需 `ANTHROPIC_API_KEY`
- `.github/workflows/poc-nightly.yml`：调度 L3 nightly job，结果写入 `audit/2026-XX-poc-prompt-cache-trend.md`
- 报告：`docs/architecture/harness/audit/2026-XX-poc-prompt-cache.md`（标注实测路径："L1 / L2 / L3"）

**通过判据**：
- ✅ **L1 必通过**（M9 Gate 必要条件）：mock 路径下 SDK 内部 cache breakpoint 注入次数、`cache_read_tokens` 字段在 ModelStreamEvent 中正确路由、reload_with 三档全部通过
- ⚠️ **L2 应通过**（M9 Gate 加权条件）：API 可用时 S1 命中率 ≥ 95%；API 限流时记 partial-pass，可仍通过 Gate
- 📊 **L3 持续观察**（不阻塞 M9 Gate）：nightly 趋势报告归档；如连续 3 天命中率 < 90% 触发 hot-fix 卡

**失败处理**：
- L1 失败 → SDK 实现缺陷，必修；可能是 reload_with 路由 bug 或 cache_read_tokens 解析 bug
- L2 失败但 L1 通过 → 重试 3 次后仍失败再判：若多模型多账号都失败 → ADR-003 假设不成立，回到架构层；若仅特定账号失败 → 转 nightly 长期观察
- L2 第 2-3 轮仍 miss → InlineReinjectionBackend 可能没有按预期工作 → tool-search crate 修订

**预期工时**：L1 2 小时；L2 2 小时；L3 持续（不计本卡工时）

---

### M9-P02 · POC-2 · Steering Queue 长 turn 语义

**评审报告锚点**：第 2 项

**目标**：验证 ADR-0017 在长 turn 场景（10+ tool calls）中 SteeringQueue.drain_and_merge 不破坏 prompt cache 的语义。

**测试场景**：

| 场景 | 输入 | 期望 |
|---|---|---|
| L1 · 基线长 turn | 触发一个需要 ≥ 10 工具调用的任务 | 完整跑完 10+ tool calls，无 panic |
| L2 · turn 中 push_steering | L1 跑到第 5 个 tool call 时 push_steering（"用户说请关注 X 部分"）| Steering 在下个 safe checkpoint drain；不污染 prompt cache |
| L3 · 多次 push_steering | L1 跑到第 5/7/9 push 三次（capacity=8）| 全部正确合并；无 DropOldest 触发 |
| L4 · 超 capacity | push_steering 9 次 → 超 capacity=8 | 第 9 次 DropOldest 触发；发出 SteeringDropped 事件 |
| L5 · TTL 过期 | push 1 次 → 等 TTL+1s | 该消息 expire；下次 drain 跳过 |

**预期产物**：
- `crates/octopus-harness-sdk/tests/poc_steering.rs`（mock LLM 即可，无需真实 API）
- 报告：`docs/architecture/harness/audit/2026-XX-poc-steering.md`

**通过判据**：
- ✅ L1-L5 全部按期望行为
- ✅ Prompt cache 命中率：L2 与无 steering 对照组相比下降 ≤ 5%（drain_and_merge 在 safe checkpoint 不破坏 cache）

**失败处理**：
- 若 L2 cache 下降 > 10% → drain_and_merge 时机有问题 → harness-engine 修订
- 若 L4 没触发 DropOldest → SteeringQueue 实现 bug

**预期工时**：3 小时

---

### M9-P03 · POC-3 · Hook 多 transport 失败模式 + replay 幂等

**评审报告锚点**：第 3 项

**目标**：验证三种 transport 在故障场景下的真实行为，确保 v1.8.1 P0-1（FailOpen）+ P1-4（事务语义）落地正确。

**测试场景**：

| Transport | 场景 | 期望 |
|---|---|---|
| **In-process** | hook handler panic | failure_mode=FailOpen → 主流程继续；HookFailedEvent 发出 |
| **In-process** | hook handler panic | failure_mode=FailClosed → 主流程拒绝；输入未改写 |
| **Exec** | 子进程 exit code 非 0 | failure_mode=FailOpen → 主流程继续 |
| **Exec** | 子进程超时 | 信号策略生效；HookTimeoutEvent 发出 |
| **HTTP** | endpoint 5xx | retry policy 生效；最终 fail-* |
| **HTTP** | endpoint 返回 SSRF 风险 IP | SSRF guard 拦截 |
| **HTTP** | endpoint mTLS 失败 | 客户端拒绝 |
| **All** | replay 同一段事件 → hook 调用次数 | 与首次一致（幂等）|

**预期产物**：
- `crates/octopus-harness-sdk/tests/poc_hook_transports.rs`
- 报告：`docs/architecture/harness/audit/2026-XX-poc-hook-transports.md`

**通过判据**：
- ✅ 8 个场景全部按期望行为
- ✅ Replay 幂等：同一段 events replay 时 hook 调用次数 = 首次

**失败处理**：
- 任一场景失败 → 对应 transport / failure_mode 实现修订

**预期工时**：4 小时

---

## 3. E2E 验收

### M9-T01 · Desktop E2E 录屏

**目标**：用户视角验证完整闭环。

**操作流程**：
1. `pnpm tauri dev` 启动 Octopus Desktop
2. 创建工作空间 / 选择项目
3. 提出需求："请扫描当前目录，找出最大的 5 个文件并解释为什么大"
4. 应用调用 ListDir / Read / Bash 工具（dialog 弹出审批 → 用户允许）
5. 流式输出 LLM 回答
6. Memory 落到 `data/memdir/projects/<id>.md`
7. 关闭应用

**预期产物**：
- 录屏（≤ 2min）：`docs/plans/harness-sdk/audit/M9-desktop-e2e.mp4`
- 截图：每步关键截图

**通过判据**：
- ✅ 全程无报错
- ✅ 流式响应可见
- ✅ 权限审批正确出现并响应
- ✅ Memory 被正确写入并下次会话自动读取

---

### M9-T02 · Server E2E API 调用

**目标**：用 curl / Postman 串完整 HTTP 调用链。

**操作流程**：
- POST /api/v1/sessions（创建 session）
- POST /api/v1/sessions/:id/runs（启动 run）
- GET  /api/v1/sessions/:id/events（SSE 订阅事件流）
- POST /api/v1/permissions/:request_id（业务侧审批）
- GET  /api/v1/sessions/:id/snapshot

**预期产物**：
- HTTP 调用日志：`docs/plans/harness-sdk/audit/M9-server-e2e.log`

**通过判据**：
- ✅ 全部 endpoint 200 OK
- ✅ SSE 流式推送正常
- ✅ 审批接口正确

---

### M9-T03 · CLI E2E

**预期产物**：
- terminal 录屏：`docs/plans/harness-sdk/audit/M9-cli-e2e.cast`（asciinema）

**通过判据**：
- ✅ CLI 启动 → 提问 → 完整收到流式响应

---

## 4. 实施验收报告

### M9-T04 · 验收报告归档

**预期产物**：
- `docs/architecture/harness/audit/2026-XX-implementation-acceptance.md`
- 内容包括：
  - SPEC 实施完整度（19 crate × 完成度百分比）
  - 5 道质量闸门历史通过率
  - feature 矩阵覆盖率
  - 3 POC 结论
  - 3 E2E 结论
  - 与原架构评审报告（v1.8.1）P0/P1/P2 修订对应实施情况
  - 已知缺陷登记
  - 后续 v1.1 计划

**通过判据**：
- ✅ 报告由架构 reviewer + 业务 reviewer 双签
- ✅ 已知缺陷全部登记到 issue tracker

---

### M9-T05 · v1.0 Release

**预期产物**：
- Git tag：`v1.0.0-octopus-harness-sdk`
- `cargo doc --no-deps --workspace`：发布到内部 doc 站
- CHANGELOG 更新：`docs/architecture/harness/CHANGELOG.md` 增加 v1.0 实施完成条目
- README 更新：进度块标记全部"已完成"

**通过判据**：
- ✅ M9-T04 审计报告已归档
- ✅ 架构 reviewer 签字
- ✅ 业务方（server / desktop / cli 三业务 TL）签字
- ✅ Git tag 推送

---

## 5. POC 失败影响矩阵（实施前评估 P2-5）

每个 POC 失败时受影响的回滚边界与最小代价：

| POC | 失败模式 | 受影响 crate | 最小回滚边界 | 重做工作量估算 |
|---|---|---|---|---|
| **P01** L1 失败 | SDK reload_with 路由 / cache_read_tokens 解析 bug | `harness-session`（reload）+ `harness-model`（anthropic 解析）| M5-T11（主循环 turn 编排）回到 in-progress | 1-2 工作日（局部修补，不动 ADR）|
| **P01** L2 失败 | 真实 API 命中率 < 90% | 同上 + `harness-tool-search/inline backend` + `harness-context/stages` | 回到 M3 Gate（重审 ContextEngine 5 阶段稳定性）| 5-7 工作日 |
| **P01** L2/L3 持续失败 | ADR-003 假设不成立 | 全栈 | 回到架构层重设计 ADR-003，可能影响 ADR-009/0017 | 2-4 周（含 ADR 重审）|
| **P02** L1-L4 失败 | SteeringQueue capacity / TTL / DropOldest bug | `harness-session/steering`（M3-T19）| 任务卡 M3-T19 reset 重派 | 1 工作日 |
| **P02** L2 cache 下降 > 10% | drain_and_merge 时机错误 | `harness-engine/turn`（M5-T11）+ `harness-session/steering` | 任务卡 M5-T11 + M3-T19 双 reset | 3-5 工作日 |
| **P02** 全失败 | ADR-0017 设计不成立 | 全栈 | 回到 ADR-0017 重审；可能引入 ADR-0019（替代方案）| 2-3 周 |
| **P03** in-process panic 路径错 | failure_mode 路由 bug | `harness-hook/dispatcher`（M3-T07）| M3-T07 reset | 1 工作日 |
| **P03** Exec / HTTP 实现 bug | transport 实现错 | `harness-hook/transport/{exec,http}`（M3-T09）| M3-T09 reset | 1-2 工作日 |
| **P03** SSRF guard / mTLS 失效 | 安全实现错 | `harness-hook/transport/http` | M3-T09 局部修补 | 1 工作日 |
| **P03** Replay 不幂等 | EventStore 装配 / Hook side-effect 失控 | `harness-journal/store` + `harness-hook` | M2-T06 / M2-T07-08 + M3-T07 全栈 reset | 3-5 工作日 |

**通用回滚原则**：
1. POC 失败首选**任务卡 reset**（铁律 3）而非"打补丁";
2. 回滚边界尽量限制在单 crate / 单任务卡;
3. 跨多 crate / 跨多任务卡的失败 → 必须开 retro 任务卡，由 maintainer + 架构 reviewer 共同决定回滚策略;
4. 涉及 ADR 假设不成立的失败 → 暂停所有 Codex 会话（00-strategy §7 紧急熔断条款），先开架构修订 ADR。

---

## 6. M9 Gate

完成本里程碑后，整个 Plan 视为完成。

**最终 Definition of Done 复核**（与 README §4 一致）：
- [x] 19 个 `octopus-harness-*` crate 全部进入 workspace
- [x] 14 个旧 `octopus-sdk*` crate 已移除
- [x] **`_octopus-bridge-stub` 临时 crate 已 `git rm`**（M8 Gate 已验证）
- [x] **`octopus-platform / octopus-infra` 的 `legacy-sdk` feature 已删除或重新接入 `octopus-harness-sdk`**
- [x] 业务层全部切换
- [x] `cargo test --workspace --all-features` 全绿
- [x] `cargo clippy --workspace -- -D warnings` 零警告
- [x] `cargo deny check` 通过
- [x] 3 POC 报告归档（含 P01 至少 L1 通过、L2 应通过）
- [x] 端到端验收通过

---

## 7. 索引

- **上一里程碑** → [`M8-business-cutover.md`](./M8-business-cutover.md)
- **总入口** → [`../README.md`](../README.md)
- **架构评审** → [`docs/architecture/harness/audit/2026-04-25-architecture-review.md`](../../../architecture/harness/audit/2026-04-25-architecture-review.md)
