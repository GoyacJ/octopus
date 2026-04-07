# Conversation Composer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rework the project conversation bottom composer into a compact integrated chat composer that matches the provided reference while preserving the existing runtime session behavior.

**Architecture:** Keep the runtime submit flow in `ConversationView.vue` unchanged and only reorganize the presentation layer. Validate the new structure with the existing conversation surface integration test so the send flow and selector wiring remain intact.

**Tech Stack:** Vue 3, Vite, Pinia, Vue Router, Vue I18n, Vitest, Tailwind CSS, `@octopus/ui`

---

### Task 1: Add a failing integration assertion for the new composer shell

**Files:**
- Modify: `apps/desktop/test/conversation-surface.test.ts`

**Step 1: Write the failing test**

Add assertions for:
- `[data-testid="conversation-composer"]`
- `[data-testid="conversation-model-select"]`
- `[data-testid="conversation-permission-select"]`
- `[data-testid="conversation-actor-select"]`
- `[data-testid="conversation-send-button"]`

**Step 2: Run test to verify it fails**

Run: `pnpm --filter octopus-desktop test -- conversation-surface.test.ts`

Expected: FAIL because the new composer test ids do not exist yet.

### Task 2: Rebuild the composer layout without changing runtime behavior

**Files:**
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`

**Step 1: Keep runtime logic unchanged**

Preserve:
- `ensureRuntimeSession`
- `submitRuntimeTurn`
- `handleComposerKeydown`

**Step 2: Replace the current form card**

Implement:
- a two-layer rounded composer shell
- a transparent textarea treatment inside the shell
- a bottom toolbar that contains model, permission, and agent selectors
- an icon-only send button
- a small localized shortcut hint

**Step 3: Run the targeted test**

Run: `pnpm --filter octopus-desktop test -- conversation-surface.test.ts`

Expected: PASS

### Task 3: Verify typing and broader desktop tests touched by the change

**Files:**
- No additional source files expected

**Step 1: Run typecheck**

Run: `pnpm --filter octopus-desktop typecheck`

Expected: PASS

**Step 2: Run a focused desktop UI test sweep**

Run: `pnpm --filter octopus-desktop test -- conversation-surface.test.ts trace-view.test.ts`

Expected: PASS
