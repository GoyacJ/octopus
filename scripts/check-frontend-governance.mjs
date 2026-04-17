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
const allowedTokenRoundedRegex = /^rounded-\[var\(--radius-(?:xs|s|m|l|xl|full)\)\]$/
const visualDriftChecks = [
  {
    label: 'uses page-private accent color classes',
    regex: /\bindigo-[A-Za-z0-9_[\]/.-]+\b/g,
  },
  {
    label: 'uses deprecated blue hue utilities outside token aliases',
    regex: /\b(?:bg|text|border|ring|from|to|via)-(?:blue|sky)-[A-Za-z0-9_[\]/.-]+\b/g,
  },
  {
    label: 'uses direct warm hue utilities outside semantic token aliases',
    regex: /\b(?:bg|text|border|ring|from|to|via)-(?:orange|amber|yellow)-[A-Za-z0-9_[\]/.-]+\b/g,
  },
  {
    label: 'uses backdrop blur',
    regex: /\bbackdrop-blur(?:-[A-Za-z0-9_[\].-]+)?\b/g,
  },
  {
    label: 'uses arbitrary rounded values outside canonical token radii',
    regex: /rounded-\[[^\]]+\]/g,
    allow: (match) => allowedTokenRoundedRegex.test(match),
  },
  {
    label: 'uses arbitrary shadow values',
    regex: /shadow-\[[^\]]+\]/g,
  },
  {
    label: 'uses gradient backgrounds outside the shared design system',
    regex: /\bbg-gradient-to-(?:t|tr|r|br|b|bl|l|tl)\b/g,
  },
]
const businessVisualDriftChecks = [
  {
    label: 'business surface uses UiSectionHeading after the workbench header migration',
    regex: /\bUiSectionHeading\b/g,
  },
  {
    label: 'business surface uses legacy selected ring classes',
    regex: /\bring-1\s+ring-primary\b/g,
  },
  {
    label: 'business surface uses dark border white overrides',
    regex: /dark:border-white\/[A-Za-z0-9.[\]-]+/g,
  },
  {
    label: 'business surface uses direct primary tint backgrounds',
    regex: /\bbg-primary\/5\b/g,
  },
  {
    label: 'business surface uses direct primary tint borders',
    regex: /\bborder-primary\/20\b/g,
  },
  {
    label: 'business surface uses direct status tint backgrounds',
    regex: /\bbg-status-(?:success|warning|error|info)\/5\b/g,
  },
  {
    label: 'business surface uses direct status tint borders',
    regex: /\bborder-status-(?:success|warning|error|info)\/20\b/g,
  },
  {
    label: 'business surface defines one-off management console containers',
    regex: /rounded-xl border border-border-subtle p-(?:4|5)\b/g,
  },
]

const legacyAllowlist = {
  'apps/desktop/src/components/layout/ConversationTabsBar.vue': {
    allowNativeFormControls: true,
    maxScopedStyleLines: 40,
  },
  'apps/desktop/src/components/layout/WorkbenchSidebar.vue': {
    allowNativeFormControls: true,
    maxScopedStyleLines: 40,
  },
  'apps/desktop/src/components/layout/WorkbenchTopbar.vue': {
    allowNativeFormControls: true,
    maxScopedStyleLines: 0,
  },
  'apps/desktop/src/views/workspace/ToolsView.vue': {
    allowNativeFormControls: true,
    maxScopedStyleLines: 0,
  },
  'apps/desktop/src/views/workspace/ProjectsView.vue': {
    allowNativeFormControls: true,
    maxScopedStyleLines: 0,
  },
  'apps/desktop/src/views/workspace/user/UserCenterMenuTree.vue': {
    allowNativeFormControls: true,
    maxScopedStyleLines: 0,
  },
}

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

function collectMatches(content, check, limit = 8) {
  const matches = new Set()
  const regex = new RegExp(check.regex.source, check.regex.flags)
  let match

  while ((match = regex.exec(content))) {
    const value = match[0]
    if (check.allow?.(value)) {
      continue
    }
    matches.add(value)
    if (matches.size >= limit) {
      break
    }
  }

  return [...matches]
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

      if (rel === 'apps/desktop/src/views/app/SettingsView.vue' && /\b(fontFamily|fontStyle)\b/.test(content)) {
        errors.push(`${rel}: settings UI must not expose font family/style controls or related preference bindings`)
      }
    }

    const shouldCheckVisualDrift = filePath.startsWith(desktopSrcDir) || filePath.startsWith(path.join(rootDir, 'packages/ui/src'))
    if (shouldCheckVisualDrift) {
      for (const check of visualDriftChecks) {
        const matches = collectMatches(content, check)
        if (matches.length) {
          errors.push(`${rel}: ${check.label} -> ${matches.join(', ')}`)
        }
      }
    }

    if (!isBusinessSurfaceFile(filePath) || !filePath.endsWith('.vue')) {
      continue
    }

    for (const check of businessVisualDriftChecks) {
      const matches = collectMatches(content, check)
      if (matches.length) {
        errors.push(`${rel}: ${check.label} -> ${matches.join(', ')}`)
      }
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
