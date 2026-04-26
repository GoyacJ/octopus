# M2 L1 Primitives Implementation Audit

审计日期：2026-04-26  
范围：`octopus-harness-model` / `journal` / `sandbox` / `permission` / `memory` 五个 L1 crate，以及 M2-S01 prompt-cache spike。

## 结论

M2 L1 修复项已收敛到可验收状态，可以进入下一阶段。

Anthropic provider 真实外部 prompt-cache 测试按当前决策从 M2 gate 移除：当前没有真实 Anthropic key，不把外部 live 测试作为阶段阻断。mock prompt-cache 路径仍保留并通过。

## 修复结果

| 项 | 状态 | 证据 |
|---|---:|---|
| `octopus-harness-model` clippy | PASS | 五个 M2 crate 组合 clippy 已通过。 |
| Bedrock / AWS TLS RUSTSEC | PASS | `rustls-webpki@0.101.7` 已从 `all-providers` feature tree 消失；当前为 `rustls-webpki@0.103.13`。 |
| `cargo deny` feature matrix | PASS | `bash scripts/deny-feature-matrix.sh` 通过。 |
| forbidden `std::sync::{Mutex,RwLock}` | PASS | 生产代码改用 `parking_lot` / async mutex；`spec-consistency.sh` 已覆盖 grouped / alias / direct import。 |
| Permission heartbeat / sweeper | PASS | `StreamBasedBroker` 后台 sweeper 生效；新增 heartbeat 事件与 pending 超时清理测试。 |
| Permission provider watch | PASS | `RuleEngineBroker` 订阅 provider watch，debounce 后原子替换 snapshot；无需手动 reload 的测试通过。 |
| Permission fallback fail-closed | PASS | `AskUser`、`DenyAll`、`AllowReadOnly`、`ClosestMatchingRule` 已按 fail-closed 语义修复并覆盖。 |
| Sandbox CWD side-FD | PASS | shell exec 通过 fd 3 输出 cwd marker，stdout 不混入 marker。 |
| Sandbox snapshot / restore | PASS | `FilesystemImage` tar snapshot / restore 已实现；restore 拒绝绝对路径和 `..` traversal。 |
| Sandbox output overflow | PASS | `Truncate` / `SpillToBlob` / `AbortExec` 均有断言；completed event 带 overflow 摘要。 |
| Sandbox backpressure | PASS | bounded stdout/stderr stream 在 consumer 暂停后发出 `SandboxBackpressureAppliedEvent`。 |
| Shared dangerous patterns | PASS | pattern spec 下沉到 `octopus-harness-contracts`，sandbox / permission 各自编译本地 regex wrapper。 |
| M2 docs drift | PASS | feature flag 表补齐；`MemoryThreatScanner` API contract 改为 struct 形态。 |
| Memory warning | PASS | `threat-scanner` feature 关闭路径不再触发 unused warning。 |
| M2-S01 live prompt-cache | WAIVED | 当前没有真实 Anthropic key；按决策暂不把 Anthropic provider 真实测试作为 M2 出口条件。mock prompt-cache 测试通过。 |

## Gate 证据

| 检查 | 结果 |
|---|---:|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy -p octopus-harness-model -p octopus-harness-journal -p octopus-harness-sandbox -p octopus-harness-permission -p octopus-harness-memory --all-targets --all-features -- -D warnings` | PASS |
| `cargo test -p octopus-harness-model -p octopus-harness-journal -p octopus-harness-sandbox -p octopus-harness-permission -p octopus-harness-memory --all-features --no-fail-fast` | PASS |
| `bash scripts/spec-consistency.sh` | PASS |
| `bash scripts/harness-legacy-boundary.sh` | PASS |
| `bash scripts/dep-boundary-check.sh` | PASS |
| `bash scripts/feature-matrix.sh` | PASS |
| `bash scripts/deny-feature-matrix.sh` | PASS |
| `cargo tree -e features -i rustls-webpki@0.101.7 --features all-providers -p octopus-harness-model` | PASS by absence: package ID did not match any packages |
| `cargo llvm-cov -p octopus-harness-model -p octopus-harness-journal -p octopus-harness-sandbox -p octopus-harness-permission -p octopus-harness-memory --all-features --summary-only` | PASS, total line coverage 79.81% |
| `cargo test -p octopus-harness-model --features anthropic --test spike_prompt_cache --no-fail-fast` | PASS |

## Lane 评估

| Lane | 代码实现 | 功能实现 | 测试实现 | 完成判定 |
|---|---|---|---|---|
| L1-A `model` | Provider trait、CredentialPool、CostCalculator、MockProvider、Anthropic、OpenAI-compatible、OpenAI、OpenRouter、国产 provider、Gemini、Bedrock、Codex、LocalLlama 已修到 clippy clean。 | all-provider surface 可用；Bedrock 走现代 rustls 路径；prompt-cache mock 路径可验证。 | provider contract、prompt-cache mock、credential pool、provider-specific tests 通过。 | 完成。Anthropic live 外部测试不作为 M2 gate。 |
| L1-B `journal` | EventStore、Jsonl、Sqlite、InMemory、BlobStore、Projection、Snapshot、VersionedStore、Retention 已实现。 | append-only、redaction、blob route、replay/projection 可用。 | contract、store、replay、version 测试通过。 | 完成。 |
| L1-C `sandbox` | SandboxBackend、Local、Noop、Docker/SSH rejecting stubs、CodeSandbox surface 已实现。 | Local exec、env filter、cwd guard、side-FD cwd marker、snapshot/restore、overflow/spill/backpressure 可用。 | contract、local、noop、dangerous、fingerprint 测试通过。 | 完成。 |
| L1-D `permission` | PermissionBroker、Direct、StreamBased、RuleEngine、RuleProvider、File/Inline/Admin/Memory provider、IntegritySigner、DangerousPatternLibrary、MockBroker 已实现。 | heartbeat/sweeper、watch auto reload、fail-closed fallback、dangerous escalation 可用。 | contract、stream、rule engine、dangerous、integrity、mock 测试通过。 | 完成。 |
| L1-E `memory` | MemoryStore、MemoryLifecycle、Builtin Memdir、External slot、MemoryManager、ThreatScanner、MockProvider 已实现。 | tenant 隔离、memdir 写入、recall budget、fail-safe、threat scan/redact/block 可用。 | contract、memdir、recall、scanner、external slot 测试通过。 | 完成。 |

## Waived Live Check

Anthropic provider 真实 prompt-cache 测试仍保留为 ignored manual test，但不计入 M2 gate。

原因：当前没有真实 Anthropic key。阶段验收以 mock prompt-cache 注入与 usage mapping、provider contract、all-feature clippy/test、deny/feature/boundary/coverage gate 为准。

## M2 Gate 对照

| M2 gate | 审计结果 | 判定 |
|---|---|---:|
| 5 crate `cargo test --all-features` 全绿 | 组合测试全绿。 | PASS |
| 5 crate contract tests | model 14、journal 4、sandbox 4、permission 3、memory 4。 | PASS |
| `cargo deny` feature matrix | 通过。 | PASS |
| `spec-consistency.sh` | 通过，且已覆盖 grouped sync import。 | PASS |
| feature matrix | 通过。 | PASS |
| coverage gate | `cargo-llvm-cov` summary 通过；total line coverage 79.81%。 | PASS |
| M2-S01 spike | mock 通过；Anthropic live 外部测试按决策暂不计入 M2 gate。 | WAIVED |

M2 当前状态：`PASS / READY FOR NEXT PHASE`。
