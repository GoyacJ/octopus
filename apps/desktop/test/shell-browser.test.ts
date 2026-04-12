// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from 'vitest'

import { browserShellClient } from '@/tauri/shell_browser'

describe('browser shell resource pickers', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    delete (window as typeof window & { showDirectoryPicker?: unknown }).showDirectoryPicker
  })

  it('uses the File System Access API when available', async () => {
    ;(window as typeof window & {
      showDirectoryPicker?: () => Promise<{ name?: string }>
    }).showDirectoryPicker = vi.fn(async () => ({ name: 'project-resources' }))

    await expect(browserShellClient.pickResourceDirectory()).resolves.toBe('/project-resources')
  })

  it('falls back to a directory input when the File System Access API is unavailable', async () => {
    const originalCreateElement = document.createElement.bind(document)
    const input = originalCreateElement('input')
    const folderFile = new File(['# Folder'], 'brief.md', { type: 'text/markdown' })
    Object.defineProperty(folderFile, 'webkitRelativePath', {
      configurable: true,
      value: 'project-resources/brief.md',
    })
    Object.defineProperty(input, 'files', {
      configurable: true,
      get: () => [folderFile],
    })
    vi.spyOn(input, 'click').mockImplementation(() => {
      input.dispatchEvent(new Event('change'))
    })
    vi.spyOn(document, 'createElement').mockImplementation(((tagName: string) => {
      if (tagName === 'input') {
        return input
      }
      return originalCreateElement(tagName)
    }) as typeof document.createElement)

    await expect(browserShellClient.pickResourceDirectory()).resolves.toBe('/project-resources')
  })
})
