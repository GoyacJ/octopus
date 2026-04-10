export type DownloadPlatform = 'macos' | 'windows' | 'linux'
export type DownloadChannel = 'stable' | 'preview'
export type DownloadVariantKey = 'appleSilicon' | 'intel' | 'x64' | 'arm64' | 'appImage' | 'deb'

export interface GithubReleaseAssetApiResponse {
  id: number
  name: string
  browser_download_url: string
  size: number
}

export interface GithubReleaseApiResponse {
  id: number
  tag_name: string
  name: string | null
  html_url: string
  body: string | null
  draft: boolean
  prerelease: boolean
  published_at: string
  assets: GithubReleaseAssetApiResponse[]
}

export interface ReleaseNoteSection {
  title: string
  items: string[]
}

export interface NormalizedAsset {
  id: number
  name: string
  downloadUrl: string
  size: number
  platform: DownloadPlatform
  variantKey: DownloadVariantKey
  variantLabel: string
  fileExtension: string
  channel: DownloadChannel
  releaseTag: string
}

export interface NormalizedRelease {
  id: number
  tagName: string
  name: string
  publishedAt: string
  htmlUrl: string
  channel: DownloadChannel
  assets: NormalizedAsset[]
  noteSections: ReleaseNoteSection[]
}

export interface PlatformCard {
  platform: DownloadPlatform
  assets: NormalizedAsset[]
}

export interface DownloadPageModel {
  latestStable: NormalizedRelease | null
  latestPreview: NormalizedRelease | null
  heroPlatform: DownloadPlatform
  heroAssets: NormalizedAsset[]
  platformCards: PlatformCard[]
  history: NormalizedRelease[]
}

const platformOrder: DownloadPlatform[] = ['macos', 'windows', 'linux']
const variantOrder: Record<DownloadVariantKey, number> = {
  appleSilicon: 0,
  intel: 1,
  x64: 0,
  arm64: 1,
  appImage: 0,
  deb: 1,
}

const variantLabels: Record<DownloadVariantKey, string> = {
  appleSilicon: 'Apple Silicon',
  intel: 'Intel',
  x64: 'x64',
  arm64: 'ARM64',
  appImage: 'AppImage',
  deb: 'DEB',
}

const ignoredSectionTitles = new Set([
  '构建元数据',
  '发布元数据',
  '验证状态',
  '构建产物',
  '技术附录',
  '自动汇总变更',
  'Preview Metadata',
  'Governance Checks',
  'Release Governance',
])

const ignoredNoteLinePrefixes = [
  '发布日期：',
  '发布渠道：',
  'VERSION：',
  'Release Tag：',
  'Commit SHA：',
  'Run Number：',
  '变更范围：',
  'Checksums：',
  'Release date:',
  'Release channel:',
  'Run number:',
  'Commit SHA:',
]

function cleanMarkdownText(value: string) {
  return value
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/[`*_~>#]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
}

function shouldIgnoreNoteLine(line: string) {
  const normalizedLine = line
    .replace(/^[-*+]\s+/, '')
    .replace(/^\d+\.\s+/, '')
    .replace(/^\[[ xX]\]\s+/, '')
    .replace(/`/g, '')

  return ignoredNoteLinePrefixes.some((prefix) => normalizedLine.startsWith(prefix))
    || normalizedLine === '---'
    || normalizedLine === '***'
    || normalizedLine === '___'
    || normalizedLine.startsWith('Merge pull request')
    || normalizedLine.includes('main branch')
    || normalizedLine.includes('main 分支')
    || normalizedLine.includes('github.com/')
    || normalizedLine.includes('GoyacJ/')
}

function finalizeNoteSection(section: ReleaseNoteSection | null) {
  if (!section) {
    return null
  }

  const items = Array.from(new Set(section.items))
    .map((item) => cleanMarkdownText(item))
    .filter(Boolean)
    .slice(0, 4)

  if (items.length === 0) {
    return null
  }

  return {
    title: section.title,
    items,
  } satisfies ReleaseNoteSection
}

export function extractReleaseNoteSections(body: string | null | undefined): ReleaseNoteSection[] {
  if (!body) {
    return []
  }

  const sections: ReleaseNoteSection[] = []
  let currentSection: ReleaseNoteSection | null = null
  let isIgnoringSection = false

  for (const rawLine of body.split('\n')) {
    const line = rawLine.trim()

    if (!line) {
      continue
    }

    if (line.startsWith('#')) {
      const heading = cleanMarkdownText(line.replace(/^#+\s*/, ''))

      if (!heading || heading.startsWith('Octopus ')) {
        continue
      }

      const finalizedSection = finalizeNoteSection(currentSection)
      if (finalizedSection && !ignoredSectionTitles.has(finalizedSection.title)) {
        sections.push(finalizedSection)
      }

      isIgnoringSection = ignoredSectionTitles.has(heading) || /^\d{4}-\d{2}-\d{2}/.test(heading)
      currentSection = isIgnoringSection
        ? null
        : { title: heading, items: [] }
      continue
    }

    if (isIgnoringSection || shouldIgnoreNoteLine(line)) {
      continue
    }

    const cleanedLine = cleanMarkdownText(line.replace(/^[-*+]\s+/, '').replace(/^\d+\.\s+/, '').replace(/^\[[ xX]\]\s+/, ''))
    if (!cleanedLine) {
      continue
    }

    if (!currentSection) {
      currentSection = { title: '', items: [] }
    }

    currentSection.items.push(cleanedLine)
  }

  const finalizedSection = finalizeNoteSection(currentSection)
  if (finalizedSection && !ignoredSectionTitles.has(finalizedSection.title)) {
    sections.push(finalizedSection)
  }

  return sections
    .filter((section) => !ignoredSectionTitles.has(section.title))
    .filter((section) => !section.items.every((item) => item.includes('没有额外')))
    .slice(0, 4)
}

function isIgnoredAsset(name: string) {
  const normalized = name.toLowerCase()

  return (
    normalized.endsWith('.sig')
    || normalized.endsWith('.json')
    || normalized.includes('sha256sum')
    || normalized === 'version'
    || normalized === 'version.txt'
    || normalized.endsWith('.yaml')
    || normalized.endsWith('.yml')
    || normalized.endsWith('.ts')
  )
}

function normalizeFileExtension(name: string) {
  const normalized = name.toLowerCase()

  if (normalized.endsWith('.appimage')) return '.AppImage'
  if (normalized.endsWith('.deb')) return '.deb'
  if (normalized.endsWith('.dmg')) return '.dmg'
  if (normalized.endsWith('.zip')) return '.zip'
  if (normalized.endsWith('.msi')) return '.msi'
  if (normalized.endsWith('.exe')) return '.exe'

  return ''
}

function detectAssetIdentity(name: string): Pick<NormalizedAsset, 'platform' | 'variantKey' | 'variantLabel' | 'fileExtension'> | null {
  if (isIgnoredAsset(name)) {
    return null
  }

  const normalized = name.toLowerCase()
  const fileExtension = normalizeFileExtension(name)

  if (/(aarch64|arm64)/i.test(name) && (normalized.endsWith('.dmg') || normalized.endsWith('.zip'))) {
    return { platform: 'macos', variantKey: 'appleSilicon', variantLabel: variantLabels.appleSilicon, fileExtension }
  }

  if (/(x86_64|x64|intel)/i.test(name) && (normalized.endsWith('.dmg') || normalized.endsWith('.zip'))) {
    return { platform: 'macos', variantKey: 'intel', variantLabel: variantLabels.intel, fileExtension }
  }

  if (/(x86_64|x64)/i.test(name) && (normalized.endsWith('.msi') || normalized.endsWith('.exe'))) {
    return { platform: 'windows', variantKey: 'x64', variantLabel: variantLabels.x64, fileExtension }
  }

  if (/arm64/i.test(name) && (normalized.endsWith('.msi') || normalized.endsWith('.exe'))) {
    return { platform: 'windows', variantKey: 'arm64', variantLabel: variantLabels.arm64, fileExtension }
  }

  if (normalized.endsWith('.appimage')) {
    return { platform: 'linux', variantKey: 'appImage', variantLabel: variantLabels.appImage, fileExtension }
  }

  if (normalized.endsWith('.deb')) {
    return { platform: 'linux', variantKey: 'deb', variantLabel: variantLabels.deb, fileExtension }
  }

  return null
}

function sortAssets(left: NormalizedAsset, right: NormalizedAsset) {
  if (left.channel !== right.channel) {
    return left.channel === 'stable' ? -1 : 1
  }

  if (left.platform !== right.platform) {
    return platformOrder.indexOf(left.platform) - platformOrder.indexOf(right.platform)
  }

  return variantOrder[left.variantKey] - variantOrder[right.variantKey]
}

export function normalizeGithubReleases(releases: GithubReleaseApiResponse[]): NormalizedRelease[] {
  return releases
    .filter((release) => !release.draft)
    .sort((left, right) => new Date(right.published_at).getTime() - new Date(left.published_at).getTime())
    .map((release) => ({
      id: release.id,
      tagName: release.tag_name,
      name: release.name ?? release.tag_name,
      publishedAt: release.published_at,
      htmlUrl: release.html_url,
      channel: release.prerelease ? 'preview' : 'stable',
      noteSections: extractReleaseNoteSections(release.body),
      assets: release.assets
        .map((asset) => {
          const identity = detectAssetIdentity(asset.name)

          if (!identity) {
            return null
          }

          return {
            id: asset.id,
            name: asset.name,
            downloadUrl: asset.browser_download_url,
            size: asset.size,
            platform: identity.platform,
            variantKey: identity.variantKey,
            variantLabel: identity.variantLabel,
            fileExtension: identity.fileExtension,
            channel: release.prerelease ? 'preview' : 'stable',
            releaseTag: release.tag_name,
          } satisfies NormalizedAsset
        })
        .filter((asset): asset is NormalizedAsset => Boolean(asset))
        .sort(sortAssets),
    }))
}

export function detectCurrentPlatform(userAgent: string): DownloadPlatform {
  const normalized = userAgent.toLowerCase()

  if (normalized.includes('win')) return 'windows'
  if (normalized.includes('linux')) return 'linux'

  return 'macos'
}

function collectPlatformAssets(releases: Array<NormalizedRelease | null>, platform: DownloadPlatform) {
  return releases
    .flatMap((release) => release?.assets ?? [])
    .filter((asset) => asset.platform === platform)
    .sort(sortAssets)
}

function getFirstAvailablePlatform(releases: Array<NormalizedRelease | null>, preferredPlatform: DownloadPlatform) {
  const preferredAssets = collectPlatformAssets(releases, preferredPlatform)
  if (preferredAssets.length > 0) {
    return preferredPlatform
  }

  return platformOrder.find((platform) => collectPlatformAssets(releases, platform).length > 0) ?? preferredPlatform
}

export function buildDownloadPageModel(releases: NormalizedRelease[], preferredPlatform: DownloadPlatform): DownloadPageModel {
  const latestStable = releases.find((release) => release.channel === 'stable') ?? null
  const latestPreview = releases.find((release) => release.channel === 'preview') ?? null
  const latestReleases = [latestStable, latestPreview]
  const heroPlatform = getFirstAvailablePlatform(latestReleases, preferredPlatform)
  const heroAssets = collectPlatformAssets(latestReleases, heroPlatform)
  const platformCards = platformOrder
    .map((platform) => ({
      platform,
      assets: collectPlatformAssets(latestReleases, platform),
    }))
    .filter((card) => card.assets.length > 0)

  return {
    latestStable,
    latestPreview,
    heroPlatform,
    heroAssets,
    platformCards,
    history: releases.slice(0, 8),
  }
}

export function buildGithubReleasesUrl(owner: string, repo: string) {
  return `https://github.com/${owner}/${repo}/releases`
}
