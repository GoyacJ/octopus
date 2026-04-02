# AGENTS.md

## Frontend Governance

- Desktop frontend baseline: `Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2`.
- Shared UI must go through `@octopus/ui`. Business pages must not introduce ad-hoc third-party UI styles or bypass the shared design system.
- Component selection order:
  1. Reuse `@octopus/ui`.
  2. If missing, reference `shadcn-vue` interaction and structure patterns, but implement the component inside `@octopus/ui`.
  3. `Dialog`, `Popover`, `DropdownMenu`, `Combobox`, `Tabs`, `Accordion`, and `ContextMenu` must be built on `Reka UI` primitives.
- Styling must use `Tailwind CSS + design tokens` only. Do not mix multiple styling systems in the same surface.
- Forms default to `vee-validate + zod`.
- Tables default to `@tanstack/vue-table`.
- Long lists default to `@tanstack/vue-virtual`.
- Config and prompt editors default to `CodeMirror`. Use `Monaco` only for explicit IDE-class scenarios.
- Desktop capabilities should prefer Tauri native APIs for file picking, drag/drop import, tray, and window-level behavior.
- Icons default to `lucide-vue-next`. Brand or rare icons use `Iconify + unplugin-icons`.
- Motion policy:
  - system motion: `motion-v`
  - natural list, form, and settings transitions: `AutoAnimate`
  - welcome, highlight, and choreography surfaces: `GSAP`
  - character and state-machine animation: `Rive`
  - success, failure, loading, and empty-state assets: `dotLottie`
  - illustration: `unDraw`
- Forbidden patterns:
  - do not introduce large all-in-one UI frameworks
  - do not import unapproved UI libraries directly in business pages
  - do not deep import from `packages/ui/src/components/*`; consume the public `@octopus/ui` export surface only
