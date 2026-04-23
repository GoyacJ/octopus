import { expect, test } from '@playwright/test'

import { gotoAuthenticatedRoute } from './browserHost'

const isMac = process.platform === 'darwin'

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' })
})

test('keeps the search dialog and account popover usable with reduced motion', async ({ page }) => {
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/trace')

  const searchTrigger = page.getByTestId('global-search-trigger')
  await searchTrigger.focus()
  await page.keyboard.press(isMac ? 'Meta+K' : 'Control+K')

  const searchPanel = page.getByTestId('search-overlay-panel')
  const dialogContent = page.locator('[data-ui-dialog-content="true"]')
  const searchInput = page.getByTestId('search-overlay-input')
  await expect(searchPanel).toBeVisible()
  await expect(dialogContent).toHaveAttribute('data-reduced-motion', 'true')
  await expect(searchInput).toBeFocused()

  await page.keyboard.press('Escape')
  await expect(searchPanel).toBeHidden()
  await expect(searchTrigger).toBeFocused()

  const profileTrigger = page.getByTestId('topbar-profile-trigger')
  await profileTrigger.focus()
  await page.keyboard.press('Enter')

  const popoverContent = page.getByTestId('ui-popover-content')
  await expect(popoverContent).toBeVisible()
  await expect(popoverContent).toHaveAttribute('data-reduced-motion', 'true')

  await page.keyboard.press('Escape')
  await expect(popoverContent).toBeHidden()
  await expect(profileTrigger).toBeFocused()
})
