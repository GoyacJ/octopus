# Product Intro PPT Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

Generate a polished Chinese `.pptx` product-introduction deck for Octopus that reuses repo-backed product definitions, website copy, and current product screenshots.

## Architecture

Keep the source of truth in the repository: product narrative comes from `apps/website` and `apps/octopus-website`, screenshots come from existing website/public and `output/playwright` assets, and the deck is built by a local JS builder in a dedicated output directory. The final deliverable is an editable PowerPoint deck plus support files that make the narrative and verification recoverable.

## Scope

- In scope:
  - Create one execution plan for this PPT work.
  - Create a narrative/source plan for the deck.
  - Create a JS slide builder and generate a final editable `.pptx`.
  - Reuse existing product screenshots and logos from the repo.
  - Render previews and run at least one fix-and-verify cycle before final handoff.
- Out of scope:
  - Editing website product copy or screenshots in app source.
  - Shipping a bilingual deck in this batch.
  - External publishing, PR creation, or website deployment.

## Risks Or Open Questions

- Current repo references some proposal-support images under `output/docx-assets`, but those files are missing. If the deck truly needs those diagrams, rebuild them inside the deck instead of waiting for the missing files.
- If the bundled slide runtime cannot resolve `@oai/artifact-tool`, stop and repair the runtime setup before writing more builder code.
- If existing screenshots are too stale or too small for full-slide placement, stop before inventing fake UI and switch to cropped usage or a fresh capture path.

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not replace repo-backed product definitions with improvised marketing claims.
- Stop when source of truth, asset availability, or deck export verification is unclear.
- Execute in small batches and update status in place after each batch.

## Task Ledger

### Task 1: Lock deck narrative and asset plan

Status: `done`

Files:
- Modify: `docs/plans/2026-04-23-product-intro-ppt.md`
- Create: `output/slides/2026-04-23-octopus-product-intro/narrative_plan.md`

Preconditions:
- Repo product copy and screenshot sources have been inspected.
- Output directory path is agreed as `output/slides/2026-04-23-octopus-product-intro`.

Step 1:
- Action: Write the deck narrative plan with audience, objective, slide list, source mapping, and visual system.
- Done when: `narrative_plan.md` lists each slide, its key message, and the repo source or asset path that supports it.
- Verify: `sed -n '1,260p' output/slides/2026-04-23-octopus-product-intro/narrative_plan.md`
- Stop if: product story cannot be grounded in repo files without making up claims.

Step 2:
- Action: Update this plan with current status and note any asset gaps that must be solved in the builder.
- Done when: the current task/step is recoverable from this file without rereading chat.
- Verify: `sed -n '1,260p' docs/plans/2026-04-23-product-intro-ppt.md`
- Stop if: the plan reveals a missing dependency that blocks deck authoring.

### Task 2: Build the editable product-intro deck

Status: `done`

Files:
- Modify: `docs/plans/2026-04-23-product-intro-ppt.md`
- Create: `output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Create: `output/slides/2026-04-23-octopus-product-intro/output.pptx`

Preconditions:
- Task 1 is done.
- Bundled Node runtime and slide dependencies are available.

Step 1:
- Action: Implement the JS builder with a consistent visual system, editable text, and repo-backed screenshots/logo assets.
- Done when: the builder creates the planned slides with real text objects, shapes, and image placements.
- Verify: `sed -n '1,320p' output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Stop if: the builder cannot resolve required dependencies or cannot access the chosen assets.

Step 2:
- Action: Run the builder and export the final `.pptx`.
- Done when: `output.pptx` exists and can be reopened by deck inspection tools.
- Verify: `test -f output/slides/2026-04-23-octopus-product-intro/output.pptx`
- Stop if: export fails or produces an unreadable deck.

### Task 3: Verify, fix, and package the deck

Status: `done`

Files:
- Modify: `docs/plans/2026-04-23-product-intro-ppt.md`
- Modify: `output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Modify: `output/slides/2026-04-23-octopus-product-intro/output.pptx`
- Create: `output/slides/2026-04-23-octopus-product-intro/preview/`

Preconditions:
- Task 2 has produced a readable `.pptx`.

Step 1:
- Action: Extract deck text and render slide previews for QA.
- Done when: the deck text can be read and preview images exist for visual inspection.
- Verify: `python -m markitdown output/slides/2026-04-23-octopus-product-intro/output.pptx | sed -n '1,260p'`
- Verify: `find output/slides/2026-04-23-octopus-product-intro/preview -maxdepth 1 -type f | sort`
- Stop if: preview rendering or text extraction fails.

Step 2:
- Action: Fix layout/content issues found in QA and rerun export plus verification.
- Done when: one fix-and-verify cycle is completed and no blocking visual/content issue remains.
- Verify: `python -m markitdown output/slides/2026-04-23-octopus-product-intro/output.pptx | sed -n '1,260p'`
- Verify: `find output/slides/2026-04-23-octopus-product-intro/preview -maxdepth 1 -type f | sort`
- Stop if: repeated fixes keep breaking export or create conflicting layout constraints.

## Checkpoint 2026-04-23 00:00

- Batch: Plan creation
- Completed:
  - Established the execution plan for the product-intro PPT work.
- Verification:
  - `sed -n '1,260p' docs/plans/2026-04-23-product-intro-ppt.md` -> pending until file is written
- Blockers:
  - none
- Next:
  - Task 1 Step 1

## Checkpoint 2026-04-23 19:35

- Batch: Task 1 complete, Task 2 complete, Task 3 in progress
- Current task:
  - Task 3 Step 2
- Completed:
  - Wrote `output/slides/2026-04-23-octopus-product-intro/narrative_plan.md`
  - Built `output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
  - Exported `output/slides/2026-04-23-octopus-product-intro/output.pptx`
  - Rendered `output/slides/2026-04-23-octopus-product-intro/preview/slide-01.png` to `slide-07.png`
- QA findings:
  - `slide-03`: conversation screenshot frame was too wide and short, so the interface proof looked over-cropped.
  - `slide-07`: value-card index labels `01/02/03` were clipped in preview.
- Changes in progress:
  - Rework `slide-03` into screenshot + proof-card layout.
  - Widen `slide-07` value-card index labels to avoid clipping.
- Verification:
  - Preview images exist under `output/slides/2026-04-23-octopus-product-intro/preview/`
  - Bundled python does not currently provide `markitdown`; if that remains true after rerender, verify editable text by inspecting `ppt/slides/slide*.xml` inside `output.pptx`.
- Blockers:
  - none
- Next:
  - Rerender deck and preview
  - Run text verification
  - Confirm no blocking visual issue remains

## Checkpoint 2026-04-23 19:48

- Batch: Task 3 complete
- Current task:
  - none
- Completed:
  - Updated `output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs` to fix `slide-03` screenshot presentation and `slide-07` index label clipping
  - Rerendered `output/slides/2026-04-23-octopus-product-intro/output.pptx`
  - Regenerated `output/slides/2026-04-23-octopus-product-intro/preview/slide-01.png` to `slide-07.png`
- Verification:
  - `'/Users/goya/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node' output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs` -> deck exported successfully
  - Visual QA on `preview/slide-03.png` and `preview/slide-07.png` -> blocking issues resolved
  - OOXML text inspection on `output.pptx` -> slide text is present as editable text in `ppt/slides/slide1.xml` to `slide7.xml`
- Blockers:
  - none
- Next:
  - Hand off final `.pptx`
