import { execFileSync } from 'node:child_process'
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot, versionFilePath } from './governance-lib.mjs'

export const releaseNotesFragmentsDir = path.join(repoRoot, 'docs', 'release-notes', 'fragments')

const fragmentCategoryOrder = ['summary', 'feature', 'fix', 'breaking', 'migration', 'docs', 'internal', 'governance']
const conventionalCommitCategoryPatterns = [
  ['breaking', /(^.+!:\s)|(^breaking(\(.+\))?:\s)|\bbreaking change\b/i],
  ['migration', /(^migration(\(.+\))?:\s)|\bmigration\b/i],
  ['fix', /(^fix(\(.+\))?!?:\s)|(^bugfix(\(.+\))?!?:\s)|\bbug fix\b/i],
  ['feature', /(^feat(\(.+\))?!?:\s)|(^feature(\(.+\))?!?:\s)|(^perf(\(.+\))?!?:\s)/i],
  ['docs', /(^docs(\(.+\))?!?:\s)|(^doc(\(.+\))?!?:\s)/i],
  ['governance', /\b(governance|release|schema|version)\b/i],
  ['internal', /(^chore(\(.+\))?!?:\s)|(^refactor(\(.+\))?!?:\s)|(^test(\(.+\))?!?:\s)|(^ci(\(.+\))?!?:\s)|(^build(\(.+\))?!?:\s)/i],
]

function runGit(args, { allowFailure = false } = {}) {
  try {
    return execFileSync('git', args, {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }).trim()
  } catch (error) {
    if (allowFailure) {
      return ''
    }
    throw error
  }
}

function toBoolean(value, defaultValue = false) {
  if (value == null) {
    return defaultValue
  }
  return ['1', 'true', 'yes', 'on'].includes(String(value).toLowerCase())
}

function slugToTitle(slug) {
  return slug
    .replace(/^\d{4}-\d{2}-\d{2}-/, '')
    .replace(/-/g, ' ')
    .replace(/\b\w/g, (character) => character.toUpperCase())
}

function normalizeCommitSubject(subject) {
  return subject
    .replace(/^(feat|feature|fix|bugfix|docs|doc|chore|refactor|test|ci|build|perf|release|migration)(\([^)]+\))?!?:\s*/i, '')
    .trim()
}

function inferCategoryFromText(text, body = '') {
  const combined = `${text}\n${body}`.trim()
  for (const [category, pattern] of conventionalCommitCategoryPatterns) {
    if (pattern.test(combined)) {
      return category
    }
  }
  return 'feature'
}

function fragmentIsUserFacing(category) {
  return ['summary', 'feature', 'fix', 'breaking', 'migration', 'docs'].includes(category)
}

function extractLead(content) {
  const lines = content
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
  const firstParagraph = lines.find((line) => !line.startsWith('- ') && !line.startsWith('* '))
  return firstParagraph ?? lines[0] ?? ''
}

function dedupeByKey(items, keySelector) {
  const seen = new Set()
  return items.filter((item) => {
    const key = keySelector(item)
    if (!key || seen.has(key)) {
      return false
    }
    seen.add(key)
    return true
  })
}

export function readArgument(flag, argv = process.argv) {
  const index = argv.indexOf(flag)
  return index >= 0 ? argv[index + 1] : undefined
}

export async function resolveReleaseNotesOptions(argv = process.argv) {
  const channel = readArgument('--channel', argv) ?? 'formal'
  if (!['formal', 'preview'].includes(channel)) {
    throw new Error(`--channel must be formal or preview, received ${channel}`)
  }

  const language = readArgument('--language', argv) ?? 'zh-CN'
  if (language !== 'zh-CN') {
    throw new Error(`Unsupported release notes language: ${language}`)
  }

  const version = (await readFile(versionFilePath, 'utf8')).trim()
  const releaseTag = readArgument('--tag', argv) ?? `v${version}`
  const requestedOutput = readArgument('--output', argv)
  const outputPath = requestedOutput
    ? path.resolve(repoRoot, requestedOutput)
    : path.join(repoRoot, 'tmp', 'release-notes', channel === 'preview' ? `${releaseTag}.md` : `v${version}.md`)
  const outputDirectory = path.dirname(outputPath)

  return {
    channel,
    language,
    version,
    releaseTag,
    runNumber: readArgument('--run-number', argv) ?? null,
    commitSha: readArgument('--sha', argv) ?? (runGit(['rev-parse', 'HEAD'], { allowFailure: true }) || null),
    sinceRef: readArgument('--since-ref', argv) ?? null,
    fragmentsDir: path.resolve(repoRoot, readArgument('--fragments-dir', argv) ?? releaseNotesFragmentsDir),
    outputPath,
    releaseNotesJsonPath: path.resolve(
      repoRoot,
      readArgument('--release-notes-json-output', argv) ?? path.join(outputDirectory, 'release-notes.json'),
    ),
    changeLogJsonPath: path.resolve(
      repoRoot,
      readArgument('--change-log-output', argv) ?? path.join(outputDirectory, 'change-log.json'),
    ),
    requireManualSummary: toBoolean(readArgument('--require-manual-summary', argv), channel === 'formal'),
  }
}

export async function collectFragments(fragmentsDir) {
  const fragmentFiles = (await readdir(fragmentsDir).catch(() => []))
    .filter((entry) => entry.endsWith('.md'))
    .filter((entry) => entry.toLowerCase() !== 'readme.md')
    .sort()

  const fragments = []
  for (const fileName of fragmentFiles) {
    const content = (await readFile(path.join(fragmentsDir, fileName), 'utf8')).trim()
    if (!content) {
      continue
    }
    const match = fileName.match(/^([a-z]+)-(.+)\.md$/)
    const category = match?.[1] ?? 'internal'
    const slug = match?.[2] ?? fileName.replace(/\.md$/, '')
    fragments.push({
      fileName,
      category,
      slug,
      title: slugToTitle(slug),
      lead: extractLead(content),
      content,
      userFacing: fragmentIsUserFacing(category),
    })
  }

  return fragments.sort((left, right) => {
    const categoryOffset = fragmentCategoryOrder.indexOf(left.category) - fragmentCategoryOrder.indexOf(right.category)
    if (categoryOffset !== 0) {
      return categoryOffset
    }
    return left.fileName.localeCompare(right.fileName)
  })
}

function resolveFormalSinceRef(releaseTag) {
  const tags = runGit(['tag', '--list', 'v[0-9]*.[0-9]*.[0-9]*', '--sort=-version:refname'], { allowFailure: true })
    .split('\n')
    .map((entry) => entry.trim())
    .filter(Boolean)

  if (!tags.length) {
    return null
  }

  const currentIndex = tags.indexOf(releaseTag)
  if (currentIndex >= 0) {
    return tags[currentIndex + 1] ?? null
  }

  return tags.find((entry) => entry !== releaseTag) ?? null
}

function resolvePreviewSinceRef(releaseTag) {
  const tags = runGit(['tag', '--list', 'v*-preview.*', '--sort=-creatordate'], { allowFailure: true })
    .split('\n')
    .map((entry) => entry.trim())
    .filter(Boolean)

  return tags.find((entry) => entry !== releaseTag) ?? null
}

export function resolveSinceRef({ channel, releaseTag, requestedSinceRef }) {
  if (requestedSinceRef) {
    return requestedSinceRef
  }
  return channel === 'preview'
    ? resolvePreviewSinceRef(releaseTag)
    : resolveFormalSinceRef(releaseTag)
}

export function resolveChangeRangeLabel(sinceRef, releaseTag) {
  return `${sinceRef ?? '仓库初始化'} -> ${releaseTag}`
}

export function collectChangeLog({ channel, releaseTag, commitSha, sinceRef }) {
  const targetRef = commitSha || releaseTag || 'HEAD'
  const rangeSpec = sinceRef ? `${sinceRef}..${targetRef}` : targetRef
  const output = runGit(['log', '--format=%H%x1f%s%x1e', rangeSpec], { allowFailure: true })
  const rawCommits = output
    .split('\x1e')
    .map((entry) => entry.trim())
    .filter(Boolean)

  const commits = rawCommits.map((entry) => {
    const [sha, subject] = entry.split('\x1f')
    const normalizedSubject = normalizeCommitSubject(subject)
    const category = inferCategoryFromText(subject)
    const prNumber = subject.match(/\(#(\d+)\)$/)?.[1] ?? null
    return {
      sha,
      shortSha: sha.slice(0, 7),
      subject,
      normalizedSubject,
      category,
      prNumber,
      userFacing: fragmentIsUserFacing(category),
    }
  })

  const pullRequests = dedupeByKey(
    commits
      .filter((commit) => commit.prNumber)
      .map((commit) => ({
        number: Number(commit.prNumber),
        title: commit.normalizedSubject.replace(/\s*\(#\d+\)\s*$/, ''),
      })),
    (pullRequest) => pullRequest.number,
  )

  return {
    channel,
    sinceRef,
    targetRef,
    releaseTag,
    rangeLabel: resolveChangeRangeLabel(sinceRef, releaseTag),
    commits,
    pullRequests,
  }
}

function createAutoSummary({ channel, fragments, changeLog }) {
  const featureCount = fragments.filter((fragment) => ['feature', 'docs'].includes(fragment.category)).length
    + changeLog.commits.filter((commit) => ['feature', 'docs'].includes(commit.category) && commit.userFacing).length
  const fixCount = fragments.filter((fragment) => fragment.category === 'fix').length
    + changeLog.commits.filter((commit) => commit.category === 'fix').length

  if (channel === 'preview') {
    return `本次预览构建聚合了 ${featureCount} 项能力变更和 ${fixCount} 项修复，用于主干持续回归与安装包验证。`
  }

  return `本次正式发布包含 ${featureCount} 项用户可感知更新和 ${fixCount} 项修复。`
}

function renderFragmentBlocks(items) {
  if (!items.length) {
    return []
  }
  return items.flatMap((item) => [
    `### ${item.title}`,
    '',
    item.content,
    '',
  ])
}

function renderCommitBullets(items) {
  if (!items.length) {
    return []
  }
  return [
    '### 自动汇总变更',
    '',
    ...items.map((item) => `- ${item.normalizedSubject}${item.prNumber ? ` (#${item.prNumber})` : ''}`),
    '',
  ]
}

export function buildReleaseNotesData(options, fragments, changeLog) {
  const releaseDate = new Date().toISOString().slice(0, 10)
  const summaryFragments = fragments.filter((fragment) => fragment.category === 'summary')

  if (options.channel === 'formal' && options.requireManualSummary && summaryFragments.length === 0) {
    throw new Error('Formal release notes require at least one summary-* fragment.')
  }

  const featureFragments = fragments.filter((fragment) => ['feature', 'docs'].includes(fragment.category))
  const fixFragments = fragments.filter((fragment) => fragment.category === 'fix')
  const upgradeFragments = fragments.filter((fragment) => ['breaking', 'migration'].includes(fragment.category))
  const internalFragments = fragments.filter((fragment) => ['internal', 'governance'].includes(fragment.category))

  const userFacingCommits = changeLog.commits.filter((commit) => commit.userFacing)
  const featureCommits = userFacingCommits.filter((commit) => ['feature', 'docs'].includes(commit.category))
  const fixCommits = userFacingCommits.filter((commit) => commit.category === 'fix')
  const internalCommits = changeLog.commits.filter((commit) => !commit.userFacing)

  const generatedSummary = createAutoSummary({ channel: options.channel, fragments, changeLog })
  const overviewItems = summaryFragments.length
    ? summaryFragments.map((fragment) => ({
        source: 'fragment',
        category: fragment.category,
        title: fragment.title,
        content: fragment.content,
      }))
    : [{
        source: 'generated',
        category: 'summary',
        title: '自动概览',
        content: generatedSummary,
      }]

  const verificationChecks = options.channel === 'preview'
    ? [
        '主干预览发布治理检查已执行',
        'VERSION / Cargo / Tauri / OpenAPI 版本一致性已校验',
        'Schema 单源生成一致性已校验',
        '全仓质量门禁已通过后再生成预览说明',
      ]
    : [
        'Tag 驱动正式发布治理检查已执行',
        'VERSION / Cargo / Tauri / OpenAPI 版本一致性已校验',
        'Schema 单源生成一致性已校验',
        '全仓质量门禁已通过后再生成正式说明',
      ]

  const appendix = {
    metadata: {
      channel: options.channel,
      language: options.language,
      version: options.version,
      tag: options.releaseTag,
      commitSha: options.commitSha,
      runNumber: options.runNumber,
      changeRange: changeLog.rangeLabel,
      checksumsPath: 'release-artifacts/SHA256SUMS.txt',
    },
    artifacts: [
      'macOS: `.dmg` 桌面安装包',
      'Windows: `NSIS .exe` 安装包',
    ],
    verificationChecks,
  }

  const sections = options.channel === 'preview'
    ? {
        previewSummary: {
          title: '预览摘要',
          required: true,
          items: [
            {
              source: 'system',
              category: 'preview',
              title: '预览声明',
              content: '这是来自 `main` 分支的预览构建，用于回归验证与试用，不承诺稳定性或升级兼容性。',
            },
            {
              source: overviewItems[0].source,
              category: 'summary',
              title: overviewItems[0].title,
              content: overviewItems[0].content,
            },
          ],
        },
        changes: {
          title: '本次变更',
          required: false,
          fragmentGroups: [
            ...featureFragments,
            ...fixFragments,
            ...upgradeFragments,
            ...internalFragments,
          ],
          commitGroups: [
            ...featureCommits,
            ...fixCommits,
            ...internalCommits,
          ],
        },
        verificationStatus: {
          title: '验证状态',
          required: true,
          items: verificationChecks,
          artifacts: appendix.artifacts,
        },
        buildMetadata: {
          title: '构建元数据',
          required: true,
          metadata: appendix.metadata,
        },
      }
    : {
        overview: {
          title: '版本概览',
          required: true,
          items: overviewItems,
        },
        userChanges: {
          title: '用户可感知变化',
          required: false,
          fragmentGroups: featureFragments,
          commitGroups: featureCommits,
        },
        upgradeNotes: {
          title: '升级提示',
          required: false,
          fragmentGroups: upgradeFragments,
          commitGroups: [],
        },
        fixes: {
          title: '修复摘要',
          required: false,
          fragmentGroups: fixFragments,
          commitGroups: fixCommits,
        },
        appendix: {
          title: '技术附录',
          required: true,
          ...appendix,
        },
      }

  return {
    channel: options.channel,
    language: options.language,
    version: options.version,
    tag: options.releaseTag,
    title: options.channel === 'preview'
      ? `Octopus Preview ${options.releaseTag}`
      : `Octopus ${options.releaseTag}`,
    releaseDate,
    sinceRef: changeLog.sinceRef,
    targetRef: changeLog.targetRef,
    changeRange: changeLog.rangeLabel,
    requireManualSummary: options.requireManualSummary,
    sections,
    appendix,
    fragments,
    changeLog,
  }
}

export function renderReleaseNotesMarkdown(data) {
  const lines = [
    `# ${data.title}`,
    '',
    `发布日期：${data.releaseDate}`,
    '',
  ]

  if (data.channel === 'preview') {
    lines.push(`## ${data.sections.previewSummary.title}`, '')
    for (const item of data.sections.previewSummary.items) {
      lines.push(item.content, '')
    }

    lines.push(`## ${data.sections.changes.title}`, '')
    const previewFragmentBlocks = renderFragmentBlocks(data.sections.changes.fragmentGroups)
    const previewCommitBlocks = renderCommitBullets(data.sections.changes.commitGroups)
    if (!previewFragmentBlocks.length && !previewCommitBlocks.length) {
      lines.push('本次预览构建没有额外的 fragments，正文已根据提交历史自动生成。', '')
    } else {
      lines.push(...previewFragmentBlocks, ...previewCommitBlocks)
    }

    lines.push(`## ${data.sections.verificationStatus.title}`, '')
    lines.push(...data.sections.verificationStatus.items.map((item) => `- ${item}`), '')
    lines.push('### 构建产物', '', ...data.sections.verificationStatus.artifacts.map((item) => `- ${item}`), '')

    lines.push(`## ${data.sections.buildMetadata.title}`, '')
    lines.push(
      `- 发布渠道：${data.appendix.metadata.channel}`,
      `- VERSION：${data.appendix.metadata.version}`,
      `- Release Tag：${data.appendix.metadata.tag}`,
      `- Commit SHA：${data.appendix.metadata.commitSha ?? 'unknown'}`,
      `- Run Number：${data.appendix.metadata.runNumber ?? 'unknown'}`,
      `- 变更范围：${data.appendix.metadata.changeRange}`,
      `- Checksums：${data.appendix.metadata.checksumsPath}`,
      '',
    )

    return `${lines.join('\n').trim()}\n`
  }

  lines.push(`## ${data.sections.overview.title}`, '')
  for (const item of data.sections.overview.items) {
    lines.push(item.content, '')
  }

  lines.push(`## ${data.sections.userChanges.title}`, '')
  const userChangeBlocks = renderFragmentBlocks(data.sections.userChanges.fragmentGroups)
  const userChangeCommits = renderCommitBullets(data.sections.userChanges.commitGroups)
  if (!userChangeBlocks.length && !userChangeCommits.length) {
    lines.push('本版本没有额外的用户可感知功能变更。', '')
  } else {
    lines.push(...userChangeBlocks, ...userChangeCommits)
  }

  lines.push(`## ${data.sections.upgradeNotes.title}`, '')
  const upgradeBlocks = renderFragmentBlocks(data.sections.upgradeNotes.fragmentGroups)
  if (!upgradeBlocks.length) {
    lines.push('本版本没有额外的升级迁移要求。', '')
  } else {
    lines.push(...upgradeBlocks)
  }

  lines.push(`## ${data.sections.fixes.title}`, '')
  const fixBlocks = renderFragmentBlocks(data.sections.fixes.fragmentGroups)
  const fixCommits = renderCommitBullets(data.sections.fixes.commitGroups)
  if (!fixBlocks.length && !fixCommits.length) {
    lines.push('本版本没有额外记录的用户可感知修复。', '')
  } else {
    lines.push(...fixBlocks, ...fixCommits)
  }

  lines.push(`## ${data.sections.appendix.title}`, '')
  lines.push(
    '### 发布元数据',
    '',
    `- 发布渠道：${data.appendix.metadata.channel}`,
    `- VERSION：${data.appendix.metadata.version}`,
    `- Release Tag：${data.appendix.metadata.tag}`,
    `- Commit SHA：${data.appendix.metadata.commitSha ?? 'unknown'}`,
    `- 变更范围：${data.appendix.metadata.changeRange}`,
    `- Checksums：${data.appendix.metadata.checksumsPath}`,
    '',
    '### 验证状态',
    '',
    ...data.appendix.verificationChecks.map((item) => `- ${item}`),
    '',
    '### 构建产物',
    '',
    ...data.appendix.artifacts.map((item) => `- ${item}`),
    '',
  )

  return `${lines.join('\n').trim()}\n`
}

export async function writeReleaseNotesArtifacts(options, releaseNotesData, changeLog, markdown) {
  await mkdir(path.dirname(options.outputPath), { recursive: true })
  await writeFile(options.outputPath, markdown)
  await writeFile(options.releaseNotesJsonPath, `${JSON.stringify(releaseNotesData, null, 2)}\n`)
  await writeFile(options.changeLogJsonPath, `${JSON.stringify(changeLog, null, 2)}\n`)
}
