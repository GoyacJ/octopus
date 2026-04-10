import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const websiteRoot = path.resolve(import.meta.dirname, '..')
const downloadPagePath = path.join(websiteRoot, 'pages', 'download.vue')
const dashboardScreenshotPath = path.join(websiteRoot, 'public', 'screenshots', 'dashboard.png')

describe('download page assets', () => {
  it('references the shared dashboard screenshot from the public screenshot registry', () => {
    const page = readFileSync(downloadPagePath, 'utf8')

    expect(page).toContain('src="/screenshots/dashboard.png"')
  })

  it('uses live release data and a dropdown-based hero download trigger', () => {
    const page = readFileSync(downloadPagePath, 'utf8')

    expect(page).not.toContain("const version = '1.0.2'")
    expect(page).toContain('useGithubReleases')
    expect(page).toContain('<UiPopover')
    expect(page).toContain('release.noteSections')
    expect(page).toContain("document.getElementById('platform-selection')")
    expect(page).toContain('align="center"')
  })

  it('keeps the dashboard screenshot asset available in public/screenshots', () => {
    expect(existsSync(dashboardScreenshotPath)).toBe(true)
  })

  it('does not expose GitHub wording in the page source copy', () => {
    const page = readFileSync(downloadPagePath, 'utf8')

    expect(page).not.toContain('GitHub Release')
    expect(page).not.toContain('GitHub Releases')
  })
})
