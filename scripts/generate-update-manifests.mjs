import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot } from './governance-lib.mjs'

function readArgument(flag) {
  const index = process.argv.indexOf(flag)
  return index >= 0 ? process.argv[index + 1] : undefined
}

function normalizeTagVersion(tagName) {
  return String(tagName ?? '').replace(/^v/, '')
}

function normalizeAssetBaseName(assetUrl) {
  try {
    return path.posix.basename(new URL(assetUrl).pathname)
  } catch {
    return String(assetUrl).split('/').pop() ?? ''
  }
}

async function fetchJson(url, {
  token,
  accept = 'application/vnd.github+json',
} = {}) {
  const headers = {
    accept,
    'user-agent': 'octopus-update-manifest-generator',
  }

  if (token) {
    headers.authorization = `Bearer ${token}`
  }

  const response = await fetch(url, { headers })
  if (!response.ok) {
    throw new Error(`request failed for ${url}: ${response.status} ${response.statusText}`)
  }
  return await response.json()
}

function resolveReleaseForChannel(releases, channel) {
  return releases.find((release) => {
    if (channel === 'preview') {
      return release?.prerelease === true
    }
    return release?.prerelease !== true
  })
}

function selectManifestAssets(release) {
  return (release?.assets ?? []).filter((asset) => String(asset?.name ?? '').toLowerCase().endsWith('latest.json'))
}

function buildReleaseAssetMap(release) {
  return new Map(
    (release?.assets ?? [])
      .filter((asset) => typeof asset?.name === 'string' && typeof asset?.browser_download_url === 'string')
      .map((asset) => [asset.name, asset.browser_download_url]),
  )
}

async function buildChannelManifest(release, channel, token) {
  if (!release) {
    throw new Error(`missing ${channel} release`)
  }

  const manifestAssets = selectManifestAssets(release)
  if (manifestAssets.length === 0) {
    throw new Error(`release ${release.tag_name} is missing updater manifest assets`)
  }

  const downloadAssetMap = buildReleaseAssetMap(release)
  const mergedPlatforms = {}

  for (const asset of manifestAssets) {
    const manifest = await fetchJson(asset.url ?? asset.browser_download_url, {
      token,
      accept: 'application/octet-stream',
    })
    const platforms = manifest?.platforms

    if (!platforms || typeof platforms !== 'object') {
      throw new Error(`asset ${asset.name} in ${release.tag_name} is missing platforms`)
    }

    for (const [platformKey, platformManifest] of Object.entries(platforms)) {
      if (mergedPlatforms[platformKey]) {
        throw new Error(`duplicate updater platform ${platformKey} in release ${release.tag_name}`)
      }

      const assetBaseName = normalizeAssetBaseName(platformManifest?.url)
      const browserDownloadUrl = downloadAssetMap.get(assetBaseName)
      if (!browserDownloadUrl) {
        throw new Error(`release ${release.tag_name} is missing downloadable asset ${assetBaseName}`)
      }

      mergedPlatforms[platformKey] = {
        ...platformManifest,
        url: browserDownloadUrl,
      }
    }
  }

  return {
    version: normalizeTagVersion(release.tag_name),
    notes: release.body ?? '',
    pub_date: release.published_at ?? null,
    channel,
    notesUrl: release.html_url ?? null,
    platforms: mergedPlatforms,
  }
}

async function main() {
  const repo = readArgument('--repo') ?? 'GoyacJ/octopus'
  const apiBaseUrl = (readArgument('--api-base-url') ?? 'https://api.github.com').replace(/\/$/, '')
  const outputDir = path.resolve(repoRoot, readArgument('--output-dir') ?? 'updates')
  const token = process.env.GITHUB_TOKEN?.trim() || undefined

  const releases = await fetchJson(`${apiBaseUrl}/repos/${repo}/releases?per_page=20`, { token })
  if (!Array.isArray(releases)) {
    throw new Error('GitHub releases response must be an array')
  }

  const formalManifest = await buildChannelManifest(resolveReleaseForChannel(releases, 'formal'), 'formal', token)
  const previewManifest = await buildChannelManifest(resolveReleaseForChannel(releases, 'preview'), 'preview', token)

  await mkdir(path.join(outputDir, 'formal'), { recursive: true })
  await mkdir(path.join(outputDir, 'preview'), { recursive: true })
  await writeFile(
    path.join(outputDir, 'formal', 'latest.json'),
    `${JSON.stringify(formalManifest, null, 2)}\n`,
  )
  await writeFile(
    path.join(outputDir, 'preview', 'latest.json'),
    `${JSON.stringify(previewManifest, null, 2)}\n`,
  )
}

await main()
