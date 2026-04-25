# 02 · AI 任务卡模板

> 状态：Accepted · 本文是任务卡的**唯一标准模板**，里程碑文件中所有任务卡必须按本模板撰写
> 适用对象：Codex AI（执行）+ 人类（撰写 / reviewer）

---

## 1. 任务卡定义

**任务卡**（Task Card）是一个 AI 单次会话的最小工作单元，描述：

- 这次会话要产出什么
- 它依赖什么、产物长什么样
- 通过什么验收
- AI 在哪里取上下文

每张任务卡**唯一 ID**：`Mx-Tyy[-子任务序号]`（如 `M2-T03` / `M3-T05a`）。

---

## 2. 任务卡标准模板

每张任务卡在里程碑文件中以如下 YAML-like 块呈现：

```markdown
### M2-T03 · `octopus-harness-permission` · DirectBroker 实现

| 字段 | 值 |
|---|---|
| **状态** | 待派发 / 已派发 / 评审中 / 已合入 / 作废 |
| **依赖** | M1 完成 · M2-T02（`octopus-harness-permission` crate 骨架） |
| **可并行** | 与 M2-T01/T05/T08/T11/T15 并行（不同 crate） |
| **预期 diff** | ≤ 250 行（含测试） |
| **预期工时** | AI 单会话 ~30 min；人类评审 ~15 min |

**SPEC 锚点**（必读，按行号片段读取）：
- `docs/architecture/harness/crates/harness-permission.md` §3.1（DirectBroker 定义，约 L147-L218）
- `docs/architecture/harness/api-contracts.md` §8.1（PermissionBroker trait）
- `docs/architecture/harness/permission-model.md` §5.1（DirectBroker 用例图，已修订带 PermissionContext）

**ADR 锚点**：
- `docs/architecture/harness/adr/0007-permission-events.md`（审批事件化）
- `docs/architecture/harness/adr/0013-integrity-signer.md`（IntegritySigner 边界，与 DirectBroker 隔离）

**前置任务产物**（必读 PR）：
- M2-T02 PR #xxx：`octopus-harness-permission` crate 空骨架 + Cargo.toml feature 矩阵

**预期产物**：
- `crates/octopus-harness-permission/src/broker/direct.rs`（新文件，~120 行）
- `crates/octopus-harness-permission/src/broker/mod.rs`（pub use）
- `crates/octopus-harness-permission/src/lib.rs`（添加 `pub mod broker`）
- `crates/octopus-harness-permission/tests/direct_broker.rs`（≥ 3 用例：同步回调成功 / 拒绝 / 超时）
- `crates/octopus-harness-permission/Cargo.toml`（feature `interactive` 启用）

**对外契约**（其他任务卡可引用）：
- `pub struct DirectBroker<F>` 类型签名公开
- `impl<F> PermissionBroker for DirectBroker<F>` 满足 SPEC §3.1 完整接口
- `DirectBroker::new(callback: F) -> Self` 公开构造函数

**关键不变量**：
- `decide()` 必须传入 `PermissionContext`（含 `tenant_id / session_id / run_id`）—— P1-5 修订强制
- 同步回调失败 → 默认 Deny（Fail-Closed，P6 原则）
- 不允许在 broker 内部维护任何跨调用状态（无状态契约）

**禁止行为**：
- 不得引入 `dep:octopus-harness-model`（除非启用 `auto-mode` feature；本卡不启用）
- 不得改动 `harness-contracts` 的 PermissionBroker trait 签名（如发现 SPEC bug，按铁律 1 例外路径处理）

**验收命令**（5 道闸门，必须全部通过）：

```bash
cargo fmt --check -p octopus-harness-permission
cargo check  -p octopus-harness-permission --features interactive
cargo clippy -p octopus-harness-permission --features interactive --all-targets -- -D warnings
cargo test   -p octopus-harness-permission --features interactive
cargo deny check
```

**SPEC 一致性自检**（PR 提交前 AI 必须运行）：

```bash
# 1. trait 签名是否与 SPEC 完全一致
grep -E '^pub (async )?fn decide' crates/octopus-harness-permission/src/broker/direct.rs

# 2. 是否漏 PermissionContext 参数
grep 'PermissionContext' crates/octopus-harness-permission/src/broker/direct.rs

# 3. 是否引入了禁止的依赖
! grep 'octopus-harness-model' crates/octopus-harness-permission/Cargo.toml
```

**PR 描述模板**：

```markdown
## [M2-T03] octopus-harness-permission · DirectBroker

### SPEC 锚点
- `harness-permission.md §3.1`
- `api-contracts.md §8.1`
- `permission-model.md §5.1`（v1.8.1 修订版 PermissionContext）

### 关键不变量
- ✅ `decide(req, ctx)` 签名与 SPEC 一致
- ✅ Fail-Closed 默认拒绝（broker 自身故障时返回 Deny）
- ✅ 无依赖 `harness-model`

### 闸门日志
<贴 5 道闸门输出（折叠）>

### 测试覆盖
- 同步回调返回 Allow → 主流程通过 ✓
- 同步回调返回 Deny → ToolError::PermissionDenied ✓
- 回调 panic → broker 转 Deny + 日志 ✓
- PermissionContext 缺 tenant_id → 编译期拒绝 ✓
```
```

---

## 3. Codex Prompt 启动框架

每个 Codex 会话开启时，使用如下 Prompt 框架（人类派发任务时复制粘贴）：

````markdown
你是 Octopus Harness SDK 的实现 AI。请严格按 vibecoding 五条铁律工作。

## 当前任务卡

[此处粘贴任务卡完整内容]

## 工作步骤（必须严格遵守）

1. **读取 SPEC 锚点**：使用 `Read` 工具读取任务卡列出的所有 SPEC + ADR 行号片段
2. **读取依赖产物**：使用 `Read` 工具读取依赖任务卡的 PR diff（git show <pr-commit>）
3. **检查工作树**：确认当前在 `feature/Mx-Tyy-<slug>` 分支，从 `main` 分出
4. **实现 + 测试同步**：按"预期产物"清单生成所有文件；测试与实现同 PR
5. **本地自检**：跑 5 道闸门 + SPEC 一致性自检，全绿才能继续
6. **PR 提交**：按 PR 描述模板填写

## 强制规则

- 如发现 SPEC 中 trait 签名或字段名不清/有歧义 → **停下**，在最后输出 `[SPEC-CLARIFY-REQUIRED]` 标记 + 具体歧义点，**不要凭猜测继续**
- 如自检任意一步失败 → **停下**，输出失败日志 + 不要"打补丁式"修复（铁律 3）
- 如本任务卡 diff 超 500 行 → **停下**，输出"任务卡需拆分"建议
- 不允许修改任务卡未列出的文件
- 不允许引入任务卡未声明的依赖

## 输出格式

完成后输出：

```
## 实现摘要
- 创建文件：N 个
- 修改文件：M 个
- 总 diff 行数：X
- 测试用例数：Y

## 闸门日志
[5 道闸门完整输出，含 cargo deny]

## SPEC 一致性自检
[每条 grep 命令输出]

## PR description
[按模板填写]
```
````

---

## 4. 任务卡 ID 命名规范

| 格式 | 示例 | 含义 |
|---|---|---|
| `Mx-Tyy` | `M2-T03` | 第 x 里程碑第 yy 张普通任务卡 |
| `Mx-Tyy[a-z]` | `M3-T05a` | 任务卡因 diff 过大被拆分 |
| `Mx-Pyy` | `M9-P02` | POC 任务卡（仅 M9）|
| `Mx-Gyy` | `M3-G01` | Review Gate 检查任务卡 |
| `Mx-Hyy` | `M0-H01` | Hot-fix（紧急修复）任务卡 |

---

## 5. 任务卡状态机

```text
   待派发  ──派发──→  已派发  ──开发完毕──→  评审中
     ▲                  │                    │
     │                  │                    ├──通过──→  已合入  ──里程碑收尾──→  已归档
     │                  │                    │
     └─作废+重写─────────┴──────不通过──────────┘
```

| 状态 | 触发 | 谁负责 |
|---|---|---|
| **待派发** | 任务卡刚写好 | Maintainer |
| **已派发** | Codex 会话启动 | AI |
| **评审中** | PR 已开 | Reviewer |
| **已合入** | PR merged 到 main | Maintainer 更新 roadmap 进度 |
| **作废** | 任意阶段失败 → reset | Maintainer 决定是否重发新卡 |
| **已归档** | 里程碑结束 | 里程碑 reviewer |

---

## 6. 任务卡撰写者注意事项

如你（人类）正在为新任务卡补充内容：

1. **SPEC 锚点必精确到行号片段**：不要写 "见 harness-tool.md"，要写 "harness-tool.md §3.5 (L420-L455)"
   - **强制 grep 自检**（撰写完后必跑）：
     ```bash
     # 任务卡的每条 SPEC 锚点必须含 "L<digit>" 行号
     awk '/SPEC 锚点/,/^---|^### /' docs/plans/harness-sdk/milestones/<file>.md \
         | grep -E '^- ' \
         | grep -vE 'L[0-9]+|§[0-9]+\.[0-9]+\.[0-9]+|（必读' \
         && echo "FAIL: 锚点缺行号片段"
     ```
   - 抽样补行号优先级（按风险）：M0-T01.5（platform 反向解耦）/ M1-T07（Redactor）/ M2-T01-05（model）/ M3-T01-T05（tool）/ M3-T11-T15（context）/ M4-T11-T18（mcp）。其他卡可允许仅章节号，但必须在 PR 评审 checklist 中显式勾选"已确认 SPEC 锚点精度可接受"。
2. **预期产物清单必完整**：列出所有文件路径，AI 不会自行决定文件位置
3. **关键不变量必显式**：哪怕看似显然（如 Fail-Closed），也要写出来防止 AI 偷工
4. **禁止行为必兜底**：列出常见的踩坑（错误依赖、错误 use 路径等）
5. **diff 上限须保守**：估不准就拆，宁拆勿合
6. **验收命令必精确到 feature**：跨 feature 的命令要展开
7. **cargo deny 例外不要复述**（实施前评估 P2-3）：
   - 已登记的 D2 §10 例外（`auto-mode / redactor / subagent-tool` 等）由 `03-quality-gates.md §5.3` 与 `module-boundaries.md §10` 集中维护
   - 任务卡内**仅引用行号**，不重写 deny 命令矩阵：
     ```markdown
     **cargo deny 矩阵**：见 `03-quality-gates.md §5.3`，本卡未引入新破窗 feature。
     ```
   - 如本卡引入了 D2 §10 之外的破窗 → **必须先开 ADR 修订 §10**，再写任务卡（铁律 1）

---

## 7. 反例：写得不好的任务卡

```markdown
### M2-T03 · 实现 DirectBroker

实现 DirectBroker，参考 SPEC。要能跑通用例。
```

**问题**：

- 没列 SPEC 锚点（AI 会去全文搜，浪费上下文）
- 没列预期产物（AI 自由决定文件位置 → 不一致）
- 没列禁止行为（AI 可能引入跨 crate 依赖）
- 没列验收命令（AI 不知道用什么 feature 跑测试）
- 没列关键不变量（AI 可能漏 PermissionContext）

**修复**：用 §2 模板补齐所有字段。

---

## 8. 索引

- **执行策略** → [`00-strategy.md`](./00-strategy.md)
- **质量闸门** → [`03-quality-gates.md`](./03-quality-gates.md)
- **上下文锚定** → [`04-context-anchoring.md`](./04-context-anchoring.md)
