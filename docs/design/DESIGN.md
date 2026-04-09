# Octopus Desktop Design Standard

`docs/design/DESIGN.md` is the canonical visual and interaction standard for `apps/desktop` and `@octopus/ui`.
All desktop UI changes must follow this document first. Preview files are reference renderings, not a second source of truth.

## 1. Product Direction

Octopus desktop uses a strict Notion-style workbench language:

- warm neutral canvases instead of cold gray or pure black
- a single blue accent for interactive priority
- whisper borders before heavy fills
- restrained radius and shadow scales
- content-first density with consistent shell structure
- no page-level visual dialects

The goal is not "inspired by Notion marketing". The goal is a calm desktop workbench where shell, list/detail, settings, dialog, and conversation all feel like one system.

## 2. Core Principles

- One desktop language: sidebar, topbar, pages, forms, dialogs, drawers, search, and chat share the same tokens.
- Neutral-first chrome: shell and page containers rely on warm white, warm gray, and near-black. Accent color is reserved for focus, links, selected states, and primary actions.
- Borders over decoration: separation comes from whisper borders and spacing, not glassmorphism, strong gradients, or novelty shadows.
- Density with breathing room: interior spacing is compact; page rhythm is generous.
- Typography is product infrastructure: UI chrome uses one sans stack only. Global font family or font style switching is not allowed.
- Dark mode is equal quality, not an exception. It remains warm, matte, and low-contrast rather than neon or high-gloss.

## 3. Forbidden Patterns

Do not introduce any of the following on desktop product surfaces:

- page-local accent palettes such as `indigo-*`, purple hero accents, rainbow badges, or branded gradients
- backdrop blur, frosted glass, translucent floating cards, or image-led chrome
- arbitrary radii or one-off radius utilities outside the canonical scale
- one-off shadow recipes outside the canonical shadow scale
- page-specific "fun" visual branches for pet, automations, or user center
- decorative hero gradients, glowing hover states, or glossy CTA treatments
- global serif / mono UI switching

## 4. Canonical Tokens

### 4.1 Color

Light theme:

- canvas: `#fbfaf8`
- surface: `#ffffff`
- surface-muted: `#f4f2ee`
- sidebar: `#f7f5f2`
- text-primary: `rgba(31, 30, 29, 0.94)`
- text-secondary: `rgba(70, 67, 63, 0.72)`
- text-tertiary: `rgba(94, 89, 82, 0.5)`
- border-whisper: `rgba(31, 30, 29, 0.1)`
- border-strong: `rgba(31, 30, 29, 0.16)`
- accent-blue: `#0b6ed0`
- accent-blue-hover: `#095fb3`
- accent-blue-soft: `#eef5fd`
- success: `#0f7b6c`
- warning: `#b36b18`
- danger: `#c4554d`

Dark theme:

- canvas: `#1f1d1b`
- surface: `#252321`
- surface-muted: `#2b2825`
- sidebar: `#23211f`
- text-primary: `rgba(255, 252, 248, 0.92)`
- text-secondary: `rgba(232, 226, 219, 0.68)`
- text-tertiary: `rgba(232, 226, 219, 0.46)`
- border-whisper: `rgba(255, 252, 248, 0.1)`
- border-strong: `rgba(255, 252, 248, 0.16)`
- accent-blue: `#4f9ae6`
- accent-blue-hover: `#6aabee`
- accent-blue-soft: `rgba(79, 154, 230, 0.16)`
- success: `#41a996`
- warning: `#d29a4b`
- danger: `#d87a74`

Rules:

- Accent blue is the only saturated product chrome color.
- Semantic colors appear only in badges, callouts, or validation states.
- Never use pure black or pure white for large desktop surfaces.

### 4.2 Border

- default divider and container border: `1px solid var(--border-whisper)`
- strong emphasis border: `1px solid var(--border-strong)`
- selected state: border plus accent tint background, not a thicker border alone

### 4.3 Radius

Only these radii are allowed:

- `4px`: inputs, buttons, compact chips
- `8px`: row cards, toolbars, segmented controls
- `12px`: panels, dialogs, metric cards, record cards
- `16px`: large page surfaces, hero panels, search overlay shell
- `9999px`: pills and status badges

### 4.4 Shadow

Use only low-contrast layered shadows:

- level-1: subtle container lift
- level-2: panel/card lift
- level-3: dialog/search overlay lift

Visual intent:

- shadows should be barely visible in light mode
- shadows should tighten, not glow, in dark mode

### 4.5 Typography

Global UI font stack:

- sans only: `"Inter", "SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`

Hierarchy:

| Role | Size | Weight | Line Height | Letter Spacing |
|------|------|--------|-------------|----------------|
| Page title | 30px | 700 | 1.15 | -0.03em |
| Section title | 22px | 700 | 1.25 | -0.02em |
| Card title | 16px | 600 | 1.35 | normal |
| Body | 14px | 400 | 1.5 | normal |
| UI label | 13px | 500 | 1.4 | normal |
| Caption | 12px | 500 | 1.35 | 0.01em |
| Badge | 12px | 600 | 1.2 | 0.02em |

Rules:

- UI chrome may scale with `fontSize` preference only.
- Serif or mono are allowed only inside content/editor surfaces that explicitly need them.
- Page titles are visually quiet: no oversized billboard heroes on desktop workbench pages.

### 4.6 Focus

- all keyboard-focusable controls must have a visible blue focus ring
- focus ring must be outside the element silhouette, not hidden by box-shadow resets
- focus ring style must be consistent across buttons, fields, tabs, rows, and cards

## 5. Workbench Shell

### 5.1 Sidebar

- full-height left workbench rail
- warm tinted background, not a card floating on the canvas
- width: `280px` expanded, `72px` collapsed rail
- section grouping through spacing and subtle labels
- selected navigation item uses accent-tinted fill and stronger text
- hover states remain neutral, not colorful

### 5.2 Topbar

- height: `48px`
- same page family as content area, separated by whisper border
- contains breadcrumbs/page label, global search trigger, preferences shortcuts, notifications, account entry
- no floating glass treatment

### 5.3 Search Overlay

- centered dialog shell, max width around `720px`
- composed of an input header and stacked result rows
- background is solid surface with dialog-level shadow

### 5.4 Main Canvas

- shell body uses the app canvas color
- content pages sit inside aligned page containers
- pages may choose standard width or full-bleed only when content type requires it, such as conversation or dashboards

### 5.5 Layering

- shell
- page surfaces
- inspector / contextual drawers
- search and dialogs
- toasts

## 6. Page Archetypes

### 6.1 Document Page

Used for overview, settings, dashboards, runtime summaries, and long-form configuration.

- page shell width: `min(1120px, 100%)`
- header block with eyebrow, title, description, actions
- body composed from stacked sections or panels
- metrics appear in a compact grid directly below the header

### 6.2 List / Detail Page

Used for tools, models, projects, project settings subareas, agents, resources.

- page header at top
- toolbar row below header
- two-column shell on desktop:
  - list pane `320px` to `420px`
  - detail pane flexible
- list rows use shared record-card language
- detail pane uses inspector/panel surfaces, not page-local framing

### 6.3 Conversation Page

Used for project conversations and assistant workflows.

- persistent message stream column
- optional right context panel
- composer docked at the bottom of the stream
- messages, approvals, tool calls, attachments, and composer share the same token language as the rest of the workbench
- no special chat-only radius, glow, or blur system

## 7. Components

### 7.1 Buttons

- primary: blue fill, white text, 4px radius
- secondary/outline: surface fill, whisper border, primary text
- ghost: transparent fill, neutral hover
- destructive: danger tint only when action semantics require it

### 7.2 Inputs

- height: `32px` standard
- border: whisper border
- background: solid surface
- placeholder uses tertiary text
- focus uses canonical ring

### 7.3 Badges

- default badges are pill shaped
- default tone is neutral or accent-soft
- semantic tones are low-saturation fills with darker text

### 7.4 Cards and Panels

- standard panel/card radius: `12px`
- border-first appearance
- optional soft shadow only where necessary
- avoid nested cards unless one surface is clearly acting as an inspector or inset section

### 7.5 Tabs and Toolbars

- tabs should read like understated workbench controls
- toolbar rows are horizontal organizers, not feature banners
- active tab uses accent underline or accent-tinted segmented fill

### 7.6 Dialogs and Popovers

- dialog shell uses 16px radius and level-3 shadow
- popovers use 12px radius and level-2 shadow
- both use solid surfaces; no blur

## 8. Motion

- default interaction transitions: `120ms` to `180ms`
- use opacity and subtle translate, not bounce or overshoot
- page-level motion should never obscure information hierarchy
- hover movement is minimal; color and border changes are preferred over lift animations

## 9. Preferences Policy

Allowed shell preferences:

- `theme`
- `locale`
- `fontSize`
- `leftSidebarCollapsed`
- `rightSidebarCollapsed`

Compatibility-only legacy fields:

- `fontFamily`
- `fontStyle`

Rules:

- legacy font fields may remain in persisted data for compatibility
- they must not affect global workbench chrome
- settings UI must not expose editing controls for them

## 10. Governance Checklist

Any new or changed desktop surface must satisfy all of the following:

- uses shared `@octopus/ui` primitives or extends them first
- consumes canonical tokens instead of page-local values
- matches one of the page archetypes
- respects the shell and theme rules in this document
- does not introduce forbidden patterns
- passes frontend governance checks
