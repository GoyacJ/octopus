# Digital Workforce Rename Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename the workspace and project agent center display copy from "智能体" to "数字员工" in the desktop frontend without changing internal `agent` contracts.

**Architecture:** Keep routes, stores, schema types, and backend transport untouched. Update only user-visible labels and descriptions in the desktop UI by changing locale copy, navigation fallback labels, and hardcoded helper text in the shared agent center surface. Add a focused regression test that proves the workspace and project entry surfaces render the new wording.

**Tech Stack:** Vue 3, Vite, Pinia, Vue Router, Vue I18n, Vitest

---

### Task 1: Add a failing regression assertion for the renamed entry surfaces

**Files:**
- Modify: `apps/desktop/test/agent-center.test.ts`

**Step 1: Write the failing test**

Add assertions that the workspace and project agent center pages render `数字员工` wording in visible UI text.

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/desktop test -- agent-center.test.ts`
Expected: FAIL because the current UI still renders `智能体` / `Agent` wording.

### Task 2: Update user-visible copy to 数字员工

**Files:**
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/navigation/menuRegistry.ts`
- Modify: `apps/desktop/src/views/agents/AgentCenterView.vue`
- Modify: `apps/desktop/src/views/agents/TeamUnitCard.vue`

**Step 1: Update locale-backed labels**

Rename workspace/project agent-center Chinese copy from `智能体` to `数字员工`, including headers, button labels, empty states, list titles, and workspace/project descriptions.

**Step 2: Update fallback navigation labels**

Change menu fallback labels so the UI still shows the right wording even when locale resolution is unavailable.

**Step 3: Update hardcoded helper strings**

Replace remaining hardcoded `Agent` / `Agent Team` helper text in the shared agent center surface with `数字员工` semantics, while keeping internal types and identifiers unchanged.

### Task 3: Verify the rename end-to-end

**Files:**
- Verify: `apps/desktop/test/agent-center.test.ts`
- Verify: `apps/desktop/test/desktop-i18n.test.ts`

**Step 1: Run the focused regression tests**

Run: `pnpm -C apps/desktop test -- agent-center.test.ts desktop-i18n.test.ts`
Expected: PASS

**Step 2: Run typecheck for desktop frontend**

Run: `pnpm -C apps/desktop typecheck`
Expected: PASS
