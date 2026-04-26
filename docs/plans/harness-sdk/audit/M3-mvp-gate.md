# M3 MVP Gate Audit

> 状态：M3 Gate 已通过，待评审
> 范围：tool / hook / context / session 最小闭环

## 结果

- `e2e_minimal.rs` 已跑通 create session → run turn → UserPromptSubmit hook → context assemble → mock LLM → ListDir tool → permission allow → tool result → assistant message → RunEnded。
- 临时 driver 只存在于 `octopus-harness-session` integration test。
- CLI `run --once <prompt>` 已接入 M3 lower-level driver，旧 SDK 其它路径继续冻结保留。
- 文件头包含 `TODO(M5-T15)` 删除约束。
- 不新增正式 engine 逻辑。

## 已验证

```bash
cargo test -p octopus-harness-session --test e2e_minimal --all-features
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
