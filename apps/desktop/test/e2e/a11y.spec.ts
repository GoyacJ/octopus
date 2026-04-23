import AxeBuilder from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'

import { gotoAuthenticatedRoute } from './browserHost'

async function expectNoBlockingViolations(page: Page, selectors: string[]) {
  const builder = new AxeBuilder({ page })

  for (const selector of selectors) {
    builder.include(selector)
  }

  const results = await builder.analyze()
  const blockingViolations = results.violations.filter(violation => violation.impact === 'critical')

  expect(
    blockingViolations.map(violation => ({
      id: violation.id,
      impact: violation.impact,
      targets: violation.nodes.map(node => node.target.join(' ')),
    })),
  ).toEqual([])
}

test('keeps conversation live-region and composer semantics accessible', async ({ page }) => {
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
  const messageList = page.getByTestId('conversation-message-list')
  const composer = page.getByTestId('conversation-composer')
  const composerInput = page.getByTestId('conversation-composer-input')

  await expect(messageList).toBeVisible()
  await expect(messageList).toHaveAttribute('role', 'log')
  await expect(messageList).toHaveAttribute('aria-live', 'polite')
  await expect(composer).toBeVisible()
  await expect(composerInput).toHaveAttribute('aria-label', /完成会话配置后即可输入消息|Type your message/i)
  await expect(page.getByTestId('conversation-setup-callout').getByTestId('conversation-setup-open-models')).toBeVisible()
})

test('passes blocking a11y checks on the trace surface', async ({ page }) => {
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/trace')
  await expect(page.getByTestId('trace-view')).toBeVisible()
  await expectNoBlockingViolations(page, ['[data-testid="trace-view"]'])
})

test('passes blocking a11y checks on the project task surface', async ({ page }) => {
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/tasks')
  await expect(page.getByTestId('project-tasks-view')).toBeVisible()
  await expectNoBlockingViolations(page, ['[data-testid="project-tasks-view"]'])
})

test('passes blocking a11y checks with the global search overlay open', async ({ page }) => {
  await gotoAuthenticatedRoute(page, '/#/workspaces/ws-local/projects/proj-redesign/trace')

  await page.getByTestId('global-search-trigger').click()
  await expect(page.getByTestId('search-overlay-panel')).toBeVisible()
  await expect(page.getByTestId('search-overlay-input')).toBeFocused()

  await expectNoBlockingViolations(page, ['[data-testid="search-overlay-panel"]'])
})
