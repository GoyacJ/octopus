# M3 MVP Gate Audit

> 状态：已被 M3 remediation supersede
> 范围：tool / hook / context / session 最小闭环

> 后续权威结论见 `docs/plans/harness-sdk/audit/M3-remediation-audit.md`。

## 结果

- `Session::run_turn` 已跑通 create session → run turn → UserPromptSubmit hook → context assemble → mock LLM → ListDir tool → permission allow → tool result → assistant message → RunEnded。
- `e2e_minimal.rs` 临时 driver 已删除。
- CLI `run --once <prompt>` 已接入 M3 `Session::run_turn` runtime，旧 SDK 其它路径继续冻结保留。
- 不提前实现 M5 完整 engine 主循环。

## 已验证

```bash
cargo test -p octopus-harness-session --test run_turn --all-features
cargo fmt --all -- --check
cargo check -p octopus-harness-contracts -p octopus-harness-model -p octopus-harness-memory -p octopus-harness-journal -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-features
cargo clippy -p octopus-harness-contracts -p octopus-harness-model -p octopus-harness-memory -p octopus-harness-journal -p octopus-harness-tool -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session --all-targets --all-features -- -D warnings
cargo test -p octopus-harness-tool --all-features
cargo test -p octopus-harness-hook --all-features
cargo test -p octopus-harness-context --all-features
cargo test -p octopus-harness-session --all-features
cargo test -p octopus-cli run_once_smoke
bash scripts/spec-consistency.sh
bash scripts/harness-legacy-boundary.sh
bash scripts/dep-boundary-check.sh
git diff --check
```

## Gate 结论

M3 下层 core 闭环已满足进入评审条件。M4 / M5 仍需 maintainer 明确放行后再启动。
