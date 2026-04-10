import { describe, expect, it } from 'vitest'

import {
  buildDownloadPageModel,
  detectCurrentPlatform,
  extractReleaseNoteSections,
  normalizeGithubReleases,
  type GithubReleaseApiResponse,
} from '../utils/github-releases'

const githubReleases: GithubReleaseApiResponse[] = [
  {
    id: 103,
    tag_name: 'preview-2026.04.10',
    name: 'Octopus Preview preview-2026.04.10',
    html_url: 'https://github.com/GoyacJ/octopus/releases/tag/preview-2026.04.10',
    body: '# Octopus Preview preview-2026.04.10\n\n发布日期：2026-04-10\n\n## 预览摘要\n\n这是来自 `main` 分支的预览构建。\n\n## 本次变更\n\n### Design Workbench Release\n\n桌面端现在采用统一的工作台设计系统。\n\n- 统一页面骨架\n- 收敛共享组件语言\n\n## 构建元数据\n\n- Release Tag：preview-2026.04.10\n',
    draft: false,
    prerelease: true,
    published_at: '2026-04-10T08:00:00Z',
    assets: [
      {
        id: 1001,
        name: 'Octopus_1.1.0_arm64.dmg',
        browser_download_url: 'https://downloads.example/macos-arm64-preview.dmg',
        size: 10,
      },
      {
        id: 1002,
        name: 'Octopus_1.1.0_x64.msi',
        browser_download_url: 'https://downloads.example/windows-x64-preview.msi',
        size: 11,
      },
      {
        id: 1003,
        name: 'Octopus_1.1.0_arm64.msi',
        browser_download_url: 'https://downloads.example/windows-arm64-preview.msi',
        size: 12,
      },
      {
        id: 1004,
        name: 'Octopus_1.1.0.AppImage',
        browser_download_url: 'https://downloads.example/linux-preview.AppImage',
        size: 13,
      },
      {
        id: 1005,
        name: 'latest.json',
        browser_download_url: 'https://downloads.example/latest.json',
        size: 1,
      },
    ],
  },
  {
    id: 102,
    tag_name: 'v1.0.2',
    name: 'Octopus v1.0.2',
    html_url: 'https://github.com/GoyacJ/octopus/releases/tag/v1.0.2',
    body: '# Octopus v1.0.2\n\n发布日期：2026-04-09\n\n## 版本概览\n\nOctopus `v1.0.2` 强化了桌面工作台体验。\n\n## 用户可感知变化\n\n- 修复平台安装包下载入口\n- 优化版本展示逻辑\n\n## 验证状态\n\n- 全仓质量门禁已通过\n',
    draft: false,
    prerelease: false,
    published_at: '2026-04-09T08:00:00Z',
    assets: [
      {
        id: 2001,
        name: 'Octopus_1.0.2_arm64.dmg',
        browser_download_url: 'https://downloads.example/macos-arm64.dmg',
        size: 21,
      },
      {
        id: 2002,
        name: 'Octopus_1.0.2_x64.dmg',
        browser_download_url: 'https://downloads.example/macos-x64.dmg',
        size: 22,
      },
      {
        id: 2003,
        name: 'Octopus_1.0.2_x64.msi',
        browser_download_url: 'https://downloads.example/windows-x64.msi',
        size: 23,
      },
      {
        id: 2004,
        name: 'Octopus_1.0.2_arm64.msi',
        browser_download_url: 'https://downloads.example/windows-arm64.msi',
        size: 24,
      },
      {
        id: 2005,
        name: 'Octopus_1.0.2_amd64.AppImage',
        browser_download_url: 'https://downloads.example/linux.AppImage',
        size: 25,
      },
      {
        id: 2006,
        name: 'Octopus_1.0.2_amd64.deb',
        browser_download_url: 'https://downloads.example/linux.deb',
        size: 26,
      },
      {
        id: 2007,
        name: 'Octopus_1.0.2_amd64.AppImage.sig',
        browser_download_url: 'https://downloads.example/linux.AppImage.sig',
        size: 2,
      },
    ],
  },
  {
    id: 101,
    tag_name: 'v1.0.1',
    name: 'Octopus v1.0.1',
    html_url: 'https://github.com/GoyacJ/octopus/releases/tag/v1.0.1',
    body: null,
    draft: false,
    prerelease: false,
    published_at: '2026-04-08T08:00:00Z',
    assets: [
      {
        id: 3001,
        name: 'Octopus_1.0.1_x64.dmg',
        browser_download_url: 'https://downloads.example/macos-x64-old.dmg',
        size: 31,
      },
    ],
  },
]

describe('github release normalization', () => {
  it('normalizes stable and preview release assets using repository naming rules', () => {
    const releases = normalizeGithubReleases(githubReleases)

    expect(releases).toHaveLength(3)
    expect(releases[0]?.channel).toBe('preview')
    expect(releases[1]?.channel).toBe('stable')
    expect(releases[0]?.noteSections.map((section) => section.title)).toEqual(['Design Workbench Release'])
    expect(releases[1]?.assets.map((asset) => `${asset.platform}:${asset.variantKey}`)).toEqual([
      'macos:appleSilicon',
      'macos:intel',
      'windows:x64',
      'windows:arm64',
      'linux:appImage',
      'linux:deb',
    ])
  })

  it('builds the download page model with current-platform hero assets and real platform cards', () => {
    const releases = normalizeGithubReleases(githubReleases)
    const model = buildDownloadPageModel(releases, 'macos')

    expect(model.latestStable?.tagName).toBe('v1.0.2')
    expect(model.latestPreview?.tagName).toBe('preview-2026.04.10')
    expect(model.heroPlatform).toBe('macos')
    expect(model.heroAssets.map((asset) => `${asset.channel}:${asset.variantKey}`)).toEqual([
      'stable:appleSilicon',
      'stable:intel',
      'preview:appleSilicon',
    ])
    expect(model.platformCards.map((card) => card.platform)).toEqual(['macos', 'windows', 'linux'])
    expect(model.platformCards[0]?.assets.map((asset) => `${asset.channel}:${asset.variantKey}`)).toEqual([
      'stable:appleSilicon',
      'stable:intel',
      'preview:appleSilicon',
    ])
    expect(model.history.map((release) => release.tagName)).toEqual([
      'preview-2026.04.10',
      'v1.0.2',
      'v1.0.1',
    ])
  })

  it('falls back to preview assets when the current platform has no latest stable build', () => {
    const releases = normalizeGithubReleases(githubReleases)
    const model = buildDownloadPageModel(releases, 'linux')

    expect(model.heroPlatform).toBe('linux')
    expect(model.heroAssets.map((asset) => `${asset.channel}:${asset.variantKey}`)).toEqual([
      'stable:appImage',
      'stable:deb',
      'preview:appImage',
    ])
  })

  it('extracts user-facing update sections and strips release metadata lines', () => {
    expect(extractReleaseNoteSections(githubReleases[0]?.body)).toEqual([
      {
        title: 'Design Workbench Release',
        items: ['桌面端现在采用统一的工作台设计系统。', '统一页面骨架', '收敛共享组件语言'],
      },
    ])
  })
})

describe('current platform detection', () => {
  it('maps browser user agents to supported platform keys', () => {
    expect(detectCurrentPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_4)')).toBe('macos')
    expect(detectCurrentPlatform('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('windows')
    expect(detectCurrentPlatform('Mozilla/5.0 (X11; Linux x86_64)')).toBe('linux')
    expect(detectCurrentPlatform('Mozilla/5.0 (FreeBSD)')).toBe('macos')
  })
})
