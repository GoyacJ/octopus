# Octopus Website Redesign Design

## Context

The first `apps/website` implementation ships the correct platform choices and route structure, but the visual result is too card-heavy, too even in section weight, and too weak in first-screen hierarchy for a brand narrative site.

The redesign should reference the composition discipline of `opencow.ai`:

- stronger first viewport
- fewer but larger visual ideas
- image-led proof instead of equal-weight feature cards
- longer editorial rhythm instead of repeated panel blocks

It must still stay inside Octopus's own brand constraints:

- warm neutral surfaces
- restrained blue accent
- border-first visual language
- no purple gradients, glassmorphism, glow CTA, or generic AI aesthetics
- real product screenshots as primary proof

## Product And Visual Thesis

### Brand Thesis

Octopus is not introduced as an enterprise platform category label. It is introduced as a product about people directing work and AI carrying it forward across conversation, knowledge, trace, and runtime surfaces.

### Visual Thesis

Editorial product site, not admin UI and not generic SaaS landing page.

- the first viewport should feel like a poster
- the dominant image should be a current, real product screenshot
- typography should carry more authority than chrome
- the page should rely on spacing, scale, cropping, and contrast before cards

### Interaction Thesis

Motion stays restrained:

- gentle hero reveal on load
- section fade/translate entry on scroll
- screenshot hover or parallax depth only where it strengthens focus

No ornamental motion loops.

## Screenshot Policy

All product screenshots currently referenced by the site must be refreshed before final implementation lands.

Required refresh set:

- `dashboard`
- `conversation`
- `knowledge`
- `trace`
- `settings-governance`

Rules:

- capture from the current browser-host desktop UI, not archived output
- use real fixture-backed data from the current desktop experience
- export to `output/playwright/*.png` first
- then copy the approved refreshed files into `apps/website/public/screenshots/`
- if a surface no longer matches the current product, replace the website usage rather than forcing an outdated screenshot name to survive

## Homepage Redesign

## New Section Order

1. Hero
2. Belief statement
3. Product proof
4. Narrative loop
5. Scenario chapters
6. Closing CTA

### 1. Hero

Replace the current split hero with a more forceful composition:

- left: oversized headline, one short paragraph, two actions
- right: one dominant screenshot framed as product proof
- remove small equal-weight supporting cards from the first viewport
- keep the current theme switch and locale switch in the global header, but visually quiet them

### 2. Belief Statement

This becomes a sparse editorial block:

- one large line of belief-driven copy
- one supporting paragraph
- optional short metadata line

No card treatment.

### 3. Product Proof

Replace the current proof strip with a single primary evidence section:

- one large screenshot or sequence anchor
- a short vertical list of 3 proof statements or metrics
- light supporting labels only

### 4. Narrative Loop

Keep the `think / execute / retain` logic, but stop rendering it as three equal cards.

Use stacked alternating media/text chapters:

- chapter 1 text left, media right
- chapter 2 media left, text right
- chapter 3 text left, media right

### 5. Scenario Chapters

Keep `个人 / 团队 / 企业`, but present them more like chapters than marketing cards:

- lighter framing
- stronger headings
- shorter bullets
- one shared product throughline

### 6. Closing CTA

Use a clean final band:

- large headline
- one short description
- primary `Book a Demo`
- secondary `View Product`

No oversized panel shell unless needed for contrast.

## Inner Pages

The inner pages should follow the new visual system but remain more explanatory than the homepage.

### Product

- stronger hero with one dominant structure diagram or screenshot
- feature loop as large alternating rows, not stacked panel cards

### Scenarios

- chapter-style sections for `个人 / 团队 / 企业`
- less chrome, stronger layout rhythm

### About

- editorial tone
- fewer boxed cards
- clearer mission and principles hierarchy

### Book a Demo

- simpler conversion page
- stronger headline and booking options
- less panel duplication

## Layout And Styling Changes

### Keep

- shared tokens from `@octopus/ui/tokens.css`
- light/dark theme system
- bilingual routing and SEO contracts

### Change

- reduce reliance on `.brand-panel`
- reduce repeated rounded-card sections
- increase whitespace and visual asymmetry
- strengthen display type sizing and text column discipline
- make header lighter and more brand-site-like
- use screenshots at larger scale with better cropping and fewer simultaneous visuals

## Files Expected To Change

- `apps/website/pages/index.vue`
- `apps/website/pages/product.vue`
- `apps/website/pages/scenarios.vue`
- `apps/website/pages/about.vue`
- `apps/website/pages/book-demo.vue`
- `apps/website/components/brand/*`
- `apps/website/assets/css/website.css`
- `apps/website/content-or-copy/site.ts`
- `apps/website/public/screenshots/*`

## Verification Goals

- homepage first viewport has one dominant visual idea
- the page is understandable by scanning section headings only
- screenshots are current and consistent with the present desktop UI
- no stale or historical product visuals remain
- light and dark themes both preserve the same brand quality
- `pnpm check:website` remains green
