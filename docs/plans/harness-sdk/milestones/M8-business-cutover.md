# M8 · Business Cutover · 业务层切换到 `octopus-harness-sdk`

> 状态：待启动 · 依赖：M7 完成 · 阻塞：M9
> 关键交付：`octopus-server / octopus-desktop / octopus-cli` 全面切到 `octopus-harness-sdk` · 14 个旧 `octopus-sdk*` crate 删除
> 预计任务卡：12 张 · 累计工时：AI 16 小时（3 路并行约 6 小时墙钟）+ 人类评审 8 小时
> 并行度：**3 路并行**（server / desktop / cli 独立）

---

## 0. 里程碑级注意事项

1. **本里程碑是从冻结旧 SDK 切到新 `octopus-harness-sdk` 的关键步骤**
2. **3 路并行**：`octopus-server / octopus-desktop / octopus-cli` 各自一路；`apps/desktop/src-tauri` 在 desktop 路内
3. **保留 octopus-core / octopus-persistence / octopus-platform / octopus-infra**：这些是业务基础设施，不在 SDK 范畴
4. **旧 SDK 必须在业务全切后删除**：
   - 14 个 `octopus-sdk*` crate 整体在 M8-T12 Gate 前 `git rm`
   - `octopus-platform / octopus-infra` 中只服务旧 SDK 的模块必须删除或基于 `octopus-harness-sdk` 重接
   - 历史会话回放需求由 `octopus-harness-sdk` adapter 满足，不保留旧 SDK live path
5. **`AGENTS.md` Persistence Governance** 必须严格遵守：
   - `runtime/events/*.jsonl` 接 JsonlEventStore
   - `data/main.db` 接 SqliteEventStore（仅作 projection / blob 元数据）
   - `data/blobs/*` 接 FileBlobStore
   - `config/runtime/*.json` 走 octopus-platform 现有 runtime config
6. **adapter 一致性**：`apps/desktop/src/tauri/shell.ts / workspace-client.ts / shared.ts` 必须同时适配 Tauri host + Browser host
7. **OpenAPI / schema 治理**（M8-T02）：SDK 新事件如对外暴露必须走 `docs/api-openapi-governance.md` 流程

---

## 1. 任务卡总览

| 路 | 任务卡 | 内容 |
|---|---|---|
| **B-S** server | M8-T01 ~ T04 | HTTP API 接 SDK 事件流 + 凭证池 + Stream Permission |
| **B-D** desktop | M8-T05 ~ T08 | Tauri command 接 SDK + adapter 切换 |
| **B-C** cli | M8-T09 ~ T11 | CLI 启动 + interactive broker 接线 |
| **共用** | M8-T12 | M8 Gate 检查 + 旧 SDK 删除 + 集成测试 |

---

## 2. 路 B-S · `octopus-server`

### M8-T01 · 移除 octopus-sdk* 依赖 + 引入 octopus-harness-sdk

**预期产物**：
- `crates/octopus-server/Cargo.toml`：
  - 删除 `octopus-sdk*` 依赖
  - 添加 `octopus-harness-sdk = { path = "...", features = [<server profile>] }`
  - 启用 `feature-flags.md` §3.2 服务器生产 profile
- 清除 server 内所有旧 SDK import 与 dependency

**关键不变量**：
- `Cargo.toml` 仅依赖 `octopus-harness-sdk` 和 `octopus-harness-contracts`（除业务专属 crate）

**预期 diff**：< 200 行

---

### M8-T02 · HTTP API 接 SDK 事件流（含 OpenAPI / schema 同步）

**SPEC 锚点**：
- `docs/api-openapi-governance.md`（**必读**：HTTP/API/OpenAPI 修订流程）
- `contracts/openapi/AGENTS.md`（contracts 目录治理规则）
- `contracts/openapi/`
- `packages/schema/src/`

**ADR 锚点**：
- ADR-001（event-sourcing：SSE event tagging 兼容）

**预期产物**：

- 修改 `octopus-server` 中 session / run / event 相关 axum handler
- 把 `EventStream` 通过 SSE 推给前端（轮询 → SSE）
- StreamBasedBroker 接 HTTP 审批接口
- **OpenAPI / schema 同步动作**（实施前评估 P2-1）：
  - 评估 SDK 引入的新事件（**事件名以 `harness-contracts.md §3.3` 与 `event-schema.md §3` 为权威源**：`GraceCallTriggered / SteeringMessageQueued / SteeringMessageApplied / SteeringMessageDropped / ExecuteCodeStepInvoked / ExecuteCodeWhitelistExtended / CredentialPoolSharedAcrossTenants / ManifestValidationFailed / HookFailed / HookPanicked / 其他 v1.8.1 新增`）是否对外（SSE / REST 响应）暴露
  - 如对外暴露 → 走 `docs/api-openapi-governance.md` 流程：先改 `contracts/openapi/src/**` → 跑 `pnpm openapi:bundle` → 跑 `pnpm schema:generate` → 同步 `packages/schema/src/*` 业务 schema → 再改 server handler
  - 如仅内部事件（不暴露给前端）→ 在 PR 描述明确声明"本卡未引入对外 schema 变更，原因：xxx"

**关键不变量**：
- HTTP 契约不变（`packages/schema/src/*`）；如必须变更，**先改 OpenAPI 源 → 再改 server**
- 现有 OpenAPI bundle 必须继续生效（`pnpm openapi:bundle` 通过）
- **`packages/schema/src/generated.ts` 严禁手改**（仓库根 AGENTS.md 已声明）
- 任何新增 SSE event 类型必须在 `contracts/openapi/src/**` 显式声明

**禁止行为**：
- 不要直接改 `packages/schema/src/generated.ts`
- 不要直接改 `contracts/openapi/octopus.openapi.yaml`（必须改 `src/**` 后 bundle）
- 不要在 server crate 内自定义 SSE event 类型不通过 OpenAPI 治理

**验收命令**：

```bash
cargo check -p octopus-server
pnpm openapi:bundle    # 必须通过
pnpm schema:generate   # 必须通过
# 跑前端 type-check（如有）
pnpm --filter desktop type-check
```

**预期 diff**：< 500 行（含 OpenAPI / schema 同步）

---

### M8-T03 · CredentialPool + 多租户隔离接入

**预期产物**：
- 修改 `octopus-server` 的 auth 中间件：把 user 映射为 TenantId + 注入 CredentialKey
- 多租户测试用例

**预期 diff**：< 350 行

---

### M8-T04 · Server 集成测试

**预期产物**：
- `crates/octopus-server/tests/integration.rs`：启动 server → 创建 session → 跑一个 turn → 关 server
- 验证 events 写到 `runtime/events/<tenant>/<session>.jsonl`

**预期 diff**：< 250 行

---

## 3. 路 B-D · `octopus-desktop` + `apps/desktop/src-tauri`

### M8-T05 · 移除 octopus-sdk* + 引入 octopus-harness-sdk

**预期产物**：
- `crates/octopus-desktop/Cargo.toml`
- `apps/desktop/src-tauri/Cargo.toml`
- 启用 desktop CLI 最小 profile（`feature-flags.md` §3.1）

**预期 diff**：< 200 行

---

### M8-T06 · Tauri Command 接 SDK

**预期产物**：
- 修改 `apps/desktop/src-tauri/src/commands/*.rs`：把 session / run / permission 命令切换到 SDK
- 清除 Tauri command 内所有旧 SDK import 与 dependency

**关键不变量**：
- 现有 Tauri command 名称不变（前端 adapter 兼容）
- adapter `apps/desktop/src/tauri/workspace-client.ts` 不必改

**预期 diff**：< 600 行

---

### M8-T07 · DirectBroker 接 Tauri dialog

**预期产物**：
- 实现 `DirectBroker` 包装 Tauri permission 对话框
- 测试：用户点 Allow / Deny 都能正确返回

**预期 diff**：< 300 行

---

### M8-T08 · Desktop 启动 E2E

**预期产物**：
- 跑 `pnpm tauri dev`：启动应用 → 创建 session → 提问 → 工具调用 → 收到流式输出
- 截图归档到 `docs/plans/harness-sdk/audit/M8-desktop-e2e.png`

**预期 diff**：< 200 行

---

## 4. 路 B-C · `octopus-cli`

### M8-T09 · 移除 octopus-sdk* + 引入 SDK

**预期产物**：
- `crates/octopus-cli/Cargo.toml`：CLI 最小 profile + interactive broker

**预期 diff**：< 100 行

---

### M8-T10 · CLI 主流程接线

**预期产物**：
- `crates/octopus-cli/src/main.rs`：启动 → HarnessBuilder → create_session → run_turn 循环
- `crates/octopus-cli/src/interactive_broker.rs`：基于 stdin/stdout 的 DirectBroker 实现

**预期 diff**：< 400 行

---

### M8-T11 · CLI E2E 测试

**预期产物**：
- `crates/octopus-cli/tests/e2e.rs`：通过 `cmd_lib` / `assert_cmd` 跑一个 turn
- 验证 stdout 出现流式 token

**预期 diff**：< 250 行

---

## 5. 共用 · M8 Gate 检查

### M8-T12 · 集成 Gate

**预期产物**：
- 一份 `docs/plans/harness-sdk/audit/M8-cutover-gate.md`（人类填写）
- 14 个旧 `octopus-sdk*` crate 已从 workspace 与磁盘删除
- 跑全 workspace 集成测试：`cargo test --workspace --release`
- `pnpm openapi:bundle` 通过

**Gate 通过判据**：
- ✅ `cargo build --workspace --release` 通过
- ✅ `cargo test --workspace --all-features` 全绿
- ✅ `crates/octopus-server` 启动且 HTTP API 可访问
- ✅ `apps/desktop` 通过 `pnpm tauri dev` 启动并能完成一次完整对话
- ✅ `crates/octopus-cli` 可执行命令成功
- ✅ `runtime/events/*.jsonl` 被正确写入
- ✅ `data/main.db` 投影正确
- ✅ 业务层旧 `octopus-sdk*` 引用全部清除（grep 验证）
- ✅ 14 个旧 `octopus-sdk*` crate 已 `git rm`
- ✅ `octopus-platform / octopus-infra` 中只服务旧 SDK 的模块已通过 `octopus-harness-sdk` 重新接入或删除
- ✅ 不存在 `_octopus-bridge-stub` 或 `legacy-sdk` feature
- ✅ `pnpm openapi:bundle` + `pnpm schema:generate` 通过；前端 schema 与 SDK 事件对齐

---

## 6. SPEC 一致性自检（M8 全局）

```bash
# 不应再出现旧 sdk 引用
! grep -rE 'octopus_sdk' crates/octopus-server crates/octopus-desktop crates/octopus-cli apps/desktop/src-tauri --include='*.rs'
! grep -rE 'octopus-sdk' crates/octopus-server crates/octopus-desktop crates/octopus-cli apps/desktop/src-tauri --include='Cargo.toml'

# 旧 SDK crate 目录已被删除
! ls crates/ | grep -E '^octopus-sdk'

# 必须依赖 octopus-harness-sdk
grep -q 'octopus-harness-sdk' crates/octopus-server/Cargo.toml
grep -q 'octopus-harness-sdk' crates/octopus-desktop/Cargo.toml
grep -q 'octopus-harness-sdk' crates/octopus-cli/Cargo.toml

# 不应残留临时兼容通道
! grep -rE '_octopus[-_]bridge[-_]stub|legacy-sdk' crates/ apps/ --include='*.toml' --include='*.rs'
```

---

## 7. 索引

- **上一里程碑** → [`M7-l4-facade.md`](./M7-l4-facade.md)
- **下一里程碑** → [`M9-poc-and-acceptance.md`](./M9-poc-and-acceptance.md)
- **D10 Feature Flags** → [`docs/architecture/harness/feature-flags.md`](../../../architecture/harness/feature-flags.md)
