# W8 · 清理与拆分 + `octopus-persistence`

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `docs/sdk/04-session-brain-hands.md §4.5` → `docs/sdk/10-failure-modes.md §10.8` → `docs/plans/sdk/00-overview.md §3 W8 / §5` → `docs/plans/sdk/02-crate-topology.md §1.2 / §3.2 / §3.3 / §8` → `docs/plans/sdk/03-legacy-retirement.md §0 / §8 / §9` → `Cargo.toml` → `crates/octopus-platform/src/runtime_sdk/{secret_vault,registry_bridge}.rs` → `crates/octopus-infra/src/{infra_state,projects_teams,agent_assets,access_control}.rs` → `crates/octopus-server/src/{handlers,workspace_runtime}.rs` → `crates/octopus-core/src/lib.rs`。

## Status

状态：`done`

## Active Work

当前 Task：`Task 9`

当前 Step：`completed`

Open Questions：

- <none>

## Goal

上线 `crates/octopus-persistence` 作为业务侧 SQLite 入口，清掉业务 crate 的直连 `Connection::open`，并把当前所有超 800 行的 Rust 文件拆到门禁以内，在不改变 W7 已交付 runtime / host 合同的前提下完成 SDK 重构收尾。

## Non-goal

- 不把 `octopus-sdk-session` 并入 `octopus-persistence` 或改写 W1 已冻结的 SQLite + JSONL 双通道语义。
- 不在 W8 借“拆文件”顺手改 `/api/v1/*` payload、route、auth 或 transport 合同。
- 不把 runtime config、业务 DTO、前端状态缓存引入 `octopus-persistence`。
- 不靠忽略 `octopus-platform` / `octopus-cli` 的超限文件来绕过 repo 级 800 行门禁。

## Architecture

- `octopus-persistence` 只接业务侧 SQLite ownership：`octopus-infra`、`octopus-platform`、`octopus-server` 通过统一 `Database` / repository 边界取连接、跑 migration、落 query-heavy projection；它不接管 file-first runtime config 的真相源。
- `octopus-sdk-session` 继续遵守 W1 的“双通道”约束：`data/main.db` 负责结构化索引，`runtime/events/*.jsonl` 负责 append-only 事件流；它不经 `octopus-persistence` 取连接。若后续要改这条边界，先改控制面再落代码。
- 文件拆分按 ownership 做，不按“为了过行数扫描”做随机切片：`octopus-server` 按 route / resource，`octopus-infra` 按 repository / domain，`octopus-core` 按 record / type domain，`octopus-platform` 和 `octopus-cli` 按 runtime / command 子域。

## Scope

- In scope：
  - 新建 `crates/octopus-persistence`。
  - 迁移 `octopus-infra`、`octopus-platform`、`octopus-server` 的生产 SQLite 入口到 `octopus-persistence`。
  - 保持 `00-overview.md`、`02-crate-topology.md` 与 `Cargo.toml` 对 `octopus-sdk-session` / `default-members` 的控制面一致。
  - 拆分当前全部超 800 行的 Rust 文件，包括：
    - `crates/octopus-core/src/lib.rs`
    - `crates/octopus-server/src/{lib.rs,handlers.rs,workspace_runtime.rs}`
    - `crates/octopus-infra/src/{infra_state.rs,projects_teams.rs,agent_assets.rs,access_control.rs,resources_skills.rs,auth_users.rs,artifacts_inbox_knowledge.rs,project_tasks.rs,agent_bundle/import.rs}`
    - `crates/octopus-platform/src/runtime_sdk/{config_bridge.rs,registry_bridge.rs}`
    - `crates/octopus-cli/src/{automation.rs,workspace.rs}`
  - 复核 W7 的 legacy 目录删除态、`runtime/sessions/*.json` 退场约束、workspace build / clippy / test。
- Out of scope：
  - 新业务功能、新 UI 功能、新 transport 能力。
  - 把 runtime config 改成数据库权威源。
  - 重新引入 legacy crate 或恢复 `octopus-runtime-adapter` 语义。
  - 无必要地扩 `octopus-sdk-*` 公共面；若必须改，先回填 `02-crate-topology.md` 再继续。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | 执行中若把 `octopus-sdk-session` 误并到 `octopus-persistence`，会破坏 W1 双通道边界。 | 保持 `SqliteJsonlSessionStore` 在 SDK 侧独立；若要变更 ownership，先改控制面再写代码。 | #1 / #8 |
| R2 | W8 执行中若调整 `default-members`，`00` / `02` / `Cargo.toml` 容易再次漂移。 | 当前口径以 `02 §8` + live `Cargo.toml` 为准；后续收敛必须配置与文档同批改。 | #8 |
| R3 | `octopus-persistence` 迁移时把 file-first runtime config 顺手塞进 `main.db`。 | 明确只收 SQLite connection / migration / repository ownership；config 仍 file-first。 | #2 |
| R4 | 为了过 800 行门禁，把 `server / infra` 的真实 ownership 切碎。 | 只按 resource / repository / domain 拆；禁止“helper.rs 大杂烩”。 | #4 |
| R5 | `octopus-platform` / `octopus-cli` 当前也有 >800 行文件，但总控示例列表没写到。 | 以硬门禁为准，把 residual offenders 纳入 Task Ledger；如要缩范围，先改 `00-overview.md`。 | #8 |
| R6 | `octopus-server` 文件拆分时改变 runtime transport / route 行为。 | 所有拆分先保持函数签名和 route 注册不变，行为变化必须走 OpenAPI-first。 | #3 |
| R7 | `octopus-infra` 的大量测试直接 `Connection::open`，迁移后可能先爆测试。 | 测试可暂时保留直开库；W8 先清生产路径，测试路径只在不破坏隔离前提下逐步收口。 | #6 |
| R8 | `octopus-core/lib.rs` 公共 re-export 太大，拆分时容易改坏外部 import 面。 | 先拆内部 module，再用 `lib.rs` 保持现有 export surface；变更 public 面必须回填 `02`。 | #1 |

## 已确认的审核决策（2026-04-22）

| # | 决策点 | 确认结论 | 关联章节 |
|---|---|---|---|
| D1 | W8 主线顺序 | **先持久化 ownership，后文件级拆分，最后做 repo 级行数收尾。** 不把“接管 SQLite”与“大规模 module split”混在同一批。 | Architecture / Task 1 / Task 2 / Task 3 |
| D2 | 持久化边界 | **`octopus-persistence` 只接业务侧 SQLite ownership，不改变 file-first runtime config。** | Architecture / Scope / Task 1 |
| D3 | 行数门禁口径 | **以 `find crates ... wc -l ... > 800` 的 repo 级硬门禁为准。** 总控示例列表不是豁免名单。 | Scope / Task 4–Task 8 |
| D4 | legacy 态处理 | **W8 只复核 W7 的删除态，不恢复任何 legacy crate。** 若新 split 需要旧实现参考，只读历史，不回拉代码。 | Scope / Task 9 |

## 公共面变更登记

| 变更点 | 登记位置 | 当前冻结结论 | 触发条件 |
|---|---|---|---|
| `octopus-persistence::Database` 与首批 repository 边界 | `docs/plans/sdk/02-crate-topology.md §3.2` | 只管理业务侧 SQLite connection / migration / repository ownership；`octopus-sdk-session` 保持独立。 | Task 1 Step 2 若公共面比当前草案更宽或更窄 |
| `octopus-server` 的 resource split 与 route 边界 | `docs/plans/sdk/02-crate-topology.md §3.3` | 拆分只按 resource/module，route path、payload、auth 行为不变。 | Task 2 或 Task 6–8 需要改 server 公共面时 |
| workspace `default-members` 策略 | `docs/plans/sdk/02-crate-topology.md §8` + `docs/plans/sdk/00-overview.md §3/§5` | 现行控制面跟随 live `Cargo.toml`；若 W8 后续决定收敛，必须配置与文档同批改动。 | Task 1 Step 1 或后续执行实际修改 `Cargo.toml` 时 |
| `docs/sdk/*` 规范层勘误 | `docs/sdk/README.md` `## Fact-Fix 勘误` | 本次审计未证明需要改 `docs/sdk/*`。 | 仅当执行暴露规范层与实现矛盾时 |

## 退役登记

| 退役项 | 登记位置 | W8 要求 | 验证 |
|---|---|---|---|
| W7 已删除的 11 个 legacy crate 目录 | `docs/plans/sdk/03-legacy-retirement.md §9` | 只复核删除态，不恢复任何目录或旧依赖。 | `ls crates/ | rg '^(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)$'` |
| `runtime/sessions/*.json` 作为恢复源 | `docs/plans/sdk/03-legacy-retirement.md §8` | 仅允许测试或显式 debug export 命中。 | `rg "runtime/sessions/.*\\.json" crates/ --glob '!**/tests/**' --glob '!**/fixtures/**'` |
| `split_module_tests.rs` 风格的大型测试合集 | `docs/plans/sdk/03-legacy-retirement.md §2.2` + `docs/plans/sdk/02-crate-topology.md §4.2` | 若 W8 触碰相关测试，优先拆到 feature tests 或小型同文件测试。 | `find crates -type f -name '*split_module_tests.rs'` 仅允许遗留待拆点，不能新增 |

## Weekly Gate 对齐表（W8）

| `00-overview.md §3` 条目 | 本周落点 | 验证 |
|---|---|---|
| `crates/octopus-persistence` 上线，业务侧 SQLite ownership 收口；`octopus-sdk-session` 保持独立 | Task 1 / Task 2 / Task 3 | `cargo test -p octopus-persistence -p octopus-platform -p octopus-server -p octopus-infra && ! rg "Connection::open\\(" crates/octopus-platform/src/runtime_sdk crates/octopus-server/src crates/octopus-infra/src --glob '!**/tests/**' --glob '!**/test_*.rs' --glob '!**/*tests.rs' --glob '!**/split_module_tests.rs'` |
| `octopus-core / octopus-infra / octopus-server` 拆到 ≤ 800 行 | Task 4 / Task 5 / Task 6 / Task 7 / Task 8 | `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` |
| W7 的 11 个 legacy crate 删除态保持成立 | Task 9 | `ls crates/ | rg '^(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)$'` |
| `runtime/sessions/*.json` 仅剩测试 / debug export | Task 9 | `rg "runtime/sessions/.*\\.json" crates/ --glob '!**/tests/**' --glob '!**/fixtures/**'` |
| workspace build / clippy / desktop suite 全绿 | Task 9 | `cargo test --workspace && cargo clippy --workspace -- -D warnings && pnpm -C apps/desktop test` |

## Execution Rules

- 持久化 ownership 未冻结前，不进入生产代码迁移。
- 每次拆分只处理一个 ownership cluster；单批 diff > 800 行必须继续拆。
- 任何 `/api/v1/*` payload 变化都先改 `contracts/openapi/src/**`，不手改生成物。
- `octopus-persistence` 不得反向依赖 `octopus-sdk-*` 以外的业务 runtime 语义；只持有 database / repository 责任。
- 生产代码的 `Connection::open` 清零优先于测试代码清零；测试路径允许阶段性保留直连数据库。
- 执行中若需要修改 `docs/sdk/*`，必须先在 `docs/sdk/README.md` 追加 Fact-Fix 再继续。

## Task Ledger

### Task 1: 冻结 `octopus-persistence` ownership 与控制面口径

Status: `done`

Files:
- Modify: `docs/plans/sdk/11-week-8-cleanup-and-split.md`
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Create: `crates/octopus-persistence/Cargo.toml`
- Create: `crates/octopus-persistence/src/lib.rs`
- Create: `crates/octopus-persistence/src/database.rs`
- Create: `crates/octopus-persistence/src/migrations.rs`
- Create: `crates/octopus-persistence/tests/database.rs`

Preconditions:
- W7 Weekly Gate 已完成。
- `00-overview.md §3/§5`、`02-crate-topology.md §3.2/§8`、当前 `Cargo.toml` 已完成逐条核对。

Step 1:
- Action: 复核 `00` / `02` / `Cargo.toml` 三处控制面，确认两条冻结结论仍成立：`octopus-sdk-session` 不走 `octopus-persistence`；workspace `default-members` 现行策略跟随 `02 §8` 与 live `Cargo.toml`。
- Done when: 三处文档/配置对同一问题保持一个口径；若执行期需要改策略，阻塞项被明确写进本文件和 `README` 状态。
- Verify: `rg -n "octopus-persistence|SqliteJsonlSessionStore|default-members|5 业务 crate" docs/plans/sdk/{00-overview.md,02-crate-topology.md,README.md} Cargo.toml`
- Stop if: `octopus-sdk-session` 的 ownership 无法在“SDK 自持”与“业务侧集中管理”之间给出单一答案。

Step 2:
- Action: 新建 `crates/octopus-persistence` 最小骨架，冻结 `Database::open / acquire / run_migrations` 之类的业务侧入口，不把 runtime config 或业务 DTO 混进 crate 根面。
- Done when: `octopus-persistence` 可独立 build / test，且对外面只表达 database / repository 责任。
- Verify: `cargo test -p octopus-persistence`
- Stop if: crate 需要直接依赖 `octopus-server` / `octopus-infra` 业务类型才能成立。

Notes:
- `octopus-sdk-session` 当前冻结为 SDK 自持会话投影存储；除非另开控制面决策，不进入 `octopus-persistence`。
- `default-members` 当前冻结为 `02 §8` / live `Cargo.toml` 记载的扩展闭包；若后续收敛，必须同批修改文档与配置。

### Task 2: `octopus-platform` / `octopus-server` 切到 `octopus-persistence`

Status: `done`

Files:
- Modify: `crates/octopus-platform/Cargo.toml`
- Modify: `crates/octopus-platform/src/runtime_sdk/mod.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/builder.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/secret_vault.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs`
- Modify: `crates/octopus-platform/tests/runtime_config_bridge.rs`
- Modify: `crates/octopus-server/Cargo.toml`
- Modify: `crates/octopus-server/src/handlers.rs`
- Modify: `crates/octopus-server/src/lib.rs`
- Create: `crates/octopus-server/src/handlers/`
- Modify: `crates/octopus-server/src/test_runtime_sdk.rs`

Preconditions:
- Task 1 已冻结 `octopus-persistence` 公共面。
- 已确认 `octopus-platform` / `octopus-server` 不改变现有 runtime transport shape。

Step 1:
- Action: 把 `octopus-platform::runtime_sdk` 里的生产数据库入口改为通过 `octopus-persistence` 获取连接与 migration，清掉 direct `Connection::open`。
- Done when: `crates/octopus-platform/src/runtime_sdk/*.rs` 生产代码不再直开 `main.db`。
- Verify: `cargo test -p octopus-platform && ! rg "Connection::open\\(" crates/octopus-platform/src/runtime_sdk`
- Stop if: platform runtime bridge 需要把 repository 细节泄漏给 `octopus-sdk-*` 或 server handler。

Step 2:
- Action: 把 `octopus-server` 的 host notification / host-level SQLite 使用收口到 `octopus-persistence`，并保持既有 route 行为。
- Done when: `crates/octopus-server` 生产代码不再直接 `Connection::open(data/main.db)`。
- Verify: `cargo test -p octopus-server && ! rg "Connection::open\\(" crates/octopus-server/src --glob '!**/test_*.rs' --glob '!**/*tests.rs'`
- Stop if: server 侧需要借机改 HTTP payload / route contract 才能完成迁移。

### Task 3: `octopus-infra` 切到 `octopus-persistence`

Status: `done`

Files:
- Modify: `crates/octopus-infra/Cargo.toml`
- Modify: `crates/octopus-infra/src/lib.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/workspace_paths.rs`
- Create: `crates/octopus-infra/src/persistence/`
- Modify: `crates/octopus-infra/src/bootstrap.rs`
- Modify: `crates/octopus-infra/src/split_module_tests.rs`

Preconditions:
- Task 1 已完成。
- `octopus-infra` 的 repository ownership 与 `octopus-platform` / `octopus-server` 边界已明确。

Step 1:
- Action: 把 `InfraState::open_db()`、`initialize_database()`、seed / load 路径改成通过 `octopus-persistence` 取连接与执行 migration。
- Done when: `octopus-infra` 生产路径不再把 `WorkspacePaths.db_path` 当成“谁都能直接 open”的入口。
- Verify: `cargo test -p octopus-infra && ! rg "Connection::open\\(" crates/octopus-infra/src --glob '!**/tests/**' --glob '!**/test_*.rs' --glob '!**/*tests.rs' --glob '!**/split_module_tests.rs'`
- Stop if: `octopus-infra` 的 load / save API 无法在不重写业务语义的前提下迁出 direct-open 入口。

Step 2:
- Action: 把高频 load / save SQL helpers 按 repository / bootstrap / projection 三类先抽到 `src/persistence/`，为后续 W8 拆文件任务清出边界。
- Done when: `infra_state.rs` 的数据库 bootstrap 与 query helper 不再继续膨胀，且后续拆分任务有稳定着力点。
- Verify: `cargo test -p octopus-infra`
- Stop if: 抽取后出现循环依赖，需要把业务类型下沉到 `octopus-persistence`。

### Task 4: 拆 `octopus-core/src/lib.rs`

Status: `done`

Files:
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-core/src/`

Preconditions:
- Task 1 已明确 public surface 不新增裸符号。
- 已确认 `octopus-core` 只做 domain records / shared error / shared payload，不承载 infra / server logic。

Step 1:
- Action: 按 domain 把 `lib.rs` 拆成稳定模块，`lib.rs` 只保留 `mod` + 受控 re-export。
- Done when: `lib.rs` 不再承载大段类型定义，且现有外部 import 面保持兼容。
- Verify: `cargo test -p octopus-core`
- Stop if: 模块拆分要求同时改动大量外部 import 或改变 public type path。

Step 2:
- Action: 跑行数守护，确认 `crates/octopus-core/src/lib.rs` ≤ 80，相关新模块各自 ≤ 800。
- Done when: `octopus-core` 不再命中 W8 行数门禁。
- Verify: `find crates/octopus-core -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`
- Stop if: 某个 domain 模块天然 > 800，需要更细一级 ownership 才能落地。

### Task 5: 拆 `octopus-server` 的 route / runtime 大文件

Status: `done`

Files:
- Modify: `crates/octopus-server/src/lib.rs`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/handlers.rs`
- Create: `crates/octopus-server/src/handlers/`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Create: `crates/octopus-server/src/workspace_runtime/`
- Modify: `crates/octopus-server/src/test_runtime_sdk.rs`

Preconditions:
- Task 2 已完成。
- `02-crate-topology.md §3.3` 的 server resource split 边界已作为唯一目标态。

Step 1:
- Action: 先按 host / access / workspace / project / task / catalog / knowledge / inbox 等资源把 `handlers.rs` 拆成子模块，不改 route contract。
- Done when: `handlers.rs` 主文件只保留组装 / 导出，资源处理函数落到对应子模块。
- Verify: `cargo test -p octopus-server`
- Stop if: 为了拆分必须同时改 API 路径、payload 或 auth 行为。

Step 2:
- Action: 再按 `/api/v1/runtime/*` 资源族拆 `workspace_runtime.rs`，把 turn / session / events / config / approval / generation 等路径分开。
- Done when: `workspace_runtime.rs` 主文件与子模块全部 ≤ 800，runtime transport 测试继续通过。
- Verify: `cargo test -p octopus-server && pnpm -C apps/desktop exec vitest run test/runtime-store.test.ts`
- Stop if: 拆分暴露出新的 runtime contract 漂移，需要先走 OpenAPI-first。

Step 3:
- Action: 清理 `octopus-server/src/lib.rs` 的再导出和模块装配，避免主文件继续过长。
- Done when: `crates/octopus-server/src/{lib.rs,handlers.rs,workspace_runtime.rs}` 全部不再命中行数守护。
- Verify: `find crates/octopus-server -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`
- Stop if: 行数下降只能靠把逻辑移到新“杂物模块”而不是 resource ownership 模块。

### Task 6: 拆 `octopus-infra::infra_state`

Status: `done`

Files:
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/persistence/mod.rs`
- Create: `crates/octopus-infra/src/persistence/schema.rs`
- Create: `crates/octopus-infra/src/infra_state/`

Preconditions:
- Task 3 已把 DB ownership 收口到稳定入口。
- `infra_state.rs` 的 bootstrap / migrations / loaders / defaults 边界已通过 Task 3 分出一层。

Step 1:
- Action: 把 `infra_state.rs` 按 `config / bootstrap`、`schema / migrations`、`loaders`、`defaults / pet projection` 至少拆成独立模块。
- Done when: `infra_state.rs` 主文件只保留状态结构与薄入口，子模块各自职责单一。
- Verify: `cargo test -p octopus-infra`
- Stop if: 某个子模块拆出后仍强耦合全部 `InfraState` 内部细节，导致边界名不副实。

Step 2:
- Action: 跑 targeted line scan，确认 `infra_state.rs` 相关文件全部 ≤ 800。
- Done when: `crates/octopus-infra/src/infra_state*.rs` 不再命中行数守护。
- Verify: `find crates/octopus-infra/src/infra_state* -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`
- Stop if: 还需要再拆一层 repository / domain 才能满足 800 行门禁。

### Task 7: 拆 `octopus-infra` 的 access / project / asset 大文件

Status: `done`

Files:
- Modify: `crates/octopus-infra/src/access_control.rs`
- Create: `crates/octopus-infra/src/access_control/`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Create: `crates/octopus-infra/src/projects_teams/`
- Modify: `crates/octopus-infra/src/agent_assets.rs`
- Create: `crates/octopus-infra/src/agent_assets/`

Preconditions:
- Task 3 已完成。
- Task 6 已把共享 bootstrap / loader 边界稳定下来。

Step 1:
- Action: 把 `access_control.rs` 按 role / policy / assignment / audit 或等价资源边界拆开。
- Done when: `access_control.rs` 主文件与子模块都 ≤ 800，且 public entry points 不变。
- Verify: `cargo test -p octopus-infra access_control -- --nocapture`
- Stop if: access control 逻辑拆分需要改权限语义或跨 crate API。

Step 2:
- Action: 把 `projects_teams.rs` 按 project / team / link / dashboard 等边界拆开。
- Done when: `projects_teams.rs` 不再是多资源耦合文件，相关测试继续通过。
- Verify: `cargo test -p octopus-infra projects_teams -- --nocapture`
- Stop if: 项目 / 团队链接与 dashboard 共享状态无法在不引入新全局模块的前提下拆开。

Step 3:
- Action: 把 `agent_assets.rs` 按 parse / import / export / builtin-skill-mcp / avatar 等边界拆开。
- Done when: `agent_assets.rs` 主文件与子模块都 ≤ 800。
- Verify: `cargo test -p octopus-infra agent_assets -- --nocapture`
- Stop if: bundle parse / export / asset persistence 的 ownership 仍不清楚，需要先补更高层设计。

### Task 8: 清零剩余 >800 行文件

Status: `done`

Files:
- Modify: `crates/octopus-infra/src/resources_skills.rs`
- Modify: `crates/octopus-infra/src/auth_users.rs`
- Modify: `crates/octopus-infra/src/artifacts_inbox_knowledge.rs`
- Modify: `crates/octopus-infra/src/project_tasks.rs`
- Modify: `crates/octopus-infra/src/agent_bundle/import.rs`
- Create: `crates/octopus-infra/src/resources_skills/`
- Create: `crates/octopus-infra/src/auth_users/`
- Create: `crates/octopus-infra/src/artifacts_inbox_knowledge/`
- Create: `crates/octopus-infra/src/project_tasks/`
- Create: `crates/octopus-infra/src/agent_bundle/import/`
- Modify: `crates/octopus-platform/src/runtime_sdk/config_bridge.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/config_bridge/`
- Create: `crates/octopus-platform/src/runtime_sdk/registry_bridge/`
- Modify: `crates/octopus-cli/src/automation.rs`
- Modify: `crates/octopus-cli/src/workspace.rs`
- Create: `crates/octopus-cli/src/automation/`
- Create: `crates/octopus-cli/src/workspace/`

Preconditions:
- Task 2–Task 7 已把主 ownership cluster 稳住。
- 当前全仓超限列表已复核。

Step 1:
- Action: 清理 `octopus-infra` 的 residual offenders，按 resource / subdomain 继续拆到 ≤ 800。
- Done when: `resources_skills.rs`、`auth_users.rs`、`artifacts_inbox_knowledge.rs`、`project_tasks.rs`、`agent_bundle/import.rs` 全部清零超限。
- Verify: `find crates/octopus-infra -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`
- Stop if: residual offenders 实际暴露出新的 persistence / transport ownership 问题。

Step 2:
- Action: 清理 `octopus-platform::runtime_sdk::{config_bridge,registry_bridge}` 与 `octopus-cli::{automation,workspace}` 的 residual offenders。
- Done when: `octopus-platform` 和 `octopus-cli` 不再命中行数守护。
- Verify: `find crates/octopus-platform crates/octopus-cli -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`
- Stop if: 拆分要求改动 desktop / CLI 行为，而不是纯 module ownership 收口。

### Task 9: W8 Weekly Gate 与文档收口

Status: `done`

Files:
- Modify: `docs/plans/sdk/11-week-8-cleanup-and-split.md`
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/sdk/README.md`（仅发生 Fact-Fix 时）

Preconditions:
- Task 1–8 全部完成或明确 `blocked`。

Step 1:
- Action: 跑 W8 Weekly Gate，补 checkpoint、状态、变更日志、legacy 复核与 DoD 对齐。
- Done when: W8 出口状态与硬门禁逐条勾选完成，`README` 中 W8 行切到 `done`。
- Verify: `cargo test --workspace && cargo clippy --workspace -- -D warnings && pnpm -C apps/desktop test && ! rg "runtime/sessions/.*\\.json" crates/ --glob '!**/tests/**' --glob '!**/fixtures/**' && ! ls crates/ | rg '^(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)$'`
- Stop if: 任一 Weekly Gate 只能靠“示例列表没写到”或“先不看这个文件”来过门禁。

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-22 | 首稿：新建 W8 计划，冻结“先持久化 ownership、再按 ownership 拆文件、最后跑 repo 级 ≤800 行门禁”的执行顺序；把 `octopus-persistence` / `default-members` / `sdk-session` 三处控制面冲突显式登记为 Task 1 前置。 | Codex |
| 2026-04-22 | 文档审计修复：补齐 `Non-goal / 公共面变更登记 / 退役登记` 三个必备章节；把 `octopus-sdk-session` 冻结为独立于 `octopus-persistence`；把 `Connection::open` 守护改成只扫生产路径，不把 `split_module_tests.rs` / `test_runtime_sdk.rs` 误算进门禁。 | Codex |
| 2026-04-22 | Task 1 执行完成：`octopus-persistence` 首批公共面冻结为 `Database + Migration`；新增 crate 骨架与迁移账本；`Cargo.toml` / `README.md` / `02-crate-topology.md` 同批补入 `octopus-persistence` 的 live 控制面。 | Codex |
| 2026-04-22 | Task 2 执行完成：`octopus-platform::runtime_sdk` 与 `octopus-server` host notifications 改走 `octopus-persistence::Database`；`secret_vault` 与 host notifications 的建表迁移收口到 migration registry；`Connection::open` 已从平台 runtime_sdk 和 server 生产路径移除。 | Codex |
| 2026-04-22 | Task 3 执行完成：`octopus-infra` 的生产 SQLite 入口统一改走 `WorkspacePaths::database()` + `octopus-persistence`；新增 `src/persistence/` 边界；清掉 `octopus-infra/src` 内所有 `Connection::open` 命中，并修正一处跟随 MCP discovery 演进而失效的内联测试。 | Codex |
| 2026-04-22 | Task 4 执行完成：`octopus-core/src/lib.rs` 按 app / host / workspace / model catalog / capability / access control / runtime config / runtime memory / runtime session / operations 等 domain 拆成稳定模块；`lib.rs` 收敛为 `mod` + `pub use`，公共导出面保持不变；`octopus-core` 全部相关文件已落到 W8 的 ≤800 行门禁内。 | Codex |
| 2026-04-22 | Task 5 执行完成：`octopus-server` 的 `handlers.rs`、`workspace_runtime.rs`、`lib.rs` 已按 resource/runtime ownership 拆成子模块；修复 `lib.rs` helper split 的可见性与边界串位；`cargo test -p octopus-server`、`apps/desktop` 的 `runtime-store.test.ts`、以及 `octopus-server` 行数门禁全部通过。 | Codex |
| 2026-04-22 | Task 6 执行完成：`octopus-infra::infra_state` 已按 `config / startup / schema / loaders / defaults / pet` 拆成子模块，根模块只保留状态结构、常量、重导出与定向测试；`cargo test -p octopus-infra` 与 `infra_state*` 行数门禁全部通过。 | Codex |
| 2026-04-22 | Task 7 执行完成：`octopus-infra::{access_control,projects_teams,agent_assets}` 已按 access/project+team+resource/asset parse+catalog+runtime+export ownership 拆成子模块；`projects_teams` 根模块收敛为 trait 委托层，`agent_assets` 根模块收敛为 types + include 装配层；定向测试与 `cargo test -p octopus-infra` 全量回归全部通过。 | Codex |
| 2026-04-22 | Task 8 执行完成：`octopus-infra` 的 residual offenders 已清零；`octopus-cli::{automation,workspace}` 与 `octopus-platform::runtime_sdk::{registry_bridge,config_bridge}` 已按 command/runtime ownership 拆成子模块，`cargo test -p octopus-cli`、`cargo test -p octopus-platform` 与 repo 级 `octopus-cli + octopus-platform` 行数门禁全部通过。 | Codex |
| 2026-04-22 | Task 9 执行完成：W8 Weekly Gate 通过；`cargo test --workspace`、`cargo clippy --workspace -- -D warnings`、`pnpm -C apps/desktop test`、legacy 目录复核、`runtime/sessions/*.json` 守护与 repo 级 ≤800 行门禁全部通过；`README.md` 中 W8 行切为 `done`。 | Codex |

## Checkpoint 2026-04-22 16:20

- Week: W8
- Batch: Task 1 Step 1 → Task 1 Step 2
- Completed:
  - 冻结 `octopus-persistence` 首批公共面为 `Database + Migration`，不提前引入 repositories。
  - 新建 `crates/octopus-persistence` 最小骨架，带迁移账本和幂等测试。
  - 同步 `Cargo.toml`、`docs/plans/sdk/README.md`、`docs/plans/sdk/02-crate-topology.md`、W8 计划状态。
- Files changed:
  - `Cargo.toml` (modified)
  - `docs/plans/sdk/README.md` (modified)
  - `docs/plans/sdk/02-crate-topology.md` (modified)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
  - `crates/octopus-persistence/Cargo.toml` (+added)
  - `crates/octopus-persistence/src/lib.rs` (+added)
  - `crates/octopus-persistence/src/database.rs` (+added)
  - `crates/octopus-persistence/src/migrations.rs` (+added)
  - `crates/octopus-persistence/tests/database.rs` (+added)
- Verification:
  - `rg -n "octopus-persistence|SqliteJsonlSessionStore|default-members|5 业务 crate" docs/plans/sdk/{00-overview.md,02-crate-topology.md,README.md} Cargo.toml` → pass
  - `cargo test -p octopus-persistence` → pass
  - `cargo clippy -p octopus-persistence -- -D warnings` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-22 18:05

- Week: W8
- Batch: Task 2 Step 1 → Task 2 Step 2
- Completed:
  - `octopus-platform::runtime_sdk` 改为通过 `Database` 获取连接；`runtime_secret_records` 建表逻辑登记为迁移。
  - `octopus-server` host notifications 改为通过 `Database` 获取连接；`notifications` 建表逻辑登记为迁移。
  - `octopus-platform` / `octopus-server` 新增 `octopus-persistence` 依赖并完成回归验证。
- Files changed:
  - `crates/octopus-platform/Cargo.toml` (modified)
  - `crates/octopus-platform/src/runtime_sdk/mod.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/secret_vault.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs` (modified)
  - `crates/octopus-server/Cargo.toml` (modified)
  - `crates/octopus-server/src/handlers.rs` (modified)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `cargo test -p octopus-platform` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-server` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo clippy -p octopus-platform -p octopus-server -- -D warnings` → pass
  - `rg -n "Connection::open\\(" crates/octopus-platform/src/runtime_sdk` → 0 hits
  - `rg -n "Connection::open\\(" crates/octopus-server/src/{handlers.rs,lib.rs,routes.rs,test_runtime_sdk.rs}` → 0 hits
  - `rg -n "Connection::open\\(" crates/octopus-server/src/workspace_runtime.rs` → 2 inline-test hits only
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-22 21:55

- Week: W8
- Batch: Task 3 Step 1 → Task 3 Step 2
- Completed:
  - `InfraState::open_db()`、`initialize_database()`、seed/load 路径已统一经 `WorkspacePaths::database()` + `octopus-persistence` 取连接和跑 migration。
  - `octopus-infra` 新增 `src/persistence/` 边界，收口 workspace database 入口。
  - 清掉 `octopus-infra/src` 内所有 `Connection::open` 命中；内联测试统一改走 `database().acquire()`。
  - 修正 `tool_catalog_marks_unsupported_mcp_servers_as_attention`，让测试断言回到当前实际“不支持 transport=ws”语义。
- Files changed:
  - `crates/octopus-infra/Cargo.toml` (modified)
  - `crates/octopus-infra/src/lib.rs` (modified)
  - `crates/octopus-infra/src/infra_state.rs` (modified)
  - `crates/octopus-infra/src/workspace_paths.rs` (modified)
  - `crates/octopus-infra/src/persistence/mod.rs` (+added)
  - `crates/octopus-infra/src/persistence/database.rs` (+added)
  - `crates/octopus-infra/src/split_module_tests.rs` (modified)
  - `crates/octopus-infra/src/agent_assets.rs` (modified)
  - `crates/octopus-infra/src/artifacts_inbox_knowledge.rs` (modified)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-infra` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo clippy -p octopus-infra -- -D warnings` → pass
  - `rg -n "Connection::open\\(" crates/octopus-infra/src --glob '!**/tests/**' --glob '!**/test_*.rs' --glob '!**/*tests.rs' --glob '!**/split_module_tests.rs'` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-22 23:05

- Week: W8
- Batch: Task 4 Step 1 → Task 4 Step 2
- Completed:
  - `octopus-core/src/lib.rs` 已按 domain 拆成 `app / host / workspace / model_catalog / capability_management / access_control / runtime_config / runtime_memory / runtime_session / operations` 等子模块。
  - `lib.rs` 收敛为 `mod` + `pub use`，保持既有公共导出面不变，不引入新的裸 public surface。
  - `octopus-core` 新旧文件全部通过 W8 行数守护，`lib.rs` 已降到 80 行以内。
- Files changed:
  - `crates/octopus-core/src/lib.rs` (modified)
  - `crates/octopus-core/src/app.rs` (+added)
  - `crates/octopus-core/src/host.rs` (+added)
  - `crates/octopus-core/src/workspace.rs` (+added)
  - `crates/octopus-core/src/model_catalog.rs` (+added)
  - `crates/octopus-core/src/capability_management.rs` (+added)
  - `crates/octopus-core/src/access_control.rs` (+added)
  - `crates/octopus-core/src/runtime_config.rs` (+added)
  - `crates/octopus-core/src/runtime_memory.rs` (+added)
  - `crates/octopus-core/src/runtime_session.rs` (+added)
  - `crates/octopus-core/src/operations.rs` (+added)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-core` → pass
  - `find crates/octopus-core -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 5 Step 1

## Checkpoint 2026-04-23 01:05

- Week: W8
- Batch: Task 7 Step 1 → Task 7 Step 3
- Completed:
  - `access_control.rs` 已按 defaults / loaders / resolve / summaries / system_roles / service_members / service_governance / tests 拆分，根模块保留稳定入口。
  - `projects_teams.rs` 已按 project、resource、agent+team、workspace admin 与 helper/test 边界拆分；根模块改成 `WorkspaceService` 委托层，子模块落成 `*_impl` 方法。
  - `agent_assets.rs` 已按 parse、builtin catalog、frontmatter/normalize、asset state/id、record/action、runtime doc+persistence、export、tests 拆分，根路径保持稳定。
- Files changed:
  - `crates/octopus-infra/src/access_control.rs` (modified)
  - `crates/octopus-infra/src/access_control/` (+added, split modules and tests)
  - `crates/octopus-infra/src/projects_teams.rs` (modified)
  - `crates/octopus-infra/src/projects_teams/` (+added, split service/helper/test modules)
  - `crates/octopus-infra/src/agent_assets.rs` (modified)
  - `crates/octopus-infra/src/agent_assets/` (+added, split modules and tests)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `cargo fmt --package octopus-infra` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-infra access_control -- --nocapture` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-infra projects_teams -- --nocapture` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-infra agent_assets -- --nocapture` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-infra` → pass
  - `find crates/octopus-infra -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` → only Task 8 residual offenders remain: `resources_skills.rs`, `auth_users.rs`, `artifacts_inbox_knowledge.rs`, `project_tasks.rs`, `agent_bundle/import.rs`
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 8 Step 1

## Checkpoint 2026-04-22 16:47

- Week: W8
- Batch: Task 5 Step 1 → Task 5 Step 3
- Completed:
  - `crates/octopus-server/src/handlers/`、`workspace_runtime/`、`lib.rs` 的拆分已收口，主文件保留组装/导出，resource/runtime 逻辑落到对应子模块。
  - 修复 `lib.rs` helper split 后的可见性断点，把跨模块使用的 auth/audit/http/runtime/session helpers 改回 `pub(crate)`。
  - 清掉 `auth_rate_limit.rs` 中误混入的 audit helper，恢复 `audit_support.rs` 与 rate-limit 模块的真实边界。
  - 补装 worktree 的 pnpm 依赖后，`apps/desktop/test/runtime-store.test.ts` 回归通过。
- Files changed:
  - `crates/octopus-server/src/auth_rate_limit.rs` (modified)
  - `crates/octopus-server/src/audit_support.rs` (modified)
  - `crates/octopus-server/src/http_support.rs` (modified)
  - `crates/octopus-server/src/runtime_support.rs` (modified)
  - `crates/octopus-server/src/session_auth.rs` (modified)
  - `crates/octopus-server/src/lib_tests.rs` (modified)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-server --no-run` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-server` → pass
  - `pnpm install --frozen-lockfile` → pass
  - `pnpm -C apps/desktop exec vitest run test/runtime-store.test.ts` → pass
  - `find crates/octopus-server -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 6 Step 1

## Checkpoint 2026-04-22 17:16

- Week: W8
- Batch: Task 6 Step 1 → Task 6 Step 2
- Completed:
  - `crates/octopus-infra/src/infra_state.rs` 已收敛为根模块，保留状态结构、常量、重导出和定向测试。
  - 新增 `infra_state/{startup,schema_*,loaders_*,defaults,pet,config}.rs`，把 bootstrap、schema/migration、loaders、defaults、pet projection 拆到独立 ownership 模块。
  - 保持 `crate::infra_state::...` 访问路径不变，避免连带改动 `bootstrap.rs`、`agent_assets.rs`、`projects_teams.rs`、`auth_users.rs` 等调用点。
- Files changed:
  - `crates/octopus-infra/src/infra_state.rs` (modified)
  - `crates/octopus-infra/src/infra_state/config.rs` (modified)
  - `crates/octopus-infra/src/infra_state/defaults.rs` (modified)
  - `crates/octopus-infra/src/infra_state/loaders_assets.rs` (+added)
  - `crates/octopus-infra/src/infra_state/loaders_core.rs` (modified)
  - `crates/octopus-infra/src/infra_state/loaders_projects.rs` (modified)
  - `crates/octopus-infra/src/infra_state/loaders_runtime.rs` (+added)
  - `crates/octopus-infra/src/infra_state/pet.rs` (modified)
  - `crates/octopus-infra/src/infra_state/schema_assets.rs` (+added)
  - `crates/octopus-infra/src/infra_state/schema_bootstrap_access.rs` (+added)
  - `crates/octopus-infra/src/infra_state/schema_bootstrap_core.rs` (+added)
  - `crates/octopus-infra/src/infra_state/schema_bootstrap_runtime.rs` (+added)
  - `crates/octopus-infra/src/infra_state/schema_projects.rs` (+added)
  - `crates/octopus-infra/src/infra_state/schema_resources.rs` (+added)
  - `crates/octopus-infra/src/infra_state/schema_runtime.rs` (+added)
  - `crates/octopus-infra/src/infra_state/schema_support.rs` (+added)
  - `crates/octopus-infra/src/infra_state/startup.rs` (+added)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-infra` → pass
  - `find crates/octopus-infra/src/infra_state* -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 7 Step 1

## Checkpoint 2026-04-22 22:17

- Week: W8
- Batch: Task 8 Step 1 → Task 8 Step 2
- Completed:
  - `octopus-infra` 的 residual offenders 已清零，`resources_skills.rs`、`auth_users.rs`、`artifacts_inbox_knowledge.rs`、`project_tasks.rs`、`agent_bundle/import.rs` 全部拆到子模块。
  - `octopus-cli::{automation,workspace}` 已按 command / output / parse / test ownership 拆分，根模块收敛为装配层。
  - `octopus-platform::runtime_sdk::{registry_bridge,config_bridge}` 已按 runtime bridge ownership 拆分，跨 sibling 共用方法统一收口为 `pub(crate)`。
- Files changed:
  - `crates/octopus-infra/src/resources_skills.rs` (modified)
  - `crates/octopus-infra/src/resources_skills/` (+added, split modules and tests)
  - `crates/octopus-infra/src/auth_users.rs` (modified)
  - `crates/octopus-infra/src/auth_users/` (+added, split modules and tests)
  - `crates/octopus-infra/src/artifacts_inbox_knowledge.rs` (modified)
  - `crates/octopus-infra/src/artifacts_inbox_knowledge/` (+added, split modules and tests)
  - `crates/octopus-infra/src/project_tasks.rs` (modified)
  - `crates/octopus-infra/src/project_tasks/` (+added, split modules and tests)
  - `crates/octopus-infra/src/agent_bundle/import.rs` (modified)
  - `crates/octopus-infra/src/agent_bundle/import/` (+added, split modules and tests)
  - `crates/octopus-cli/src/automation.rs` (modified)
  - `crates/octopus-cli/src/automation/` (+added, split modules and tests)
  - `crates/octopus-cli/src/workspace.rs` (modified)
  - `crates/octopus-cli/src/workspace/` (+added, split modules and tests)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge/` (+added, split modules)
  - `crates/octopus-platform/src/runtime_sdk/config_bridge.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/config_bridge/` (+added, split modules)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `cargo fmt --package octopus-cli` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-cli automation -- --nocapture` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-cli workspace -- --nocapture` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-cli` → pass
  - `cargo fmt --package octopus-platform` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-platform registry_bridge -- --nocapture` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-platform config_bridge -- --nocapture` → pass
  - `CARGO_TARGET_DIR=/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target cargo test -p octopus-platform` → pass
  - `find crates/octopus-platform crates/octopus-cli -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - Task 9 Step 1

## Checkpoint 2026-04-22 22:28

- Week: W8
- Batch: Task 9 Step 1
- Completed:
  - W8 Weekly Gate 已全量通过，workspace Rust、desktop test、legacy 目录复核、`runtime/sessions/*.json` 守护与 repo 级 ≤800 行门禁全部收口。
  - `cargo clippy --workspace -- -D warnings` 暴露的 5 处拆分后遗留项已修复：3 处 SQL raw string、2 处 `needless_question_mark`，以及 `project_runtime` helper 可见性收回到文件私有。
  - `docs/plans/sdk/README.md` 的 W8 行已切到 `done`，`00-overview.md` 与 `03-legacy-retirement.md` 已补 weekly gate 变更日志。
- Files changed:
  - `crates/octopus-infra/src/infra_state/schema_bootstrap_access.rs` (modified)
  - `crates/octopus-infra/src/infra_state/schema_bootstrap_core.rs` (modified)
  - `crates/octopus-infra/src/infra_state/schema_bootstrap_runtime.rs` (modified)
  - `crates/octopus-infra/src/projects_teams/service_projects.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/project_runtime.rs` (modified)
  - `docs/plans/sdk/README.md` (modified)
  - `docs/plans/sdk/00-overview.md` (modified)
  - `docs/plans/sdk/03-legacy-retirement.md` (modified)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `cargo test --workspace` → pass
  - `cargo clippy --workspace -- -D warnings` → pass
  - `pnpm -C apps/desktop test` → pass
  - `rg "runtime/sessions/.*\\.json" crates/ --glob '!**/tests/**' --glob '!**/fixtures/**'` → 0 hits
  - `ls crates/ | rg '^(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)$'` → 0 hits
  - `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - W8 complete

## Checkpoint 2026-04-23 03:09

- Week: W8
- Batch: Task 9 Step 1 · post-audit closeout
- Completed:
  - 修复 post-audit 暴露的拆分回归：`octopus-platform::runtime_sdk::secret_vault` 恢复 `Connection` 导入并改为复用传入的 `Database`；`octopus-server` 清掉错误 re-export、补回 `runtime_events` 模块装配、修正 host notification 路径 fallback；`octopus-infra` 的 bootstrap/load/test 路径全部切回当前 `octopus-persistence` API。
  - 删除未挂载的孤儿文件 `crates/octopus-infra/src/persistence/schema.rs` 后，repo 级 Rust `<=800` 行门禁保持 0 命中；`workspace_runtime` 测试装配改为 `tests` + `tests_legacy` 双包装器，恢复拆分后的 server 测试面。
  - W8 计划重新验收通过，本文档状态切回 `done`。
- Files changed:
  - `crates/octopus-platform/src/runtime_sdk/secret_vault.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/builder.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/mod.rs` (modified)
  - `crates/octopus-server/src/lib.rs` (modified)
  - `crates/octopus-server/src/handlers/host.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/mod.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/runtime_sessions.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/tests.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/tests/mod.rs` (modified)
  - `crates/octopus-infra/src/bootstrap.rs` (modified)
  - `crates/octopus-infra/src/infra_state.rs` (modified)
  - `crates/octopus-infra/src/lib.rs` (modified)
  - `crates/octopus-infra/src/persistence/schema.rs` (deleted)
  - `docs/plans/sdk/11-week-8-cleanup-and-split.md` (modified)
- Verification:
  - `cargo test --workspace` → pass
  - `cargo clippy --workspace -- -D warnings` → pass
  - `pnpm -C apps/desktop test` → pass
  - `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - <none>
- Next:
  - <none>
