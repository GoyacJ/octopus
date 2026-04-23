# Website Copy Refresh Plan

## Goal

Bring the richer product narrative from `apps/octopus-website` into `apps/website` so the public site explains Octopus more clearly across the homepage, product, scenarios, about, and consultation pages.

## Architecture

`apps/website` keeps `locales/*.json` as the content source of truth and `pages/*.vue` as the section layout layer. This refresh should reuse the current Nuxt page structure and shared UI primitives, expand locale-backed content objects where needed, and only add page-local rendering for new narrative sections that the current site cannot express yet.

## Scope

- In scope:
  - Refresh `apps/website` Chinese and English copy using `apps/octopus-website` as the reference narrative.
  - Add missing website sections where the current public site is too thin to carry the source messaging.
  - Keep locale object shapes aligned across both languages and preserve existing i18n array access patterns.
- Out of scope:
  - Rebuilding the site visual system.
  - Editing `apps/octopus-website`.
  - Changing download/release fetching logic or non-copy website infrastructure.

## Risks Or Open Questions

- `apps/website/test/site-content.test.ts` currently locks `pages.scenarios.segments` to 3 items and `pages.product.governance.items` to 3 items. If the refresh needs more translated object arrays, tests must be updated deliberately.
- The current worktree already has unrelated user changes outside `apps/website`; do not touch or revert them.

## Execution Rules

- Do not change locale object shapes in one language only.
- Keep array-backed translated sections on `tm()` + `rt()` access paths when page code reads objects.
- Prefer page-local additions in `apps/website/pages/*.vue` over introducing new shared components for this copy refresh.
- Run website verification after each implementation batch and record the result in this file.

## Task Ledger

### Task 1: Define the new website copy model

Status: `done`

Files:
- Create: `docs/plans/2026-04-23-website-copy-refresh.md`
- Modify: `apps/website/locales/zh-CN.json`
- Modify: `apps/website/locales/en-US.json`
- Modify: `apps/website/test/site-content.test.ts`

Preconditions:
- `apps/octopus-website/src/components/sections/*.tsx` has been reviewed for reusable narrative.
- Current `apps/website/pages/*.vue` usage of `t()`, `tm()`, and `rt()` is understood.

Step 1:
- Action: Add the missing locale fields needed to express homepage workflow, chat-vs-task comparison, scenario/use-case detail, about-page principles, and consultation value props.
- Done when: both locale files expose matching field paths for every new section or list used by the updated pages.
- Verify: `node -e "const fs=require('fs'); const zh=JSON.parse(fs.readFileSync('apps/website/locales/zh-CN.json','utf8')); const en=JSON.parse(fs.readFileSync('apps/website/locales/en-US.json','utf8')); const keys=['pages.home.workflow.steps','pages.home.comparison.items','pages.home.useCases','pages.home.faq.items','pages.product.narrative','pages.scenarios.useCases','pages.about.principles','pages.bookDemo.valueProps']; for (const key of keys) { const walk=(obj,path)=>path.split('.').reduce((acc,k)=>acc&&acc[k],obj); if (!walk(zh,key) || !walk(en,key)) { throw new Error('Missing '+key); } } console.log('locale-shape-ok')"`
- Stop if: a needed section cannot be represented cleanly without inventing a third content source beyond the locale files.

Step 2:
- Action: Update the website content test so it validates any new translated object arrays that pages depend on.
- Done when: the test suite asserts the new array-backed sections without breaking the existing `tm()` / `rt()` contract.
- Verify: `pnpm -C apps/website test`
- Stop if: the existing tests reveal a conflicting locale contract that would require broader website architecture changes.

### Task 2: Rebuild the public narrative on the existing pages

Status: `done`

Files:
- Modify: `apps/website/pages/index.vue`
- Modify: `apps/website/pages/product.vue`
- Modify: `apps/website/pages/scenarios.vue`
- Modify: `apps/website/pages/about.vue`
- Modify: `apps/website/pages/book-demo.vue`

Preconditions:
- Task 1 locale fields exist in both languages.

Step 1:
- Action: Expand the homepage so it carries the source website's stronger narrative with workflow, comparison, cross-team use cases, and FAQ sections.
- Done when: `index.vue` renders the new locale-backed sections and the homepage no longer relies only on hero + feature grid + CTA to explain the product.
- Verify: `pnpm -C apps/website typecheck`
- Stop if: the new sections need shared UI primitives that do not exist and cannot be expressed cleanly with current `Ui*` building blocks.

Step 2:
- Action: Update product, scenarios, about, and consultation pages so their copy and supporting sections match the stronger narrative from `apps/octopus-website`.
- Done when: each page has materially richer content grounded in the source site and still reads from the locale files instead of hardcoded page copy.
- Verify: `pnpm -C apps/website typecheck && pnpm -C apps/website test`
- Stop if: page structure changes start conflicting with current screenshot assets or existing route-level expectations.

### Task 3: Verify and checkpoint the refresh

Status: `done`

Files:
- Verify only

Preconditions:
- Tasks 1 and 2 are complete.

Step 1:
- Action: Run the website build and test checks for the refreshed content.
- Done when: the website typecheck, tests, and static generation all pass.
- Verify: `pnpm -C apps/website typecheck && pnpm -C apps/website test && pnpm -C apps/website generate`
- Stop if: failures point to unrelated pre-existing breakage outside the files changed for this task.

Step 2:
- Action: Record the finished batch and verification results in this plan.
- Done when: a checkpoint summarizes changed files, verification status, and next state.
- Verify: `tail -n 40 docs/plans/2026-04-23-website-copy-refresh.md`
- Stop if: the checkpoint would need to omit unresolved failures or uncertainties.

## Checkpoint 2026-04-23 10:00

- Batch: Task 1 Step 1 -> Task 3 Step 2
- Completed:
  - Refreshed `apps/website/locales/zh-CN.json` and `apps/website/locales/en-US.json` with source-backed homepage, product, scenario, about, and consultation content.
  - Rebuilt `apps/website/pages/index.vue`, `product.vue`, `scenarios.vue`, `about.vue`, and `book-demo.vue` to render the richer locale-backed narrative.
  - Extended `apps/website/test/site-content.test.ts` to guard the new translated object-array sections.
- Verification:
  - `node -e "JSON.parse(require('fs').readFileSync('apps/website/locales/zh-CN.json','utf8')); JSON.parse(require('fs').readFileSync('apps/website/locales/en-US.json','utf8')); console.log('json-ok')"` -> pass
  - `node -e "const fs=require('fs'); const zh=JSON.parse(fs.readFileSync('apps/website/locales/zh-CN.json','utf8')); const en=JSON.parse(fs.readFileSync('apps/website/locales/en-US.json','utf8')); const keys=['pages.home.workflow.steps','pages.home.comparison.items','pages.home.useCases.items','pages.home.faq.items','pages.product.modules.items','pages.scenarios.useCases.items','pages.about.principles.items','pages.bookDemo.valueProps.items']; const walk=(obj,path)=>path.split('.').reduce((acc,k)=>acc&&acc[k],obj); for (const key of keys) { if (!walk(zh,key) || !walk(en,key)) throw new Error('Missing '+key); } console.log('shape-ok')"` -> pass
  - `pnpm -C apps/website typecheck` -> pass with npm env warnings from the local toolchain
  - `pnpm -C apps/website test` -> pass
  - `pnpm -C apps/website generate` -> pass
- Blockers:
  - none
- Next:
  - none
