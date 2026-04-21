# 2026-04-21 · UI Performance & A11y (可度量稳定底线)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

把 "稳定流畅" 从形容词转成 CI 可校验的数字：为桌面 UI 建立性能硬指标（INP / 滚动帧率 / 冷启动 LCP / 主包体积）与无障碍硬指标（axe 零违规 / tab-order / 对比度 / prefers-reduced-motion 回归），并把检测接入 `pnpm check:desktop` 流水线。

## Architecture

性能端：视口外渲染用 CSS `content-visibility`；媒体资产（Rive/Lottie）用 IntersectionObserver 按需初始化；打包层面所有 motion/animation 库走 **动态 import**，不进主 bundle。A11y 端：接入 `@axe-core/playwright` 与自定义 tab-order 快照；`prefers-reduced-motion` 用 Playwright emulate 自动切换分支验证。所有指标由脚本产出报告并在 CI 输出阈值比较。

## Scope

- In scope:
  - `UiMetricCard / UiRecordCard / UiAreaChart / UiDonutChart` 等视图密集组件加 `content-visibility: auto`
  - `UiDotLottie / UiRiveCanvas` 接入 IntersectionObserver，进入视口才初始化
  - 主包动态 import motion 库（GSAP、Rive、Lottie）
  - 性能硬指标基线 + Playwright 用例：INP、滚动 P95 帧率、冷启动 LCP
  - A11y 基线 + Playwright 用例：axe、tab-order、对比度
  - `prefers-reduced-motion` 回归用例
  - `scripts/check-frontend-performance.mjs`（新脚本，接入 `check:desktop`）
- Out of scope:
  - Rust 端冷启动优化
  - 网络请求优化（adapter 层）
  - 数据库读写性能（crates 层）

## Risks Or Open Questions

- Playwright 在 Tauri 环境下不直连 webview，需要走 `apps/desktop/src/mocks` 或 browser host 模式跑基线。冷启动 LCP 只能在 browser host 下测，Tauri 端走原生工具（另立）。
- `content-visibility: auto` 对含粘性定位或滚动吸附的子元素有副作用，需先在 `UiAreaChart` 验证。
- GSAP / Rive 动态 import 会导致首次使用有 ~100ms loading delay，需在入口组件加 `UiSkeleton` 占位（依赖 P2 Task 1）。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a business-page-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.
- 任何指标新阈值先软阈值（警告）一轮，稳定后再转硬阈值（阻塞）。

## Task Ledger

### Task 1: 视口外渲染隔离

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiMetricCard.vue`
- Modify: `packages/ui/src/components/UiRecordCard.vue`
- Modify: `packages/ui/src/components/UiAreaChart.vue`
- Modify: `packages/ui/src/components/UiDonutChart.vue`
- Modify: `packages/ui/src/components/UiSparkline.vue`
- Modify: `packages/ui/src/tokens.css`（新增 `--ui-card-intrinsic-height`）

Preconditions:
- 无。

Step 1:
- Action: 在上述组件根节点添加 CSS：`content-visibility: auto; contain-intrinsic-size: auto var(--ui-card-intrinsic-height, 180px);`；tokens.css 新增 `--ui-card-intrinsic-height: 180px`（可被业务页覆盖但不推荐）。
- Done when: 5 个组件视觉无回归；在 Chrome DevTools Rendering 面板开 "Content visibility" 能看到视口外绿框。
- Verify: `pnpm -C apps/desktop build:ui && pnpm -C apps/desktop test` 通过；人工验证长列表滚动无跳动。
- Stop if: `UiAreaChart` 的 tooltip 定位因 content-visibility 失效（需改回退方案：仅卡片容器开，不含 tooltip 层）。

### Task 2: Lottie/Rive 按需加载

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiDotLottie.vue`
- Modify: `packages/ui/src/components/UiRiveCanvas.vue`
- Modify: `packages/ui/src/components/__tests__/UiDotLottie.test.ts`
- Modify: `packages/ui/src/components/__tests__/UiRiveCanvas.test.ts`

Preconditions:
- 无。

Step 1:
- Action: 两组件内部用 `IntersectionObserver` 监听根节点；未进入视口时只渲染占位（`div` + `aria-hidden`），不加载 runtime；进入视口后再 `import(...)` 对应 runtime 包并初始化。离开视口时暂停（pause），不销毁。
- Done when: 首屏主 bundle 不包含 `@lottiefiles/dotlottie-vue` 与 `@rive-app/canvas`（bundle analyzer 验证）；视口内动画首帧延迟 < 150ms；单测覆盖"未进入视口不初始化"、"进入视口初始化"、"离开视口暂停"。
- Verify: `pnpm -C apps/desktop build:ui` 后跑 bundle analyzer（`pnpm -C apps/desktop exec vite build --mode analyze` 或新增脚本）；`pnpm -C apps/desktop test -- UiDotLottie UiRiveCanvas` 通过。
- Stop if: `IntersectionObserver` 在 Tauri webview 不触发（极少数场景，需 polyfill 或改 visibility observer）。

### Task 3: 主包动态 import motion 库

Status: `pending`

Files:
- Modify: `packages/ui/src/lib/motion.ts`
- Modify: `apps/desktop/src/**/*.ts`（所有直接 `import gsap` / `from '@rive-app/...'` 的入口）
- Modify: `apps/desktop/vite.config.ts`（如需配置 `build.rollupOptions.output.manualChunks`）

Preconditions:
- Task 2 已完成。

Step 1:
- Action: 所有 `gsap` / `@rive-app` / `@lottiefiles` 的 import 改为动态 `import()`，并在组件入口处提供 `UiSkeleton` 占位；`apps/desktop/vite.config.ts` 里对 motion 库配置独立 chunk，命名 `motion-vendor`。
- Done when: 主 bundle `entry` chunk 不含上述三库；首屏 LCP 提升可测。
- Verify: `pnpm -C apps/desktop build:ui` 后通过 bundle visualizer 观察；首屏 LCP 对比基线。
- Stop if: 某业务页同步依赖 GSAP Timeline 的同步 API（需改为 async 初始化或切换到 `motion-v`）。

### Task 4: 性能硬指标基线 + Playwright

Status: `pending`

Files:
- Create: `apps/desktop/tests/performance/interaction.spec.ts`
- Create: `apps/desktop/tests/performance/scroll.spec.ts`
- Create: `apps/desktop/tests/performance/lcp.spec.ts`
- Create: `scripts/check-frontend-performance.mjs`
- Modify: `package.json`（加 `check:frontend-performance` script）

Preconditions:
- Task 1 + Task 2 + Task 3 已完成。
- `apps/desktop` 已能在 browser host 模式启动（用于 Playwright）。

Step 1:
- Action: 实现 3 个 Playwright 用例：
  1. `interaction.spec.ts`：对关键交互（点击按钮、打开 dialog、提交表单）测 INP，阈值 **P95 < 200ms**。
  2. `scroll.spec.ts`：对长列表滚动测帧率，阈值 **P95 ≥ 55fps**。
  3. `lcp.spec.ts`：冷启动 LCP，阈值 **< 1200ms**（browser host 模式）。
- Done when: 3 用例稳定通过；阈值先以软阈值（警告）跑 1 周，稳定后改硬阈值。
- Verify: `pnpm exec playwright test apps/desktop/tests/performance/` 通过。
- Stop if: browser host 启动失败，需先修 dev server（不属于本计划范围）。

Step 2:
- Action: `scripts/check-frontend-performance.mjs` 汇总 3 个用例的输出，写一份 `docs/perf-baseline.md` 快照，接入 `check:desktop` 作为可选步骤（`PERF=1 pnpm check:desktop`）。
- Done when: 脚本可跑；文档产出。
- Verify: `PERF=1 pnpm check:desktop` 通过。
- Stop if: CI 机器不稳定导致指标抖动 > 20%，需在 scripts 里加统计平均与重试。

### Task 5: A11y 基线 + axe + tab-order

Status: `pending`

Files:
- Create: `apps/desktop/tests/a11y/axe.spec.ts`
- Create: `apps/desktop/tests/a11y/tab-order.spec.ts`
- Create: `apps/desktop/tests/a11y/reduced-motion.spec.ts`
- Modify: `package.json`（加 `check:a11y` script）
- Modify: `packages/ui/src/components/*.vue`（修补 axe 报告的违规）

Preconditions:
- 无。

Step 1:
- Action: 用 `@axe-core/playwright` 覆盖 10 个代表性页面（shell、会话、agents 列表、agents 详情、工具列表、工具详情、settings、个人中心、权限中心、empty）；违规必须清零后才能合并。
- Done when: 10 页 axe report 零违规；脚本 `pnpm check:a11y` 在 CI 中阻塞合并。
- Verify: `pnpm exec playwright test apps/desktop/tests/a11y/axe.spec.ts` 通过。
- Stop if: 违规数过多（> 50），先分批修补并用白名单（文件级豁免）过渡。

Step 2:
- Action: `tab-order.spec.ts` 对每页按 Tab 键序列化焦点元素，产出 snapshot 存 `apps/desktop/tests/a11y/__snapshots__/`；后续变更需同步快照。
- Done when: 快照稳定；人工检查焦点顺序符合视觉阅读顺序。
- Verify: 同上。
- Stop if: 快照频繁抖动（Reka UI 的 portal 会打乱顺序），需在快照里白名单处理。

Step 3:
- Action: `reduced-motion.spec.ts` 用 `page.emulateMedia({ reducedMotion: 'reduce' })` 验证 Skeleton / Toast / Dialog 等组件不再动画，但可见性与交互不变。
- Done when: 用例通过；覆盖至少 5 个含动画的组件。
- Verify: 同上。
- Stop if: 某组件动画是信息载体（如 progress spinner），需有"仍传达状态"的静态替代。

### Task 6: 对比度 token 校验

Status: `pending`

Files:
- Create: `scripts/check-color-contrast.mjs`
- Modify: `package.json`（加 `check:color-contrast` script）

Preconditions:
- 无。

Step 1:
- Action: 脚本读取 `packages/ui/src/tokens.css`，提取 light/dark 两 theme 的关键对比组合（text-primary ↔ canvas、text-secondary ↔ surface、accent ↔ surface、danger ↔ surface-muted），按 WCAG 2.1 AA 阈值（正常文本 4.5、大文本 3.0、UI 组件 3.0）计算并断言。
- Done when: 脚本在本仓库全通过；若失败给出具体组合与当前比例。
- Verify: `pnpm check:color-contrast` 通过。
- Stop if: 某组合在 dark mode 不过线（历史值），需设计师介入调整 token。

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 -> Task 2
- Completed: short list
- Verification:
  - `pnpm -C apps/desktop build:ui` -> pass
  - bundle visualizer -> motion-vendor chunk split ok
  - `pnpm check:a11y` -> pass
- Blockers:
  - none
- Next:
  - Task 3
```
