# 04 · 上下文锚定规范

> 状态：Accepted · 本文是**防 AI 幻觉**的硬性规则
> 适用对象：所有 Codex AI 会话 + 任务卡撰写者

---

## 1. 为什么需要上下文锚定

AI 在以下场景容易"幻觉"（生成与 SPEC 不一致的代码）：

- 仅凭任务卡描述"猜"trait 签名 → 漏字段 / 改类型 / 误参数顺序
- 仅凭 crate 命名"猜"功能 → 实现错位 / 越权
- 仅凭"经验"生成测试 fixture → 用例与 SPEC 关心的边界条件无关
- 上下文窗口被无关代码占满 → 关键约束被 AI"压缩遗忘"

**解决方案**：每个 AI 会话开始前，**强制读取**精确到行号片段的 SPEC + ADR + 依赖产物，并保持 context "新鲜"。

---

## 2. 强制读取协议（每个 Codex 会话开局）

### 2.1 读取顺序（不可调换）

```text
1. 任务卡（milestones/Mx-Tyy 段）  ← 派发时的入口
2. SPEC 锚点列表中的所有文件 + 行号片段 ← 必读
3. ADR 锚点列表中的所有 ADR 全文        ← 必读
4. 依赖任务卡的 PR diff（git show）     ← 必读
5. （可选）相关参考资料：reference-analysis/
6. （可选）当前 crate 的 Cargo.toml + 已有 src/lib.rs
```

### 2.2 读取工具规约

| 工具 | 用途 | 注意 |
|---|---|---|
| `Read` | 精确读取文件指定行号片段（`offset` + `limit`）| **必须用 offset+limit 而非全文读**，节省 context |
| `Grep` | 验证 SPEC 中是否存在某 trait 签名 | 用 `output_mode=files_with_matches` 减少噪声 |
| `Glob` | 找具体的 SPEC 文件 | 仅在路径不确定时使用 |
| `Shell git show <commit>` | 读取依赖任务卡的 PR diff | 不要全 clone history |

**严禁**：

- 用 `SemanticSearch` 替代精确读取（语义搜索不能替代行号精读）
- 直接复制粘贴 SPEC 全文进 prompt（context 过载）
- 跳过 ADR 直接看 crate SPEC（ADR 才是设计意图源）

---

## 3. SPEC 锚点格式规范

任务卡中的 SPEC 锚点至少必须精确到两段信息；承载 trait / enum / event / schema / provider / hook / session 行为签名的锚点必须带行号区间：

```markdown
- 文件路径：docs/architecture/harness/crates/harness-permission.md
- 章节号：§3.1
- 行号区间：L147-L218（签名类锚点必填）
```

**示例（好）**：

```markdown
**SPEC 锚点**：
- `docs/architecture/harness/crates/harness-permission.md` §3.1（DirectBroker，L147-L218）
- `docs/architecture/harness/api-contracts.md` §8.1（PermissionBroker trait，L420-L455）
- `docs/architecture/harness/permission-model.md` §5.1（DirectBroker 用例图，L242-L292）
```

**示例（坏）**：

```markdown
**SPEC 锚点**：参见 harness-permission 文档
```

**强制 grep 自检模板**（任务卡撰写者必跑、CI 验证）：

```bash
failed=0
# Maintainer 在任务卡进入派发窗口前，把对应 ID 加入本列表。
line_required_cards='M1-T07|M2-S01|M3-S01|M3-S02'

for f in docs/plans/harness-sdk/milestones/*.md; do
  awk -v cards="$line_required_cards" '
    /^### / { in_card = ($0 ~ cards); in_spec = 0 }
    in_card && /^\*\*SPEC 锚点\*\*/ { in_spec = 1; next }
    in_spec && (/^---/ || /^### / || /^\*\*/) { in_spec = 0 }
    in_card && in_spec && /^- `docs\/architecture\/harness/ && $0 !~ /L[0-9]+/ {
      print FILENAME ":" FNR ": missing line range: " $0
      failed = 1
    }
    END { exit failed }
  ' "$f" || failed=1
done

exit "$failed"
```

**渐进收敛策略**：M0/M1/M2/M3 中承载 trait / enum / event / schema / provider / hook / session 行为签名的任务卡必须带行号。内部 plan 锚点、ADR 锚点、依赖 PR 锚点可用文件 + 章节号；PR checklist 必须声明已确认其精度足够。进入派发窗口前，maintainer 必须把对应任务卡 ID 加入上方 `line_required_cards`，让 CI 变成阻断项。

---

## 4. ADR 锚点的特殊性

ADR 不只是"决策记录"，是**设计意图的根基**。任务卡的 ADR 锚点必须满足：

- 列出所有**直接关联**的 ADR 编号（不只是 D 文档头注引用的）
- 标注**Status**（Accepted / Superseded / Reverted）
- 如 ADR 与当前任务卡有冲突，明确标记

**示例**：

```markdown
**ADR 锚点**（必读）：
- ADR-007（permission-events，**Accepted**）— 决议审批事件化
- ADR-013（integrity-signer，**Accepted**）— 决议 IntegritySigner 与 ManifestSigner 隔离
- ADR-018（no-loop-intercepted-tools，**Accepted 反向决议**）— 不引入 Loop-Intercepted Tools，仅供边界确认
```

---

## 5. 上下文 budget

Codex 单次会话 context 有限。上下文锚定必须**精打细算**：

| 内容 | 推荐占用 | 备注 |
|---|---|---|
| 任务卡本体 | < 500 tokens | 简洁、必有信息全 |
| SPEC 锚点（精读片段） | < 4000 tokens | 多个片段拼接，每个用 `Read offset+limit` |
| ADR 全文（关键 1-2 篇） | < 3000 tokens | 长 ADR 仅读结论段 |
| 依赖 PR diff | < 2000 tokens | 仅读改动文件 |
| 当前 crate 已有代码 | < 1000 tokens | 主要看 lib.rs / Cargo.toml |
| 通用规则（00-strategy / 03-quality-gates）| < 500 tokens | 引用即可，不全文复制 |
| **小计** | **< 11k tokens** | 留出 60% 给生成 |

---

## 6. 防幻觉检查表（AI 自检）

每张任务卡完成后，AI 在 PR description 中必须勾选：

```markdown
## 防幻觉自检
- [ ] 我已读取所有带行号的 SPEC 锚点片段；无行号的内部 plan / ADR / PR 锚点已按章节全文确认
- [ ] 我已读取所有 ADR 锚点全文
- [ ] 我已读取所有依赖任务卡的 PR diff
- [ ] 我的实现 trait 签名与 SPEC 逐字一致（已用 grep 验证）
- [ ] 我的错误类型沿用 harness-contracts 既有族（未自创）
- [ ] 我的测试用例覆盖 SPEC 列出的边界条件
- [ ] 如发现 SPEC 不清/有歧义，我已停下并标记 [SPEC-CLARIFY-REQUIRED]
```

如任意一项未勾选 → PR 视为不完整 → reviewer 直接 reject。

---

## 7. 跨任务卡的"知识传递"

任务卡之间的契约通过 PR description 传递：

| 来源 | 接收者 | 媒介 |
|---|---|---|
| 上一卡的"对外契约"段 | 下一卡 | 任务卡引用 PR # |
| 已合入卡修改的公开 API | 后续所有卡 | `cargo doc` + 接口的 SPEC 引用 |
| 临时约定（如临时 stub） | 标记于任务卡的"已知缺陷"段 | 后续卡明确补全 |

**禁止**：

- 通过 chat 私聊向 AI 传递任务卡之外的信息
- 在 issue / wiki / 飞书文档传递架构相关约定（必须回流到 SPEC 或 ADR）

---

## 8. 防"自由发挥"的硬性手段

AI 容易在以下边界自由发挥，必须以**任务卡的禁止行为**段堵死：

| 自由发挥点 | 后果 | 任务卡如何堵 |
|---|---|---|
| 公开 API 命名 | 用户层不一致 | 任务卡列"对外契约"段 |
| 错误类型 | 错误处理碎片化 | 任务卡禁止行为列"不得自定义新错误家族" |
| feature 启用 | feature 矩阵失控 | 任务卡明确列"启用哪些 feature" |
| 跨 crate use | 违反 D2 模块边界 | 任务卡列"禁止 use 哪些 crate" |
| 模块组织 | 命名漂移 | SPEC 已规定模块结构，任务卡复述 |
| 测试 mock 实现 | 跨 crate 共用错乱 | 任务卡明确 mock 位置（src/mock.rs vs testing.rs）|

---

## 9. 例外路径：AI 发现 SPEC 缺陷

AI 阅读 SPEC + ADR 后，如发现：

- trait 签名内部矛盾（如 `async fn` 但返回非 Future）
- 字段命名与同 crate 其它字段冲突
- ADR 之间互相矛盾
- 类型 ID 与 D2 §10 例外登记不符

**正确处理**：

```markdown
## [SPEC-CLARIFY-REQUIRED]

### 矛盾点
- `harness-permission.md §3.1 L150` 定义 DirectBroker 回调返回 `Decision`
- `harness-contracts.md §3.2 L88` 定义 Decision 包含 4 变体
- 但 `permission-model.md §5.1 L245` 示例中使用 `DecisionKind`（仅 2 变体）

### 影响
若以 permission-model 为准 → 实现签名错误
若以 harness-permission 为准 → 示例代码不可编译

### 建议
请 maintainer 决定：
1. 修订 permission-model.md 示例（推荐）
2. 在 harness-contracts 增加 DecisionKind alias

本任务卡暂停，等候裁决。
```

**禁止**：

- 自行选择"看似合理"的版本然后实现
- "先实现再补"
- 在任务卡之外的渠道（chat / issue）讨论

---

## 10. 索引

- **任务卡模板** → [`02-task-template.md`](./02-task-template.md)
- **执行策略** → [`00-strategy.md`](./00-strategy.md)
- **质量闸门** → [`03-quality-gates.md`](./03-quality-gates.md)
- **架构基线** → [`docs/architecture/harness/README.md`](../../architecture/harness/README.md)
