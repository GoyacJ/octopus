import { readdir, readFile } from 'node:fs/promises'
import path from 'node:path'

const rootDir = path.resolve(import.meta.dirname, '..')
const desktopSrcDir = path.join(rootDir, 'apps/desktop/src')
const businessViewDir = path.join(desktopSrcDir, 'views')
const businessLayoutDir = path.join(desktopSrcDir, 'components/layout')
const ignoredSegments = new Set(['node_modules', '.git', 'dist', 'target', '.turbo'])
const disallowedUiLibraries = [
  'reka-ui',
  'shadcn-vue',
  'element-plus',
  'ant-design-vue',
  'naive-ui',
  'vuetify',
  'quasar',
  'primevue',
  'radix-vue',
  '@radix-vue',
]
const scopedStyleHardLimit = 120
const forbiddenVisualPatternTokens = ['hero', 'panel', 'card', 'toolbar', 'dialog', 'ranking', 'timeline', 'metric', 'shell']
const nativeFormControlRegex = /<\s*(input|select|textarea|button)\b/g

const legacyAllowlist = {}

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true })
  const files = []

  for (const entry of entries) {
    if (ignoredSegments.has(entry.name)) {
      continue
    }

    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...await walk(fullPath))
      continue
    }

    files.push(fullPath)
  }

  return files
}

function toRepoPath(filePath) {
  return path.relative(rootDir, filePath).split(path.sep).join('/')
}

function extractImportSources(content) {
  const sources = []
  const regex = /from\s+['"]([^'"]+)['"]|import\s+['"]([^'"]+)['"]/g
  let match

  while ((match = regex.exec(content))) {
    sources.push(match[1] ?? match[2])
  }

  return sources
}

function extractStyleBlocks(content) {
  return [...content.matchAll(/<style\b([^>]*)>([\s\S]*?)<\/style>/g)].map((match) => ({
    attributes: match[1] ?? '',
    content: match[2] ?? '',
    scoped: /\bscoped\b/.test(match[1] ?? ''),
  }))
}

function countScopedStyleLines(content) {
  return extractStyleBlocks(content)
    .filter((block) => block.scoped)
    .reduce((total, block) => total + block.content.split('\n').length, 0)
}

function extractDefinedClassNames(content) {
  const names = new Set()
  for (const block of extractStyleBlocks(content)) {
    const regex = /\.([A-Za-z_-][A-Za-z0-9_-]*)/g
    let match
    while ((match = regex.exec(block.content))) {
      names.add(match[1])
    }
  }
  return [...names]
}

function findForbiddenPatternClasses(content) {
  const classNames = extractDefinedClassNames(content)
  return classNames.filter((name) =>
    forbiddenVisualPatternTokens.some((token) => new RegExp(`(^|-)${token}($|-)`).test(name)),
  )
}

function stripAllowedNativeControlExceptions(content) {
  return content.replace(
    /<input\b(?=[^>]*type\s*=\s*["']file["'])(?=[^>]*class\s*=\s*["'][^"']*\bhidden\b[^"']*["'])[^>]*>/g,
    '',
  )
}

function hasNativeFormControls(content) {
  const normalizedContent = stripAllowedNativeControlExceptions(content)
  nativeFormControlRegex.lastIndex = 0
  return nativeFormControlRegex.test(normalizedContent)
}

function isBusinessSurfaceFile(filePath) {
  return filePath.startsWith(businessViewDir) || filePath.startsWith(businessLayoutDir)
}

async function main() {
  const errors = []
  const warnings = []
  const repoFiles = await walk(rootDir)

  for (const filePath of repoFiles) {
    if (!/\.(ts|tsx|js|mjs|vue)$/.test(filePath)) {
      continue
    }

    const content = await readFile(filePath, 'utf8')
    const sources = extractImportSources(content)
    const rel = toRepoPath(filePath)

    if (!rel.startsWith('packages/ui/')) {
      const deepImport = sources.find((source) =>
        source.includes('packages/ui/src/components/')
        || source.includes('@octopus/ui/src/components/'),
      )

      if (deepImport) {
        errors.push(`${rel}: deep import is forbidden -> ${deepImport}`)
      }
    }

    if (filePath.startsWith(desktopSrcDir)) {
      const disallowedImport = sources.find((source) => disallowedUiLibraries.includes(source))
      if (disallowedImport) {
        errors.push(`${rel}: business code imports an unapproved UI library -> ${disallowedImport}`)
      }
    }

    if (!isBusinessSurfaceFile(filePath) || !filePath.endsWith('.vue')) {
      continue
    }

    const allowlistEntry = legacyAllowlist[rel]
    const scopedStyleLines = countScopedStyleLines(content)

    if (!allowlistEntry && scopedStyleLines > scopedStyleHardLimit) {
      errors.push(`${rel}: scoped style debt (${scopedStyleLines} lines) exceeds the non-allowlist limit of ${scopedStyleHardLimit}`)
    }

    if (allowlistEntry && scopedStyleLines > allowlistEntry.maxScopedStyleLines) {
      errors.push(`${rel}: scoped style debt grew from ${allowlistEntry.maxScopedStyleLines} to ${scopedStyleLines} lines`)
    }

    if (!allowlistEntry?.allowPatternClasses) {
      const offendingClasses = findForbiddenPatternClasses(content)
      if (offendingClasses.length) {
        errors.push(`${rel}: defines forbidden reusable visual class names -> ${offendingClasses.slice(0, 8).join(', ')}`)
      }
    }

    if (!allowlistEntry?.allowNativeFormControls && hasNativeFormControls(content)) {
      errors.push(`${rel}: uses native form controls; route the surface through shared Ui* controls or add an explicit migration allowlist entry`)
    }

    if (allowlistEntry && scopedStyleLines < scopedStyleHardLimit && !allowlistEntry.allowNativeFormControls && !allowlistEntry.allowPatternClasses) {
      warnings.push(`${rel}: debt looks retired; consider removing it from the frontend governance allowlist`)
    }
  }

  if (errors.length) {
    console.error('Frontend governance check failed:\n')
    for (const error of errors) {
      console.error(`- ${error}`)
    }
  }

  if (warnings.length) {
    console.warn('Frontend governance warnings:\n')
    for (const warning of warnings) {
      console.warn(`- ${warning}`)
    }
  }

  if (!errors.length && !warnings.length) {
    console.log('Frontend governance check passed with no findings.')
  }

  process.exitCode = errors.length ? 1 : 0
}

await main()
