# Claude-Inspired Project Conversation Deliverable Workbench Design Spec

**Goal:** Define the design philosophy, visual style, and interaction rules for the Claude-inspired `Project -> Conversation -> Deliverable` workbench so Codex can implement the desktop chat experience with a consistent Notion-style language instead of inventing page-local UI.

**Scope:** This document applies to the conversation surface, the right-side deliverable workbench, the project-level deliverables page, and the related routing, list/detail, and preview states that support generated outputs.

**Primary source of truth:** This document extends, but does not replace, [DESIGN.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/design/DESIGN.md). If a rule conflicts, `docs/design/DESIGN.md` wins.

---

## 1. Design Thesis

This feature should feel like a calm workbench, not like a flashy AI chat product.

The design target is the *idea* behind Notion, not a pixel copy of Notion:

- content is always more important than chrome
- structure is visible but quiet
- users feel they are working inside one coherent system, not jumping between special-purpose pages
- controls are present when needed, but they do not dominate the canvas
- outputs are treated as working materials, not throwaway chat attachments

For Octopus, that means:

- the message stream is the narrative of work
- the right rail is the active work surface for the selected deliverable
- project pages are calm index surfaces over the same objects, not alternate visual dialects
- trace, approvals, and runtime machinery are available, but visually secondary

## 2. Notion Design Principles To Preserve

### 2.1 Calm Before Clever

- The UI should feel steady, quiet, and legible before it feels novel.
- Motion, elevation, and color exist to support orientation, not delight for its own sake.
- The user should never feel that the interface is advertising itself.

### 2.2 Content-First Hierarchy

- The generated deliverable is the highest-value object once it exists.
- Metadata, provenance, and tooling stay nearby, but they should not outrank the deliverable body.
- Long text, code, and structured outputs should read like real documents, not message fragments.

### 2.3 One Workbench Language

- Conversation, deliverable preview, project list/detail, knowledge promotion, and ops views must feel like one product.
- Do not create a special “AI mode” visual branch with different shadows, colors, radii, or component behavior.

### 2.4 Progressive Disclosure

- Keep advanced execution detail one layer deeper than the main output flow.
- The first thing users should see is what was produced.
- Trace, workers, mailbox state, and approval detail belong behind `Ops`, not in the default attention path.

### 2.5 Direct Manipulation

- Users should feel they are selecting, reviewing, and editing a deliverable directly.
- A deliverable is not just linked text in the transcript. It must behave like a first-class document with preview, version history, and explicit actions.

### 2.6 Dense But Breathing

- The product can be information-dense, but spacing and typography must still create relief.
- Avoid oversized hero space, but also avoid compressed control clusters that feel like IDE chrome.

## 3. Visual Language

Use the exact desktop language already defined in [DESIGN.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/design/DESIGN.md):

- warm neutral canvases
- one accent blue
- whisper borders before fills
- restrained radii
- subtle layered shadows
- sans-only UI typography

### 3.1 Color Behavior

- The shell, page, and right rail stay neutral-first.
- Accent blue is used for:
  - active tabs
  - selected rows
  - focused controls
  - primary actions such as `Save version`
- Warning, success, and danger colors are limited to approval state, promotion state, validation, and run state callouts.
- Do not color-code whole panes by mode. `Deliverable`, `Context`, and `Ops` must all live in the same base palette.

### 3.2 Border And Elevation

- Prefer `1px` whisper borders to large fill blocks.
- Use inset sectioning and quiet dividers to separate subareas.
- The right rail is a panel surface, not a floating card.
- Deliverable preview and version list may use nested panel sections, but never decorative card-on-card stacking.

### 3.3 Typography

- Deliverable titles follow card-title scale, not billboard scale.
- Version metadata, provenance, and side labels use caption or UI-label scale.
- The actual deliverable content may use content-appropriate typography:
  - prose: body
  - code or JSON: mono only inside the preview/editor surface
- Message chrome, page chrome, and sidebar chrome stay in the global sans stack.

### 3.4 Motion

- Tabs, selection changes, save-state transitions, and preview swaps use short opacity or subtle translate transitions only.
- No sliding drawers, elastic tab indicators, or content zoom effects.
- Save and promote state changes should feel immediate and quiet.

## 4. Page And Surface Model

### 4.1 Conversation Page

The conversation page remains a two-column workbench:

- **main column:** message stream plus composer
- **right column:** deliverable workbench

The main column is for intent, narrative, and execution feedback.
The right column is for inspecting and acting on the currently selected deliverable.

The right column should usually be open when a deliverable exists.

### 4.2 Right Rail Modes

The right rail has exactly three top-level modes:

- `Deliverable`
- `Context`
- `Ops`

These are not equal in emphasis.

Default behavior:

- if no deliverable is selected yet, default to `Context`
- once a deliverable exists and is selected, default to `Deliverable`
- `Ops` is explicitly visited, not the default resting state

### 4.3 Project Deliverables Page

This page should use the list/detail archetype from [DESIGN.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/design/DESIGN.md):

- left pane: deliverable list with filters
- right pane: selected deliverable detail and actions

It should feel like browsing living project materials, not like browsing uploaded files.

## 5. Deliverable Mode Spec

`Deliverable` is the primary output-review surface.

### 5.1 Layout

The mode is vertically structured in this order:

1. deliverable header
2. version rail or version block
3. main preview surface
4. footer actions or save state

### 5.2 Deliverable Header

The header contains:

- title
- type or preview-kind badge
- version badge
- updated-at label
- optional promotion status

Header copy should be quiet and factual. No marketing copy, no explanatory prose.

### 5.3 Version Block

The version block should feel like understated workbench controls:

- a vertical or compact stacked version list
- selected version highlighted with accent tint and stronger text
- old versions remain visible but visually secondary

It should not read like browser tabs or a segmented control overloaded with metadata.

### 5.4 Preview Surface

The preview is the visual center of the right rail.

Rules:

- markdown and prose read as document-like content
- code and JSON use a sober editor surface
- images render directly on a neutral surface, not inside decorative frames
- unsupported files use a factual file fallback with metadata and actions

The preview should occupy most of the rail height once a deliverable is selected.

### 5.5 Edit State

Editing should feel like modifying a working document, not opening a modal wizard.

Rules:

- edit state lives inline inside the preview area
- the editing toolbar is small and local
- `Save version` is the primary action
- `Cancel` is neutral
- saving should not collapse the panel or discard version context

## 6. Context Mode Spec

`Context` is for nearby supporting material, not for the main result.

It contains:

- linked resources
- selected memory and freshness summary
- promotion context and lineage summary
- related knowledge or provenance references

Visual behavior:

- stacked sections with subtle headers
- smaller typography than deliverable content
- badges and meta rows are fine
- avoid huge empty illustration states

## 7. Ops Mode Spec

`Ops` is the execution explanation layer.

It contains:

- trace timeline
- approval or auth state
- worker/background/subrun status
- tool history and execution metadata

Visual behavior:

- trace blocks and callouts should feel utilitarian and legible
- approval or auth items may use warning callouts
- do not let ops visuals visually overpower the deliverable mode
- tool rows should feel like system evidence, not feed items

## 8. Interaction Rules

### 8.1 Selecting A Deliverable

When a user clicks a deliverable from a message:

- the right rail opens if collapsed
- the active mode switches to `Deliverable`
- the selected deliverable loads immediately
- the selected version defaults to latest unless the route explicitly targets another version

### 8.2 Switching Versions

When a user switches versions:

- the preview updates in place
- the scroll position inside the preview resets sensibly
- the conversation context does not change
- no full-page loading transition is shown

### 8.3 Saving A New Version

When a user saves:

- keep the same deliverable selected
- append a new version to history
- switch to the new version
- show quiet success feedback near the action area
- do not open toast-heavy celebratory feedback

### 8.4 Promoting To Knowledge

Promotion should feel deliberate and informational:

- show the current promotion state clearly
- keep the action inline in deliverable detail and project deliverable detail
- after success, reflect the new knowledge linkage in both `Deliverable` and `Context`

### 8.5 Forking To A Conversation

Fork should communicate continuity:

- user sees that a new conversation is being seeded from this deliverable
- the current project context remains stable
- after navigation, the new conversation should still feel like the same workbench, not a special flow

## 9. Empty, Loading, Error, And Saving States

### 9.1 Empty

Use factual, low-drama empty states:

- no illustrations
- no large banners
- one short title
- one short explanation
- optional single action

### 9.2 Loading

- prefer skeleton blocks or subdued placeholder panels
- loading should preserve layout stability
- avoid spinner-only empty canvases when panel structure is known

### 9.3 Error

- use compact error callouts
- show recovery actions where possible
- errors should remain inside the local surface unless the whole page is blocked

### 9.4 Saving

- show local save feedback near the editable area
- disable only the minimum required controls
- preserve visible content while saving

## 10. Component Guidance

Use shared `@octopus/ui` primitives first.

Expected base mapping:

- tabs or mode switch: `UiTabs` or a restrained toolbar row pattern
- rail sections: `UiPanelFrame`
- metadata chips: `UiBadge`
- list rows: `UiListRow`
- trace records: `UiTraceBlock`
- empty states: `UiEmptyState`
- actions: `UiButton`

If a new shared component is needed for version history or deliverable preview framing, add it through `@octopus/ui` rather than burying it inside one business page.

## 11. Forbidden Patterns For This Feature

Do not introduce any of the following:

- glossy “AI assistant” gradients
- purple or multicolor chat accents
- code-editor chrome for the whole page
- oversized message bubbles that dominate the workbench
- floating preview cards inside the right rail
- modal-driven version history
- pill storms with every field turned into a badge
- empty-state mascot illustrations
- separate visual dialects for `Deliverable`, `Context`, and `Ops`

## 12. Visual Acceptance Checklist

- [ ] The page still reads as Octopus desktop, not a standalone AI product.
- [ ] The first visual priority after generation is the deliverable body.
- [ ] The right rail feels like a document work surface, not a diagnostics inspector.
- [ ] `Context` and `Ops` are clearly accessible but visually secondary to `Deliverable`.
- [ ] Typography, color, borders, radii, and shadows all match `docs/design/DESIGN.md`.
- [ ] There are no page-local accent colors, gradients, blur, or novelty motion.
- [ ] The project deliverables page feels like the same system as the conversation page.
- [ ] Empty, loading, error, and saving states are calm and layout-stable.

## 13. Handoff To The Implementation Plan

This design spec pairs with:

- [Implementation Plan](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/chat/2026-04-16-claude-inspired-project-conversation-deliverable-implementation.md)

Implementation tasks that must follow this document most closely:

- Task 4: desktop state model
- Task 5: conversation right rail rebuild
- Task 6: project deliverables surface
