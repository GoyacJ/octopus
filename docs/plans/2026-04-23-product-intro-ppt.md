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

### Task 4: Supplement the deck with `apps/octopus-website` product definitions

Status: `done`

Files:
- Modify: `docs/plans/2026-04-23-product-intro-ppt.md`
- Modify: `output/slides/2026-04-23-octopus-product-intro/narrative_plan.md`
- Modify: `output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Modify: `output/slides/2026-04-23-octopus-product-intro/output.pptx`
- Modify: `output/slides/2026-04-23-octopus-product-intro/preview/`

Preconditions:
- The first deck export is already readable and recoverable from Task 1-3 outputs.
- `apps/octopus-website/src/components/sections/*` has been re-reviewed for product-definition language.

Step 1:
- Action: Extend the narrative plan with the additional product-definition points now confirmed from `apps/octopus-website`.
- Done when: the plan records which new website-backed claims are being added to the deck and which slide carries them.
- Verify: `sed -n '1,260p' output/slides/2026-04-23-octopus-product-intro/narrative_plan.md`
- Stop if: the website language conflicts with the existing deck narrative and cannot be reconciled without changing source-of-truth wording.

Step 2:
- Action: Update the JS builder to add the missing website-backed product-definition content and rerender the deck.
- Done when: the builder contains the supplemented slide copy/layout and `output.pptx` is regenerated successfully.
- Verify: `'/Users/goya/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node' output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Stop if: the updated layout causes export failure or severe visual regression.

Step 3:
- Action: Re-check previews and editable text coverage after the supplement pass.
- Done when: the new or changed slides pass one visual QA pass and the key new copy is present in slide XML.
- Verify: `find output/slides/2026-04-23-octopus-product-intro/preview -maxdepth 1 -type f | sort`
- Verify: `PPTX='output/slides/2026-04-23-octopus-product-intro/output.pptx'; for f in $(zipinfo -1 "$PPTX" | rg '^ppt/slides/slide[0-9]+\\.xml$' | sort -V); do printf '=== %s ===\n' "$f"; unzip -p "$PPTX" "$f" | perl -ne 'while(/<a:t>([^<]*)<\\/a:t>/g){print "$1\n"}'; done | sed -n '1,260p'`
- Stop if: the supplemental copy only appears in images or the rerender introduces blocking layout issues.

## Checkpoint 2026-04-23 20:22

- Batch: Task 4 complete
- Current task:
  - none
- Completed:
  - Re-reviewed `apps/octopus-website/src/components/sections/hero.tsx`, `features.tsx`, `platform.tsx`, `workflow.tsx`, `capabilities.tsx`, `comparison.tsx`, `faq.tsx`, `usecases.tsx`, `cta.tsx`
  - Extended `output/slides/2026-04-23-octopus-product-intro/narrative_plan.md` with a new product-definition slide and updated slide numbering
  - Updated `output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs` to add a new `slide-03` for core design decisions, align the interface page with `platform.tsx`, and add open-source/free/local positioning to the closing page
  - Rerendered `output/slides/2026-04-23-octopus-product-intro/output.pptx`
  - Regenerated `output/slides/2026-04-23-octopus-product-intro/preview/slide-01.png` to `slide-08.png`
- Verification:
  - `'/Users/goya/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node' output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs` -> deck exported successfully
  - Visual QA on `preview/slide-03.png`, `slide-05.png`, `slide-08.png` -> no blocking layout issue found after supplement pass
  - OOXML text inspection on `output.pptx` -> new website-backed copy is present as editable text in `ppt/slides/slide3.xml`, `slide5.xml`, and `slide8.xml`
- Blockers:
  - none
- Next:
  - Hand off updated `.pptx`

### Task 5: Rebuild the scenario slide with four scene-first directions and calm still-life visuals

Status: `done`

Files:
- Modify: `docs/plans/2026-04-23-product-intro-ppt.md`
- Modify: `output/slides/2026-04-23-octopus-product-intro/narrative_plan.md`
- Modify: `output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Modify: `output/slides/2026-04-23-octopus-product-intro/output.pptx`
- Modify: `output/slides/2026-04-23-octopus-product-intro/preview/`
- Create: `output/slides/2026-04-23-octopus-product-intro/assets/scenarios/`

Preconditions:
- Task 4 output is readable and still the active source deck.
- `apps/octopus-website/src/components/sections/usecases.tsx` has been re-read as the scenario copy source.
- Local scenario visuals under `output/slides/2026-04-23-octopus-product-intro/assets/scenarios/` are readable and suitable for non-case, non-portrait scene cards.

Step 1:
- Action: Extend the narrative plan and this execution plan with the new four-direction scenario structure and the revised still-life asset mapping.
- Done when: the plan records the chosen directions, each slide-7 scene description, and the local source for every scenario visual.
- Verify: `sed -n '1,320p' output/slides/2026-04-23-octopus-product-intro/narrative_plan.md`
- Verify: `sed -n '1,340p' docs/plans/2026-04-23-product-intro-ppt.md`
- Stop if: the scenario directions cannot be grounded in product sources or the selected still-life visuals fail to communicate the scene cleanly.

Step 2:
- Action: Replace the old segment/case layout in `slide-07` with four scenario cards and wire the new assets into the builder.
- Done when: `buildSlide7` renders four visually distinct scenario cards for 内容创作、办公写作、软件研发、财务运营, with image frames and concise task/capability copy.
- Verify: `sed -n '620,860p' output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Stop if: the new layout makes the slide too dense, export fails, or the selected visuals do not crop cleanly.

Step 3:
- Action: Rerender the deck and re-check `slide-07` preview plus editable text coverage.
- Done when: `output.pptx` is regenerated, `preview/slide-07.png` matches the intended layout, and the new scenario copy appears in slide XML text nodes.
- Verify: `'/Users/goya/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node' output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs`
- Verify: `find output/slides/2026-04-23-octopus-product-intro/preview -maxdepth 1 -type f | sort`
- Verify: `PPTX='output/slides/2026-04-23-octopus-product-intro/output.pptx'; unzip -p \"$PPTX\" ppt/slides/slide7.xml | perl -ne 'while(/<a:t>([^<]*)<\\/a:t>/g){print \"$1\\n\"}'`
- Stop if: rerendering introduces blocking visual defects or the new copy only exists inside images.

## Checkpoint 2026-04-23 09:20

- Batch: Task 5 started
- Current task:
  - Task 5 Step 1
- Completed:
  - Re-read `apps/octopus-website/src/components/sections/usecases.tsx`
  - Confirmed the old `slide-07` layout no longer matches the requested scenario framing
  - Downloaded local scenario assets under `output/slides/2026-04-23-octopus-product-intro/assets/scenarios/`
- Asset picks in progress:
  - 内容创作：`content-film.jpg` + `content-comic.jpg`
  - 办公写作：`office-writing.jpg`
  - 软件研发：`software-tests.jpg`
  - 财务运营：`finance-ops.jpg`
- Verification:
  - `file output/slides/2026-04-23-octopus-product-intro/assets/scenarios/*` -> selected scenario assets are readable local images
- Blockers:
  - none
- Next:
  - Update the narrative plan for the four-card scenario structure
  - Replace `buildSlide7`
  - Rerender and inspect `preview/slide-07.png`

## Checkpoint 2026-04-23 11:10

- Batch: Task 5 direction reset
- Current task:
  - Task 5 Step 1
- Completed:
  - Rejected the old scenario picks that leaned on concrete案例 or visually noisy imagery
  - Reframed `slide-07` as four scene cards instead of industry case cards
  - Locked the new visual rule: no人物主体, no具体品牌/组织案例, scene-first and wallpaper-like still-life quality
- Active asset mapping:
  - 内容创作：`content-creation-lens.jpg`
  - 办公写作：`office-writing-desk.jpg`
  - 软件研发：`software-dev-code.jpg`
  - 财务运营：`finance-ops-calculator.jpg`
- Deprecated candidates:
  - `content-film.jpg`
  - `content-comic.jpg`
  - `office-writing.jpg`
  - `software-tests.jpg`
  - `finance-ops.jpg`
- Verification:
  - `file output/slides/2026-04-23-octopus-product-intro/assets/scenarios/*` -> new still-life assets are readable JPEG files
- Blockers:
  - none
- Next:
  - Update `narrative_plan.md` with the final four-card scene framing
  - Replace `buildSlide7` and wire the new local image assets
  - Rerender and inspect `preview/slide-07.png`

## Checkpoint 2026-04-23 11:26

- Batch: Task 5 complete
- Current task:
  - none
- Completed:
  - Updated `output/slides/2026-04-23-octopus-product-intro/narrative_plan.md` to the final four-scene framing
  - Replaced `buildSlide7` with a 2x2 scene-card layout and connected the new still-life assets
  - Fixed image data URL MIME detection so local `.jpg` assets render correctly
  - Rerendered `output/slides/2026-04-23-octopus-product-intro/output.pptx`
  - Regenerated `output/slides/2026-04-23-octopus-product-intro/preview/slide-01.png` to `slide-08.png`
- Verification:
  - `'/Users/goya/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node' output/slides/2026-04-23-octopus-product-intro/build-product-intro-deck.mjs` -> deck exported successfully
  - `find output/slides/2026-04-23-octopus-product-intro/preview -maxdepth 1 -type f | sort` -> preview set present through `slide-08.png`
  - `unzip -p output/slides/2026-04-23-octopus-product-intro/output.pptx ppt/slides/slide7.xml | perl ...` -> new scene copy is present as editable text nodes
  - Visual QA on `output/slides/2026-04-23-octopus-product-intro/preview/slide-07.png` -> no blocking clipping or mismatched imagery found
- Blockers:
  - none
- Next:
  - Hand off the updated `.pptx`
