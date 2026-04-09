# Dock Icon Balance Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild the desktop app icon set so the Dock icon keeps the current 3D octopus but reads smaller and more balanced.

**Architecture:** Recompose the icon master art from the transparent root octopus source plus the existing rounded-square alpha mask, then regenerate the Tauri icon outputs from that master file. Keep the change scoped to desktop icon assets and verification artifacts only.

**Tech Stack:** macOS AppKit image compositing, Tauri CLI icon generation, shell verification commands

---

### Task 1: Record the approved icon composition

**Files:**
- Create: `docs/plans/2026-04-09-dock-icon-balance-design.md`

**Step 1: Write the design record**

Document the approved direction: preserve the 3D octopus, preserve the beige rounded-square tile, and scale the foreground subject down to roughly `88%`.

**Step 2: Verify the document exists**

Run: `test -f docs/plans/2026-04-09-dock-icon-balance-design.md`
Expected: exit code `0`

### Task 2: Rebuild the icon master art

**Files:**
- Modify: `apps/desktop/src-tauri/app-icon.png`

**Step 1: Inspect the existing source assets**

Run:

```bash
file logo.png apps/desktop/src-tauri/app-icon.png
```

Expected: `logo.png` is a transparent PNG source and `app-icon.png` is the current Tauri master icon.

**Step 2: Generate the updated master icon**

Use a small AppKit composition script to:
- load `logo.png` as the foreground
- reuse the existing rounded-square alpha silhouette from `apps/desktop/src-tauri/app-icon.png`
- draw a solid beige background using that silhouette
- scale and center the octopus foreground to about `88%` of the previous composition

**Step 3: Verify the master icon**

Run:

```bash
sips -g pixelWidth -g pixelHeight apps/desktop/src-tauri/app-icon.png
```

Expected: `1024x1024`

### Task 3: Regenerate bundled desktop icon outputs

**Files:**
- Modify: `apps/desktop/src-tauri/icons/32x32.png`
- Modify: `apps/desktop/src-tauri/icons/64x64.png`
- Modify: `apps/desktop/src-tauri/icons/128x128.png`
- Modify: `apps/desktop/src-tauri/icons/128x128@2x.png`
- Modify: `apps/desktop/src-tauri/icons/icon.png`
- Modify: `apps/desktop/src-tauri/icons/icon.icns`
- Modify: `apps/desktop/src-tauri/icons/icon.ico`
- Modify: `apps/desktop/src-tauri/icons/Square107x107Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square142x142Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square150x150Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square284x284Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square30x30Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square310x310Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square44x44Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square71x71Logo.png`
- Modify: `apps/desktop/src-tauri/icons/Square89x89Logo.png`
- Modify: `apps/desktop/src-tauri/icons/StoreLogo.png`

**Step 1: Regenerate the icon set**

Run:

```bash
pnpm tauri icon apps/desktop/src-tauri/app-icon.png --output apps/desktop/src-tauri/icons
```

Expected: Tauri regenerates the macOS, Windows, and platform PNG icon outputs.

**Step 2: Verify the key files exist**

Run:

```bash
test -f apps/desktop/src-tauri/icons/icon.icns && test -f apps/desktop/src-tauri/icons/icon.png
```

Expected: exit code `0`

### Task 4: Verify the rebuilt Dock icon assets

**Files:**
- Verify only

**Step 1: Check the generated dimensions**

Run:

```bash
sips -g pixelWidth -g pixelHeight apps/desktop/src-tauri/app-icon.png apps/desktop/src-tauri/icons/icon.png
```

Expected: `app-icon.png` is `1024x1024` and `icon.png` is `512x512`

**Step 2: Inspect the changed assets**

Run:

```bash
git diff --stat -- apps/desktop/src-tauri/app-icon.png apps/desktop/src-tauri/icons
```

Expected: the icon master and generated icon bundle files are listed as modified.
