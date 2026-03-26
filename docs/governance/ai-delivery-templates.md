# AI Delivery Templates

## 1. 目的

本文件定义 AI 在各阶段必须优先使用的标准模板。  
目标是让设计说明、契约变更、实现说明、验证说明、交付说明结构化、可检视、可复用。

这些模板与 `ai-phase-gates.md` 配合使用。

---

## 2. 使用规则

- 每次任务至少应产出 `Task Definition` 与 `Delivery Note`
- 涉及实现时，应补 `Implementation Summary` 与 `Verification`
- 涉及 schema / 公共接口 / 事件 / 兼容性时，应补 `Contract Change`
- 涉及较大结构或分层调整时，应补 `Design Note`
- 涉及架构结论时，应补 ADR，而不是把长期决策埋在任务说明中
- 产品语义、正式范围、release slicing 变化进入 `docs/product/PRD.md`，不要把任务级验收或局部设计塞进 PRD
- 任务级目标、验收标准、非功能约束与 MVP 边界应记录在 task package 与本文件模板中
- UI / Surface 任务若需要冻结视觉语法，应引用或更新 `docs/architecture/VISUAL_FRAMEWORK.md`；非 UI 工作不受其阻塞

---

## 3. Task Definition

用于：Phase 0

```md
## Task Definition
- Goal:
- Scope:
- Out of Scope:
- Acceptance Criteria:
- Non-functional Constraints:
- MVP Boundary:
- Human Approval Points:
- Source Of Truth Updates:
- Affected Modules:
- Affected Layers:
- Risks:
- Validation:
```

### 填写要求

- `Goal`：一句话说明要解决什么问题
- `Scope`：列出本次明确要做的事
- `Out of Scope`：列出本次明确不做的事
- `Acceptance Criteria`：列出完成判定；任务级验收默认写在这里或 task package，不写进 PRD
- `Non-functional Constraints`：列出性能、安全、可靠性、可维护性等约束；无特殊要求时写 `None`
- `MVP Boundary`：说明本次最小可交付切片与明确不跨出的边界
- `Human Approval Points`：列出必须由人确认、审批或拍板的节点；没有则写 `None`
- `Source Of Truth Updates`：说明本次结论应更新 PRD、task package、schema / contract、ADR 或其他 owner doc 的哪一层
- `Affected Modules`：列出受影响模块
- `Affected Layers`：列出受影响层
- `Risks`：说明风险点
- `Validation`：说明如何证明完成

---

## 4. Design Note

用于：Phase 1

```md
## Design Note
- Problem:
- Goal:
- Acceptance Criteria:
- Non-functional Constraints:
- MVP Boundary:
- Layer Placement:
- Module Boundaries:
- Inputs:
- Outputs:
- State Transitions:
- Error Handling:
- Tech Stack Decision:
- Visual Framework Impact:
- Human Approval Points:
- Reused Components:
- New Abstractions:
- Trade-offs:
- Test Strategy:
- ADR Needed:
```

### 填写要求

- `Acceptance Criteria`：说明设计如何对应任务级完成判定
- `Non-functional Constraints`：说明关键 NFR 如何落到结构、依赖或验证策略
- `MVP Boundary`：说明本次设计只覆盖的最小切片
- `Layer Placement`：说明代码应位于哪一层
- `Module Boundaries`：说明模块职责边界
- `State Transitions`：涉及状态机时必须填写
- `Error Handling`：说明错误路径与边界处理
- `Tech Stack Decision`：仅在新增或变更技术栈时填写，并说明理由
- `Visual Framework Impact`：仅 UI / Surface 任务填写，说明是否依赖 `VISUAL_FRAMEWORK.md`
- `Human Approval Points`：说明哪些设计决定或高风险动作必须由人确认
- `New Abstractions`：新增抽象必须解释理由
- `ADR Needed`：说明是否需要 ADR

---

## 5. Contract Change

用于：Phase 2

```md
## Contract Change
- Change Type:
- New / Updated Schemas:
- New / Updated Commands:
- New / Updated Queries:
- New / Updated Events:
- New / Updated DTOs:
- Compatibility Impact:
- Affected Consumers:
- Migration Notes:
- Generation Impact:
- Open Questions:
```

### `Change Type` 可选值

- Schema
- Public API
- Event
- Internal Interface
- Persistence Model
- Cross-language Contract

### 填写要求

- `Compatibility Impact`：必须说明是否兼容
- `Affected Consumers`：说明受影响调用方
- `Migration Notes`：需要迁移时必须填写
- `Generation Impact`：说明是否影响代码生成

---

## 6. Implementation Summary

用于：Phase 3

```md
## Implementation Summary
- Goal:
- Files Added:
- Files Changed:
- Files Removed:
- Structure Decision:
- Why This Structure:
- Reused Patterns:
- New Dependencies:
- Error Handling Strategy:
- Deferred Items:
- Non-goals Preserved:
```

### 填写要求

- `Structure Decision`：说明模块拆分方式
- `Why This Structure`：解释为何按此方式组织
- `Deferred Items`：说明暂未处理的部分
- `Non-goals Preserved`：说明哪些非目标仍被保持

---

## 7. Verification

用于：Phase 4

```md
## Verification
- Unit Tests:
- Integration Tests:
- Contract Tests:
- Failure Cases:
- Boundary Cases:
- Manual Verification:
- Static Checks:
- Remaining Gaps:
- Confidence Level:
```

### 填写要求

- `Failure Cases`：必须说明失败路径如何验证
- `Boundary Cases`：必须说明边界条件如何验证
- `Remaining Gaps`：说明还没覆盖的点
- `Confidence Level`：建议填写 High / Medium / Low

---

## 8. Delivery Note

用于：Phase 5

```md
## Delivery Note
- What Changed:
- Why:
- User / System Impact:
- Risks:
- Rollback Notes:
- Follow-ups:
- Docs Updated:
- Tests Included:
- ADR Updated:
- Temporary Workarounds:
```

### 填写要求

- `What Changed`：概述本次交付
- `User / System Impact`：说明对外影响
- `Risks`：说明风险点
- `Rollback Notes`：说明可回滚方式或注意事项
- `Temporary Workarounds`：如果存在临时方案必须填写

---

## 9. ADR Trigger Note

用于：需要新增 ADR 时的快速记录

```md
## ADR Trigger Note
- Decision Topic:
- Why Existing Rules Are Insufficient:
- Options Considered:
- Proposed Decision:
- Consequences:
- Related Modules:
```

---

## 10. Bug Fix Template

用于：缺陷修复任务

```md
## Bug Fix Note
- Symptom:
- Root Cause:
- Scope of Impact:
- Fix Strategy:
- Regression Risk:
- Tests Added:
- Follow-ups:
```

---

## 11. Refactor Template

用于：重构任务

```md
## Refactor Note
- Current Problem:
- Refactor Goal:
- Behavior Preserved:
- Structural Changes:
- Risks:
- Tests Protecting Behavior:
- Follow-ups:
```

---

## 12. Schema Proposal Template

用于：较大 schema 设计任务

```md
## Schema Proposal
- Domain Object:
- Purpose:
- Core Fields:
- Invariants:
- Lifecycle / States:
- Commands:
- Events:
- Compatibility Considerations:
- Cross-language Consumers:
- Open Questions:
```

---

## 13. 使用建议

### 小型任务最少产物

建议至少包含：

- `Task Definition`
- `Implementation Summary`
- `Verification`
- `Delivery Note`

### 中大型任务建议产物

建议包含：

- `Task Definition`
- `Design Note`
- `Contract Change`
- `Implementation Summary`
- `Verification`
- `Delivery Note`
- ADR（如适用）

---

## 14. 一句话总纲

任何任务都不应只交付代码。  
AI 必须同时交付与阶段相匹配的结构化说明，确保后续开发者可以理解目标、边界、契约、实现与验证结论。
