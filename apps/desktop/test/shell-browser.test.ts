// @vitest-environment jsdom

import JSZip from 'jszip'
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

  it('mounts transient file inputs in the document when importing agent bundle archives', async () => {
    const originalCreateElement = document.createElement.bind(document)
    const archive = new JSZip()
    archive.file('templates/agent.md', '# Agent')
    const zipBlob = await archive.generateAsync({ type: 'blob' })
    const zipFile = new File([zipBlob], 'templates.zip', { type: 'application/zip' })

    vi.spyOn(document, 'createElement').mockImplementation(((tagName: string, options?: ElementCreationOptions) => {
      const element = originalCreateElement(tagName, options)
      if (tagName !== 'input') {
        return element
      }

      const input = element as HTMLInputElement
      const files = Object.assign([zipFile], {
        item: (index: number) => [zipFile][index] ?? null,
      }) as unknown as FileList

      Object.defineProperty(input, 'files', {
        configurable: true,
        get: () => files,
      })
      vi.spyOn(input, 'click').mockImplementation(() => {
        expect(document.body.contains(input)).toBe(true)
        input.dispatchEvent(new Event('change'))
      })

      return input
    }) as typeof document.createElement)

    await browserShellClient.pickAgentBundleArchive()

    expect(document.querySelector('input[type="file"]')).toBeNull()
  })

  it('mounts a transient anchor element when exporting agent bundles as zip', async () => {
    Object.defineProperty(URL, 'createObjectURL', {
      configurable: true,
      value: vi.fn(() => 'blob:octopus-test'),
    })
    Object.defineProperty(URL, 'revokeObjectURL', {
      configurable: true,
      value: vi.fn(),
    })
    const clickSpy = vi
      .spyOn(HTMLAnchorElement.prototype, 'click')
      .mockImplementation(function click(this: HTMLAnchorElement) {
        expect(document.body.contains(this)).toBe(true)
      })

    await browserShellClient.saveAgentBundleExport({
      rootDirName: 'finance-bundle',
      fileCount: 1,
      agentCount: 1,
      teamCount: 0,
      skillCount: 0,
      mcpCount: 0,
      avatarCount: 0,
      issues: [],
      files: [{
        fileName: 'agent.md',
        relativePath: 'finance-bundle/agent.md',
        contentType: 'text/markdown',
        byteSize: 3,
        dataBase64: 'Zm9v',
      }],
    }, 'zip')

    expect(clickSpy).toHaveBeenCalledOnce()
    expect(document.querySelector('a[download="finance-bundle.zip"]')).toBeNull()
  })
})
