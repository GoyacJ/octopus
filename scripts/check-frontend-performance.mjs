import { spawn } from 'node:child_process'
import { mkdir, readFile, rm } from 'node:fs/promises'
import path from 'node:path'

const repoRoot = path.resolve(import.meta.dirname, '..')
const pnpmCommand = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
const reportDir = path.join(repoRoot, 'tmp', 'frontend-performance')
const reportPath = path.join(reportDir, 'latest.json')

function waitForExit(child) {
  return new Promise((resolve) => {
    child.once('exit', (code, signal) => resolve({ code, signal }))
  })
}

async function runPlaywrightPerformanceCheck() {
  const child = spawn(pnpmCommand, [
    'exec',
    'playwright',
    'test',
    '--config',
    'apps/desktop/playwright.config.ts',
    'apps/desktop/test/e2e/performance.spec.ts',
  ], {
    cwd: repoRoot,
    env: {
      ...process.env,
      PLAYWRIGHT_FRONTEND_PERF_OUTPUT: reportPath,
    },
    stdio: 'inherit',
    shell: process.platform === 'win32',
  })

  const result = await waitForExit(child)
  if (result.signal) {
    process.kill(process.pid, result.signal)
    return
  }
  if ((result.code ?? 1) !== 0) {
    throw new Error('Playwright frontend performance baseline failed')
  }
}

function formatMetric(metric) {
  return `- ${metric.label}: ${metric.durationMs.toFixed(2)} ms (${metric.readyMarker})`
}

async function main() {
  await mkdir(reportDir, { recursive: true })
  await rm(reportPath, { force: true })

  await runPlaywrightPerformanceCheck()

  const report = JSON.parse(await readFile(reportPath, 'utf8'))
  const metrics = Array.isArray(report.metrics) ? report.metrics : []

  if (metrics.length === 0) {
    throw new Error('Frontend performance report is empty')
  }

  const totalDurationMs = metrics.reduce((total, metric) => total + Number(metric.durationMs || 0), 0)

  console.log('\nFrontend performance baseline')
  console.log(`Generated: ${report.generatedAt}`)
  console.log(`Mode: ${report.mode}`)
  console.log(`Report: ${path.relative(repoRoot, reportPath)}`)
  for (const metric of metrics) {
    console.log(formatMetric(metric))
  }
  console.log(`Total sampled duration: ${totalDurationMs.toFixed(2)} ms`)
}

await main()
