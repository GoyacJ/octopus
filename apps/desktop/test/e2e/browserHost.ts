import { expect, type Page } from '@playwright/test'

const ownerAvatarPng = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wn7n6kAAAAASUVORK5CYII=',
  'base64',
)
const ownerUsername = 'owner'
const ownerDisplayName = 'Workspace Owner'
const ownerPassword = 'secret-123'

async function resolveShellState(page: Page): Promise<'workbench' | 'auth'> {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (await page.getByTestId('workbench-shell').isVisible().catch(() => false)) {
      return 'workbench'
    }

    if (await page.getByTestId('browser-auth-login-view').isVisible().catch(() => false)) {
      return 'auth'
    }

    await page.waitForTimeout(250)
  }

  throw new Error('Timed out waiting for browser host shell or auth gate')
}

async function resolveWorkspaceApiBaseUrl(page: Page): Promise<string> {
  const value = (await page.getByTestId('browser-auth-intro').locator('p').last().textContent())?.trim()
  if (!value?.startsWith('http://127.0.0.1:')) {
    throw new Error(`Unable to resolve browser host API base URL from auth intro: ${value ?? '<empty>'}`)
  }

  return value
}

async function bootstrapOwnerViaApi(page: Page) {
  const baseUrl = await resolveWorkspaceApiBaseUrl(page)
  const response = await page.request.post(`${baseUrl}/api/v1/system/auth/bootstrap-admin`, {
    data: {
      clientAppId: 'octopus-web',
      workspaceId: 'ws-local',
      username: ownerUsername,
      displayName: ownerDisplayName,
      password: ownerPassword,
      confirmPassword: ownerPassword,
      avatar: {
        fileName: 'owner-avatar.png',
        contentType: 'image/png',
        dataBase64: ownerAvatarPng.toString('base64'),
        byteSize: ownerAvatarPng.byteLength,
      },
    },
  })

  if (!response.ok()) {
    throw new Error(`Bootstrap admin API failed: ${response.status()} ${await response.text()}`)
  }
}

export async function registerFirstOwnerIfNeeded(page: Page) {
  let shellState = await resolveShellState(page)
  if (shellState !== 'auth') {
    return
  }

  const avatarPickButton = page.getByTestId('auth-gate-avatar-pick')
  const isRegisterMode = await avatarPickButton.isVisible().catch(() => false)

  if (isRegisterMode) {
    await bootstrapOwnerViaApi(page)
    await page.reload()
    await page.waitForLoadState('domcontentloaded')
    shellState = await resolveShellState(page)
    if (shellState !== 'auth') {
      return
    }
  }

  await page.getByTestId('auth-gate-username-input').fill(ownerUsername)
  await page.getByTestId('auth-gate-password-input').fill(ownerPassword)
  await page.getByTestId('auth-gate-submit').click()

  await expect(page.getByTestId('workbench-shell')).toBeVisible({ timeout: 20_000 })
}

export async function gotoAuthenticatedRoute(page: Page, route: string) {
  await page.goto(route)
  await page.waitForLoadState('domcontentloaded')
  await registerFirstOwnerIfNeeded(page)
}
