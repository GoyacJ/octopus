import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'

import { expect, test } from '@playwright/test'

import { gotoAuthenticatedRoute } from './browserHost'

interface FrontendPerformanceMetric {
  id: 'shell-startup' | 'search-overlay-open' | 'conversation-route-ready' | 'trace-route-ready'
  label: string
  route: string
  readyMarker: string
  durationMs: number
}

function roundDuration(durationMs: number) {
  return Math.round(durationMs * 100) / 100
}

async function measureMetric(
  run: () => Promise<void>,
  metric: Omit<FrontendPerformanceMetric, 'durationMs'>,
): Promise<FrontendPerformanceMetric> {
  const startedAt = performance.now()
  await run()
  return {
    ...metric,
    durationMs: roundDuration(performance.now() - startedAt),
  }
}

async function writePerformanceReport(metrics: FrontendPerformanceMetric[]) {
  const reportPath = process.env.PLAYWRIGHT_FRONTEND_PERF_OUTPUT
  if (!reportPath) {
    return
  }

  await mkdir(path.dirname(reportPath), { recursive: true })
  await writeFile(reportPath, JSON.stringify({
    generatedAt: new Date().toISOString(),
    mode: 'report-only',
    metrics,
  }, null, 2))
}

test('captures a report-only browser-host frontend performance baseline', async ({ page, context }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' })
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/trace')

  const measurementPage = await context.newPage()
  await measurementPage.emulateMedia({ reducedMotion: 'reduce' })

  const metrics: FrontendPerformanceMetric[] = []

  metrics.push(await measureMetric(async () => {
    await measurementPage.goto('/#/workspaces/ws-local/projects/proj-redesign/dashboard')
    await expect(measurementPage.getByTestId('workbench-shell')).toBeVisible()
    await expect(measurementPage.getByTestId('global-search-trigger')).toBeVisible()
    await expect(measurementPage.getByTestId('project-dashboard-view')).toBeVisible()
  }, {
    id: 'shell-startup',
    label: 'Authenticated shell startup',
    route: '/#/workspaces/ws-local/projects/proj-redesign/dashboard',
    readyMarker: 'workbench-shell + global-search-trigger + project-dashboard-view',
  }))

  metrics.push(await measureMetric(async () => {
    await measurementPage.getByTestId('global-search-trigger').click()
    await expect(measurementPage.getByTestId('search-overlay-panel')).toBeVisible()
    await expect(measurementPage.getByTestId('search-overlay-input')).toBeFocused()
  }, {
    id: 'search-overlay-open',
    label: 'Global search overlay open',
    route: '/#/workspaces/ws-local/projects/proj-redesign/dashboard',
    readyMarker: 'search-overlay-panel + search-overlay-input focus',
  }))

  await measurementPage.keyboard.press('Escape')
  await expect(measurementPage.getByTestId('search-overlay-panel')).toBeHidden()

  metrics.push(await measureMetric(async () => {
    await measurementPage.goto('/#/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await expect(measurementPage.getByTestId('conversation-composer')).toBeVisible()
    await expect(measurementPage.getByTestId('conversation-composer-input')).toBeVisible()
  }, {
    id: 'conversation-route-ready',
    label: 'Conversation route ready',
    route: '/#/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign',
    readyMarker: 'conversation-composer + conversation-composer-input',
  }))

  metrics.push(await measureMetric(async () => {
    await measurementPage.goto('/#/workspaces/ws-local/projects/proj-redesign/trace')
    await expect(measurementPage.getByTestId('trace-view')).toBeVisible()
  }, {
    id: 'trace-route-ready',
    label: 'Trace route ready',
    route: '/#/workspaces/ws-local/projects/proj-redesign/trace',
    readyMarker: 'trace-view',
  }))

  await writePerformanceReport(metrics)
  await measurementPage.close()
})
