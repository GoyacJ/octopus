import { expect, test } from '@playwright/test'

import { gotoAuthenticatedRoute } from './browserHost'

test('boots the browser host shell and exercises core project navigation', async ({ page }) => {
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/trace')

  await expect(page.getByTestId('workbench-shell')).toBeVisible()
  await expect(page.getByTestId('global-search-trigger')).toBeVisible()
  await expect(page.getByTestId('trace-view')).toBeVisible()

  await page.getByTestId('global-search-trigger').click()
  await expect(page.getByTestId('search-overlay-panel')).toBeVisible()
  await expect(page.getByTestId('search-overlay-input')).toBeFocused()

  await page.keyboard.press('Escape')
  await expect(page.getByTestId('search-overlay-panel')).toBeHidden()

  await page.getByTestId('sidebar-project-module-proj-redesign-tasks').click()
  await expect(page.getByTestId('project-tasks-view')).toBeVisible()
})
