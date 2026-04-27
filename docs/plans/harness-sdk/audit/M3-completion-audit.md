# M3 Completion Audit

> 状态：有条件通过
> 审计基准：local `main` @ `4aadf9ac`
> 审计分支：`goya/audit-m3-harness-sdk`
> 审计范围：`docs/architecture/harness`、`docs/plans/harness-sdk`、M3 相关 harness crates、CLI `run --once` 接入路径

## 结论

M3 按计划口径可以进入后续评审。

这个结论只覆盖“最小可运行 SDK + 临时 driver 闭环”。不能把当前 M3 解释成正式 Session engine 已完成。`Session::run_turn` 仍是占位实现，M3 闭环由 session integration test 和 CLI lower-level driver 手动串起事件、context、mock model、tool、permission 和输出。

## 完成度

| 模块 | 结论 | 审计判断 |
|---|---|---|
| `octopus-harness-tool` | 通过，带风险 | Tool trait、registry snapshot、pool、orchestrator、9 个内置工具和 ResultBudget 测试已覆盖 M3 目标。Bash sandbox 上下文存在 `workspace_root` 丢失风险。 |
| `octopus-harness-hook` | 通过，带风险 | 20 类事件、dispatcher、registry、fail-open 默认、PreToolUse 事务语义、in-process / exec / http transport 已实现。HTTP SSRF 只检查 URL host 字面值，未验证 DNS 解析结果。 |
| `octopus-harness-context` | 通过，带缺口 | 5 阶段 compact 顺序、provider 注入、budget、microcompact / autocompact、recall-memory 路径存在。Prompt cache breakpoint 当前没有实际生成逻辑。 |
| `octopus-harness-session` | 部分通过 | 生命周期、paths、projection、fork、reload、steering queue 已落位。正式 `run_turn` 未执行 turn。 |
| CLI cutover | 通过，临时性质明确 | `run --once` 已走 M3 lower-level harness driver，并可触发 `ListDir` 输出。旧 SDK 仍保留在 legacy 路径。 |
| Spike | 通过，需后续集成验证 | Hook replay 与 Steering Queue spike 文档和验证存在。当前 steering 验证仍偏合成场景。 |
| Gate / 脚本 | 通过 | 测试、边界脚本、格式、clippy、deny 均通过。`cargo deny check` 有 duplicate dependency warning，但不是 deny failure。 |

## 主要发现

### P1 · `Session::run_turn` 不是正式 turn 执行

证据：

- `crates/octopus-harness-session/src/session.rs:148` 只检查 session 是否结束，然后返回 `Ok(())`。
- `crates/octopus-harness-session/tests/e2e_minimal.rs:139` 在调用 `session.run_turn(prompt)` 后，由 `MiniDriver` 手动追加 `RunStarted`、dispatch hook、assemble context、构造 mock tool call、执行 tool、写 permission / assistant / run ended 事件。
- `crates/octopus-cli/src/run_once.rs:545` 也采用同类 lower-level driver 流程。

影响：

M3 的“create_session → run_turn → ListDir → output”已跑通，但跑通点不在正式 `Session::run_turn`。这是 M3 计划允许的临时形态，但必须在 M5 engine 落地时替换。否则后续对外 SDK facade 会把占位 API 当成真实执行入口。

要求：

- M5-T15 必须删除 `e2e_minimal.rs` 临时 mini-engine。
- 真 engine 必须接管 `run_turn` 的事件写入、context 组装、模型调用、工具调度、permission、hook 和 run 结束语义。

### P1 · Bash sandbox 执行上下文缺少 workspace root

证据：

- `crates/octopus-harness-tool/src/builtin/bash.rs:139` 构造 `ExecContext`。
- `crates/octopus-harness-tool/src/builtin/bash.rs:145` 将 `workspace_root` 设为 `PathBuf::new()`。
- `ToolContext` 当前没有携带 workspace root。
- `docs/architecture/harness/crates/harness-sandbox.md` 要求 `ExecContext.workspace_root: PathBuf`。

影响：

Bash 是 M3 内置执行类工具。若 sandbox backend 依赖 `workspace_root` 做 WorkspaceOnly 限制、审计归因或路径归一化，当前实现会让边界语义退化。M3 的 CLI smoke 只覆盖 `ListDir`，没有覆盖 Bash sandbox 路径。

要求：

- 在 session / orchestrator / tool context 链路中传递 canonical workspace root。
- 为 Bash + sandbox backend 增加 workspace boundary 测试。

### P2 · HTTP hook SSRF guard 未校验 DNS 解析结果

证据：

- `crates/octopus-harness-hook/src/transport/http.rs:243` 在发送前执行 URL 安全检查。
- `crates/octopus-harness-hook/src/transport/http.rs:256` 调用 `blocks_host(host)`。
- `crates/octopus-harness-hook/src/transport/http.rs:271` 只在 host 是 `localhost` 或可直接解析为 IP 字面量时阻断。
- 非 IP hostname 会在 `host.parse::<IpAddr>()` 失败后返回 `false`。

影响：

allowlist 和 SSRF guard 能挡住 `127.0.0.1`、私网 IP 字面量和 metadata IP 字面量，但不能挡住解析到私网、loopback、link-local 或 metadata IP 的域名。用户可控 HTTP hooks 场景下，这不是完整 SSRF 防护。

要求：

- 发送前解析目标 host，并对所有解析 IP 应用同一 SSRF policy。
- 禁止重定向到未验证地址；当前 `max_redirects = 0` 默认安全，但非零配置也需要逐跳验证。

### P2 · Prompt cache stability 只验证空断点稳定

证据：

- `crates/octopus-harness-context/src/prompt.rs:28` 定义 `PromptCachePolicy` 与 `BreakpointStrategy`。
- `crates/octopus-harness-context/src/engine.rs:117` 返回 `AssembledPrompt`。
- `crates/octopus-harness-context/src/engine.rs:121` 只创建 `Vec::with_capacity(self.cache_policy.max_breakpoints)`，没有写入实际 `CacheBreakpoint`。

影响：

当前实现可以保持“空 breakpoints 稳定”，但不能证明 SystemOnly、SystemAnd3、EveryN 等策略的断点生成稳定。若 M2 prompt-cache spike 的目标是“预留注入位”，当前足够；若 M3 被解释为“prompt cache 策略完成”，则证据不足。

要求：

- 明确 M3 只完成 cache policy 契约占位。
- 在启用实际 prompt cache 前补齐 breakpoint generation 与快照稳定性测试。

### P3 · Steering Queue 仍停留在 spike / 合成验证层

证据：

- M3 有 `docs/architecture/harness/audit/M3-spike-steering.md`。
- Session crate 已有 steering queue 基础行为。
- 当前 M3 闭环不经过正式长 turn engine safe point。

影响：

队列语义已具备基础证据，但还没有在真实 engine turn 生命周期里验证 safe point merge、TTL/drop、interrupt、session end 的组合行为。

要求：

- M5 engine 集成时补真实长 turn 测试。
- M9 runtime session / live control 接入前复验跨重启与投影恢复边界。

## Gate 验证

已验证通过：

```bash
cargo test -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-features
cargo test -p octopus-cli run_once_smoke
bash scripts/spec-consistency.sh
bash scripts/harness-legacy-boundary.sh
bash scripts/dep-boundary-check.sh
git diff --check
cargo fmt --all -- --check
cargo check -p octopus-harness-contracts -p octopus-harness-model -p octopus-harness-memory -p octopus-harness-journal -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-features
cargo clippy -p octopus-harness-contracts -p octopus-harness-model -p octopus-harness-memory -p octopus-harness-journal -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-targets --all-features -- -D warnings
cargo run -p octopus-cli -- run --once "list current dir"
cargo deny check
```

CLI smoke 观察到：

```text
[tool.executed] name=ListDir
```

## 放行边界

M3 可以按“临时 driver MVP gate”放行。

M3 不应按“正式 harness session engine 已完成”放行。

进入 M5 / M8 前必须处理或显式建卡跟踪：

- 替换 `Session::run_turn` 占位实现。
- 删除 M3 临时 mini-engine。
- 修复 Bash sandbox workspace root 传递。
- 补 HTTP hook DNS SSRF 防护。
- 补 prompt cache breakpoint 生成测试。
- 在真实 engine 生命周期中复验 steering queue。
