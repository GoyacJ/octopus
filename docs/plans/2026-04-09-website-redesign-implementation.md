# Website Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refresh the Octopus marketing site with a stronger editorial homepage, current product screenshots, and a cleaner brand narrative while preserving the repo's bilingual, theme, and static-generation contracts.

**Architecture:** Keep `apps/website` as a standalone Nuxt 3 SSG-first app. Refresh screenshot assets from the current browser-host desktop UI first, then refactor the homepage and shared brand components around a stronger image-led narrative structure, and finally align the inner pages to the same visual system without turning them into card grids.

**Tech Stack:** Nuxt 3, Vue 3, TypeScript, Tailwind CSS, `@nuxtjs/i18n`, shared `@octopus/ui` tokens/components, Playwright CLI for capture, Vitest, Nitro static prerender.

---

### Task 1: Capture the redesign expectations in tests

**Files:**
- Modify: `apps/website/test/site-foundation.test.ts`
- Modify: `apps/website/test/website-governance.test.ts`
- Create: `apps/website/test/screenshot-registry.test.ts`

**Step 1: Write the failing test**

Add assertions that the website still expects the refreshed screenshot asset set and that public screenshot files exist for the current site.

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/website test`

Expected: FAIL if the new test references screenshots or layout expectations that are not yet implemented.

**Step 3: Write minimal implementation**

Update the tests so they assert the current required screenshot names and website asset contract.

**Step 4: Run test to verify it passes**

Run: `pnpm -C apps/website test`

Expected: PASS

**Step 5: Commit**

```bash
git add apps/website/test/site-foundation.test.ts apps/website/test/website-governance.test.ts apps/website/test/screenshot-registry.test.ts
git commit -m "test: lock website screenshot asset contract"
```

### Task 2: Re-capture current product screenshots

**Files:**
- Modify: `output/playwright/dashboard.png`
- Modify: `output/playwright/conversation.png`
- Modify: `output/playwright/knowledge.png`
- Modify: `output/playwright/trace.png`
- Modify: `output/playwright/settings-governance.png`
- Modify: `apps/website/public/screenshots/dashboard.png`
- Modify: `apps/website/public/screenshots/conversation.png`
- Modify: `apps/website/public/screenshots/knowledge.png`
- Modify: `apps/website/public/screenshots/trace.png`
- Modify: `apps/website/public/screenshots/settings-governance.png`

**Step 1: Start the current browser-host desktop UI**

Run: `pnpm dev:web`

Expected: the browser-host desktop app becomes available locally with the current fixture-backed UI.

**Step 2: Capture the current surfaces**

Use the Playwright CLI skill against the running browser-host UI and capture fresh PNGs for:

- workspace overview/dashboard
- project conversation
- knowledge
- trace
- settings/governance

Save the raw captures into `output/playwright/`.

**Step 3: Promote the approved screenshots into website public assets**

Copy the refreshed approved PNGs from `output/playwright/` into `apps/website/public/screenshots/`.

**Step 4: Verify file presence and freshness**

Run: `find output/playwright apps/website/public/screenshots -maxdepth 1 -type f | sort`

Expected: the refreshed PNGs exist in both locations.

**Step 5: Commit**

```bash
git add output/playwright/dashboard.png output/playwright/conversation.png output/playwright/knowledge.png output/playwright/trace.png output/playwright/settings-governance.png apps/website/public/screenshots/dashboard.png apps/website/public/screenshots/conversation.png apps/website/public/screenshots/knowledge.png apps/website/public/screenshots/trace.png apps/website/public/screenshots/settings-governance.png
git commit -m "chore: refresh website product screenshots"
```

### Task 3: Rebuild the homepage around one dominant visual idea

**Files:**
- Modify: `apps/website/pages/index.vue`
- Modify: `apps/website/components/brand/BrandHero.vue`
- Modify: `apps/website/components/brand/BrandSectionHeading.vue`
- Modify: `apps/website/components/brand/BrandProofStrip.vue`
- Modify: `apps/website/components/brand/BrandScreenshotFrame.vue`
- Modify: `apps/website/components/brand/BrandCallToAction.vue`
- Modify: `apps/website/content-or-copy/site.ts`

**Step 1: Write the failing test**

Add or update a structural test that expects the homepage to reference a dominant screenshot-led hero and the refreshed screenshot registry.

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/website test`

Expected: FAIL because the old homepage structure still renders the previous card-heavy composition.

**Step 3: Write minimal implementation**

Refactor the homepage to:

- reduce first-screen content to one dominant screenshot-led hero
- turn the belief section into a sparse editorial block
- replace the equal-weight proof strip emphasis with a larger evidence section
- present the `think / execute / retain` narrative as larger alternating chapters
- simplify the scenario and final CTA sections

**Step 4: Run test to verify it passes**

Run: `pnpm -C apps/website test`

Expected: PASS

**Step 5: Commit**

```bash
git add apps/website/pages/index.vue apps/website/components/brand/BrandHero.vue apps/website/components/brand/BrandSectionHeading.vue apps/website/components/brand/BrandProofStrip.vue apps/website/components/brand/BrandScreenshotFrame.vue apps/website/components/brand/BrandCallToAction.vue apps/website/content-or-copy/site.ts
git commit -m "feat: redesign website homepage hierarchy"
```

### Task 4: Replace the global website visual system

**Files:**
- Modify: `apps/website/assets/css/website.css`
- Modify: `apps/website/components/brand/BrandSiteHeader.vue`
- Modify: `apps/website/components/brand/BrandSiteFooter.vue`
- Modify: `apps/website/components/shared/SiteButtonLink.vue`
- Modify: `apps/website/components/shared/SiteThemeToggle.vue`
- Modify: `apps/website/components/shared/SiteLanguageSwitcher.vue`

**Step 1: Write the failing test**

No new automated pixel test is required. The failure condition is visual mismatch with the approved redesign direction.

**Step 2: Run visual baseline inspection**

Run: `pnpm -C apps/website generate`

Expected: the existing site still builds, but visually remains the pre-redesign version.

**Step 3: Write minimal implementation**

Rework the global CSS and shell components to:

- reduce pervasive card styling
- strengthen type hierarchy
- lighten header chrome
- improve spacing and section rhythm
- preserve warm neutral and restrained blue brand rules across both themes

**Step 4: Run build verification**

Run: `pnpm -C apps/website typecheck && pnpm -C apps/website generate`

Expected: PASS

**Step 5: Commit**

```bash
git add apps/website/assets/css/website.css apps/website/components/brand/BrandSiteHeader.vue apps/website/components/brand/BrandSiteFooter.vue apps/website/components/shared/SiteButtonLink.vue apps/website/components/shared/SiteThemeToggle.vue apps/website/components/shared/SiteLanguageSwitcher.vue
git commit -m "feat: refresh website visual system"
```

### Task 5: Align the inner pages to the new homepage language

**Files:**
- Modify: `apps/website/pages/product.vue`
- Modify: `apps/website/pages/scenarios.vue`
- Modify: `apps/website/pages/about.vue`
- Modify: `apps/website/pages/book-demo.vue`
- Modify: `apps/website/locales/zh-CN.json`
- Modify: `apps/website/locales/en-US.json`

**Step 1: Write the failing test**

Update any page-structure or locale tests if new section labels or screenshot usage are introduced.

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/website test`

Expected: FAIL if locale or screenshot references are missing.

**Step 3: Write minimal implementation**

Refactor the inner pages to use:

- cleaner hero sections
- fewer repeated panels
- larger screenshot moments
- lighter, chapter-like content groupings
- updated copy only where needed to support the new hierarchy

**Step 4: Run test to verify it passes**

Run: `pnpm -C apps/website test`

Expected: PASS

**Step 5: Commit**

```bash
git add apps/website/pages/product.vue apps/website/pages/scenarios.vue apps/website/pages/about.vue apps/website/pages/book-demo.vue apps/website/locales/zh-CN.json apps/website/locales/en-US.json
git commit -m "feat: align website inner pages with redesign"
```

### Task 6: Verify the full website end-to-end

**Files:**
- Verify only

**Step 1: Run the website verification suite**

Run: `pnpm check:website`

Expected: PASS

**Step 2: Inspect generated output for SEO and theme boot**

Run: `rg -n "octopus-theme-boot|canonical|og:image" apps/website/.output/public/index.html apps/website/.output/public/product/index.html apps/website/.output/public/en/index.html`

Expected: matching SEO and theme boot tags are present.

**Step 3: Inspect refreshed screenshot assets**

Run: `find apps/website/public/screenshots -maxdepth 1 -type f | sort`

Expected: the refreshed screenshot set is present.

**Step 4: Commit**

```bash
git add apps/website apps/website/public output/playwright
git commit -m "feat: ship octopus website redesign"
```
