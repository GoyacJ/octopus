# Octopus Desktop Brand Rename Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the remaining desktop `网易Lobster` / `Lobster` product branding with `Octopus`, and regenerate desktop icon assets from the provided root brand source image.

**Architecture:** Keep the change scoped to branding surfaces and packaging identity already present in the desktop app. Update tests first to define the new visible brand and release artifact expectations, then change Vue shell branding, Tauri metadata, release-manifest fixtures, and rasterized icon assets derived from the supplied SVG.

**Tech Stack:** Vue 3 + Vite + Vitest, Tauri 2 desktop shell, Node release scripts/tests, raster asset generation via local CLI tools.

---

### Task 1: Capture the new brand expectations in tests

**Files:**
- Modify: `apps/desktop/test/layout-shell.test.ts`
- Modify: `apps/desktop/test/user-center.test.ts`
- Modify: `apps/desktop/test/project-settings-view.test.ts`
- Modify: `apps/desktop/test/release-artifact-governance.test.ts`

**Step 1: Write the failing test**

Change the hard-coded brand assertions from `网易Lobster` / `Lobster.*` to `Octopus` / `Octopus.*` in the existing desktop shell and release-governance tests.

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run apps/desktop/test/layout-shell.test.ts apps/desktop/test/user-center.test.ts apps/desktop/test/project-settings-view.test.ts apps/desktop/test/release-artifact-governance.test.ts`

Expected: FAIL because the UI, fixtures, or release metadata still expose the old Lobster branding.

**Step 3: Write minimal implementation**

Update the production code and test fixtures in later tasks until the revised assertions pass.

**Step 4: Run test to verify it passes**

Run the same `pnpm vitest run ...` command and confirm all selected tests pass.

**Step 5: Commit**

```bash
git add apps/desktop/test/layout-shell.test.ts apps/desktop/test/user-center.test.ts apps/desktop/test/project-settings-view.test.ts apps/desktop/test/release-artifact-governance.test.ts
git commit -m "test: update desktop branding expectations"
```

### Task 2: Replace visible desktop brand surfaces

**Files:**
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `apps/desktop/index.html`
- Modify: `apps/desktop/test/support/workspace-fixture.ts`

**Step 1: Write the failing test**

Reuse the updated assertions from Task 1 for topbar breadcrumbs, sidebar branding, and owner display names.

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run apps/desktop/test/layout-shell.test.ts apps/desktop/test/user-center.test.ts apps/desktop/test/project-settings-view.test.ts`

Expected: FAIL because the shell still renders `网易Lobster` or fixture data still returns `Lobster Owner`.

**Step 3: Write minimal implementation**

Change the desktop shell brand copy to `Octopus`, switch the sidebar logo reference to the regenerated asset, and update workspace fixture display names that intentionally expose the product brand.

**Step 4: Run test to verify it passes**

Run the same focused Vitest command and confirm it passes.

**Step 5: Commit**

```bash
git add apps/desktop/src/components/layout/WorkbenchSidebar.vue apps/desktop/src/components/layout/WorkbenchTopbar.vue apps/desktop/index.html apps/desktop/test/support/workspace-fixture.ts
git commit -m "feat: rename desktop shell branding to octopus"
```

### Task 3: Replace Tauri packaging identity and release expectations

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json`
- Modify: `crates/octopus-server/src/lib.rs`
- Modify: `apps/desktop/test/release-artifact-governance.test.ts`

**Step 1: Write the failing test**

Use the updated release artifact governance expectations from Task 1 to require `Octopus.app.tar.gz`, `Octopus.nsis.zip`, and `Octopus.AppImage.tar.gz` paths.

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run apps/desktop/test/release-artifact-governance.test.ts`

Expected: FAIL because Tauri/product metadata and server-side sample manifests still emit Lobster artifact names or identifier strings.

**Step 3: Write minimal implementation**

Rename `productName`, window title, bundle identifier, and sample update URLs to the Octopus identity needed by packaging and release verification.

**Step 4: Run test to verify it passes**

Run the same focused release test and confirm it passes.

**Step 5: Commit**

```bash
git add apps/desktop/src-tauri/tauri.conf.json crates/octopus-server/src/lib.rs apps/desktop/test/release-artifact-governance.test.ts
git commit -m "feat: align tauri packaging with octopus branding"
```

### Task 4: Regenerate desktop icon assets from the root brand source image

**Files:**
- Modify: `apps/desktop/public/logo.png`
- Modify: `apps/desktop/public/logo.jpg`
- Modify: `apps/desktop/src/assets/logo.png`
- Modify: `apps/desktop/src-tauri/app-icon.png`
- Modify: `apps/desktop/src-tauri/icons/32x32.png`
- Modify: `apps/desktop/src-tauri/icons/64x64.png`
- Modify: `apps/desktop/src-tauri/icons/128x128.png`
- Modify: `apps/desktop/src-tauri/icons/128x128@2x.png`
- Modify: `apps/desktop/src-tauri/icons/icon.png`
- Modify: `apps/desktop/src-tauri/icons/icon.icns`
- Modify: `apps/desktop/src-tauri/icons/icon.ico`

**Step 1: Write the failing test**

No automated pixel-level test exists. The failure condition is visual mismatch and stale Lobster icon assets after regeneration.

**Step 2: Run test to verify it fails**

Inspect the current sidebar icon and packaging assets; confirm they do not match the supplied root brand source image.

**Step 3: Write minimal implementation**

Rasterize the provided root brand source image into the desktop web and Tauri icon sizes, then rebuild `.icns` / `.ico` from the generated PNGs using local CLI tooling.

**Step 4: Run test to verify it passes**

Run focused tests plus file inspection commands, and confirm generated assets exist at the required paths with updated timestamps and dimensions.

**Step 5: Commit**

```bash
git add apps/desktop/public/logo.png apps/desktop/public/logo.jpg apps/desktop/src/assets/logo.png apps/desktop/src-tauri/app-icon.png apps/desktop/src-tauri/icons
git commit -m "chore: regenerate desktop icon assets from octopus logo"
```

### Task 5: Verify the full rename end-to-end

**Files:**
- Verify only

**Step 1: Run focused desktop branding tests**

Run: `pnpm vitest run apps/desktop/test/layout-shell.test.ts apps/desktop/test/user-center.test.ts apps/desktop/test/project-settings-view.test.ts apps/desktop/test/release-artifact-governance.test.ts`

Expected: PASS

**Step 2: Run packaging metadata smoke check**

Run: `pnpm vitest run apps/desktop/test/tauri-build-config.test.ts`

Expected: PASS

**Step 3: Spot-check no old visible brand remains in desktop code**

Run: `rg -n --hidden -S "网易Lobster|Lobster\\.app|Lobster\\.nsis|Lobster\\.AppImage|com\\.weilaizhigu\\.lobster|com\\.weilaizhihuigu\\.lobster" apps/desktop crates`

Expected: no remaining hits in the updated desktop brand surfaces, except any intentionally unrelated historical text if discovered and reviewed.

**Step 4: Commit**

```bash
git add .
git commit -m "feat: complete octopus desktop brand migration"
```
