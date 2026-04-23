import { defineConfig, devices } from '@playwright/test'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:15421'
const basePort = Number(new URL(baseURL).port || 80)
const desktopRoot = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(desktopRoot, '..', '..')

export default defineConfig({
  testDir: './test/e2e',
  outputDir: './test-results/e2e',
  timeout: 60_000,
  expect: {
    timeout: 10_000,
  },
  fullyParallel: false,
  workers: 1,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: {
    ...devices['Desktop Chrome'],
    baseURL,
    headless: true,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'off',
    testIdAttribute: 'data-testid',
  },
  webServer: {
    command: `${process.execPath} ${join(desktopRoot, 'test', 'e2e', 'startBrowserHost.mjs')}`,
    cwd: repoRoot,
    env: {
      ...process.env,
      CI: '1',
    },
    port: basePort,
    timeout: 180_000,
    reuseExistingServer: false,
    gracefulShutdown: {
      signal: 'SIGTERM',
      timeout: 10_000,
    },
  },
})
