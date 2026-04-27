# M3 Remediation Audit

> 状态：通过
> 审计日期：2026-04-27
> 审计基准：local `main` @ `4aadf9ac`
> 审计分支：`goya/fix-m3-harness-completion`
> 审计范围：M3 completion remediation diff、`docs/architecture/harness`、`docs/plans/harness-sdk`

## 结论

M3 remediation 已把成功路径从临时 driver 修到正式 `Session::run_turn`。

原审计中的主要缺口大多已关闭：正式 turn runtime、Bash workspace root、HTTP hook DNS SSRF、prompt cache breakpoint、steering safe point、CLI cutover 都有代码和测试证据。

本轮收尾修复已关闭 P1/P2/P3：`RunEnded` 不再结束 session；`run_turn` 已在模型错误路径写 `RunEnded(Error)`；架构边界文档已同步 session 对 `model` / `sandbox` 的 trait 依赖。

## 完成度

| 项 | 结论 | 证据 |
|---|---|---|
| 正式 `Session::run_turn` | 通过 | `crates/octopus-harness-session/src/turn.rs` 已串起 hook、context、model、tool、permission、assistant message、`RunEnded`，并覆盖模型错误收尾。 |
| 删除临时 mini-engine | 通过 | `crates/octopus-harness-session/tests/e2e_minimal.rs` 已删除；CLI 不再含 `HarnessM3RunOnceDriver`。 |
| Bash sandbox workspace root | 通过 | `ToolContext.workspace_root` 已新增；`BashTool::exec_context` 使用该字段；测试验证传入 sandbox。 |
| HTTP hook DNS SSRF | 通过 | `HookHttpDnsResolver`、`with_resolver`、DNS 后 IP 校验、`resolve_to_addrs` 绑定已实现；strict redirect fail-closed。 |
| Prompt cache breakpoint | 通过 | `SystemOnly`、`SystemAnd3`、`EveryN` 已生成稳定 message breakpoints，并有测试覆盖。 |
| Steering 接入真实 run path | 通过 | `run_turn` 在 model infer 前 drain 并合并 steering；测试验证模型请求看到 merged prompt。 |
| CLI cutover | 通过 | `octopus-cli run --once` 使用 `SessionBuilder::with_turn_runtime` 和真实 `ListDir`。 |

## Findings

### P1 · `RunEnded` 污染 session 结束状态

状态：已修复。

证据：

- 架构定义：`docs/architecture/harness/overview.md` 说明一个 Session 由多个 Run 组成，每个 Run 以 `RunStarted` / `RunEnded` 包围。
- 当前实现：`crates/octopus-harness-session/src/projection.rs:195` 在收到 `Event::RunEnded` 时写 `self.end_reason = Some(event.reason)`。
- 当前恢复路径：`crates/octopus-harness-session/src/session.rs:122` 用 `projection.end_reason.is_some()` 初始化 `SessionState.ended`。
- 当前测试：`crates/octopus-harness-session/tests/projection.rs:123` 断言 `RunEnded` 后 `projection.end_reason == Some(Completed)`，把 run end 和 session end 混在一起。

影响：

一次正常 `run_turn` 后，active session 还能继续跑，是因为内存里的 `state.ended` 没被同步设置。但 projection、snapshot、未来 restore、状态展示和任何基于 `from_projection` 的路径都会把 session 看成 ended。M3 的 Session / Projection / Fork 语义不应依赖这种内存态偶然分叉。

修复结果：

- `SessionProjection.end_reason` 只由 `SessionEnded` 写入。
- `harness-journal::SessionProjection` 同步采用相同语义。
- 新增 / 更新测试覆盖 `RunEnded` 不结束 session、`SessionEnded` 才设置 `end_reason`、正常 `run_turn` 后 projection 仍保持 active。

### P2 · `run_turn` 错误路径可能留下孤儿 Run

状态：已修复。

证据：

- `crates/octopus-harness-session/src/turn.rs:69` 先写 `RunStarted`。
- `ContextEngine::assemble`、`model.infer`、`ModelStreamEvent::StreamError`、tool descriptor missing、`context.after_turn` 等路径直接 `return Err(...)`。
- 这些错误路径没有补写 `RunEnded { reason: Error(...) }` 或等价终止事件。
- 架构流程要求 Run 最终写 `RunEnded`：`docs/architecture/harness/crates/harness-engine.md:195`。

影响：

Event stream 中会出现已开始但未结束的 Run。Replay、UI event stream、审计日志和后续 engine facade 都无法可靠判断该 run 是否仍在执行。成功路径已经闭环，但失败路径还不是正式 runtime 语义。

修复结果：

- 在 `run_turn` 内引入单一 finalize 路径。
- 已写 `RunStarted` 后，hook / steering / context / model infer / model stream / tool descriptor / after_turn 错误路径会写 `RunEnded { reason: Error(message) }`。
- 新增测试覆盖 model infer error 和 model stream error。

### P3 · 架构 / gate 文档仍有旧口径残留

状态：已修复。

证据：

- `docs/plans/harness-sdk/milestones/M3-l2-core.md` 仍写 M3 闭环由 `tests/e2e_minimal.rs` 临时 driver 承载。
- `docs/plans/harness-sdk/audit/M3-mvp-gate.md` 仍写 CLI 使用 M3 lower-level driver，且验证命令指向已删除的 `e2e_minimal`。
- `scripts/check_layer_boundaries.py` 已允许 `octopus-harness-session -> model/sandbox`，但 `docs/architecture/harness/module-boundaries.md` 的 session allowed deps 仍未同步。

影响：

代码边界、gate 文档和架构文档不一致。后续 M4/M5 评审会误判 M3 当前形态，也会让边界脚本变成“脚本已放开、架构未批准”的状态。

修复结果：

- 给旧 M3 MVP gate 加 superseded 说明，指向本 remediation audit。
- 更新 M3 milestone 中临时 driver 口径，说明 remediation 后由正式 `Session::run_turn` 承载最小闭环。
- 同步 `module-boundaries.md` 和 `expected-depgraph.dot` 的 session 依赖。

## Gate 验证

修复分支验收命令已通过：

```bash
cargo test -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-features
cargo test -p octopus-harness-journal --all-features
cargo test -p octopus-cli run_once_smoke
cargo run -p octopus-cli -- run --once "list current dir"
bash scripts/spec-consistency.sh
bash scripts/harness-legacy-boundary.sh
bash scripts/dep-boundary-check.sh
cargo fmt --all -- --check
cargo check -p octopus-harness-contracts -p octopus-harness-model -p octopus-harness-memory -p octopus-harness-journal -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-features
cargo clippy -p octopus-harness-contracts -p octopus-harness-model -p octopus-harness-memory -p octopus-harness-journal -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-targets --all-features -- -D warnings
cargo deny check
git diff --check
```

`cargo deny check` 仍有 duplicate dependency warning，但退出码为 0。

## 放行判断

M3 remediation 可以按“正式 `Session::run_turn` 承载最小 SDK 闭环”放行。

边界仍按原假设：M3 只支持单轮、单批 tool calls；多 iteration、grace call、完整 interrupt matrix 和 SDK facade 仍归 M5/M7。
