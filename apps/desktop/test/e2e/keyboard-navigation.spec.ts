import { expect, test } from '@playwright/test'

import { gotoAuthenticatedRoute } from './browserHost'

test('keeps search overlay keyboard navigation and focus return stable', async ({ page, browserName }) => {
  test.skip(browserName !== 'chromium', 'keyboard baseline runs on the shared chromium path')

  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/trace')

  const searchTrigger = page.getByTestId('global-search-trigger')
  await searchTrigger.focus()
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+K' : 'Control+K')

  const searchInput = page.getByTestId('search-overlay-input')
  await expect(searchInput).toBeFocused()

  await searchInput.press('ArrowDown')
  await expect(page.locator('[data-testid="search-overlay-results"] [data-active="true"]').first()).toBeVisible()

  await page.keyboard.press('Escape')
  await expect(page.getByTestId('search-overlay-panel')).toBeHidden()
  await expect(searchTrigger).toBeFocused()
})

test('supports keyboard activation on the durable task entry path', async ({ page }) => {
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/tasks?from=conversation&conversationId=conv-redesign')

  const backToConversation = page.getByTestId('project-tasks-back-to-conversation')
  await expect(backToConversation).toBeVisible()
  await backToConversation.focus()
  await expect(backToConversation).toBeFocused()

  await page.keyboard.press('Enter')
  await expect(page).toHaveURL(/\/conversations\/conv-redesign$/)
  await expect(page.getByTestId('conversation-composer')).toBeVisible()
})
