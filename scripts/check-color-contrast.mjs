import { readFile } from 'node:fs/promises'
import path from 'node:path'

const repoRoot = path.resolve(import.meta.dirname, '..')
const tokensPath = path.join(repoRoot, 'packages/ui/src/tokens.css')
const contrastThreshold = 4.5
const themeNames = ['light', 'dark']
const pairDefinitions = [
  {
    label: 'primary text on canvas',
    foreground: 'color-text-primary',
    background: 'color-canvas',
  },
  {
    label: 'secondary text on surface',
    foreground: 'color-text-secondary',
    background: 'color-surface',
  },
  {
    label: 'accent text on surface',
    foreground: 'color-accent',
    background: 'color-surface',
  },
  {
    label: 'warning text on warning soft surface',
    foreground: 'color-status-warning',
    background: 'color-status-warning-soft',
    backgroundBase: 'color-surface',
  },
  {
    label: 'error text on error soft surface',
    foreground: 'color-status-error',
    background: 'color-status-error-soft',
    backgroundBase: 'color-surface',
  },
]

function parseThemeBlock(content, themeName) {
  const match = content.match(new RegExp(`\\[data-theme="${themeName}"\\]\\s*\\{([\\s\\S]*?)\\n\\}`))
  if (!match) {
    throw new Error(`Missing [data-theme="${themeName}"] block in tokens.css`)
  }

  return Object.fromEntries(
    [...match[1].matchAll(/--([\w-]+):\s*([^;]+);/g)].map(([, tokenName, value]) => [tokenName, value.trim()]),
  )
}

function parseColor(value) {
  const normalizedValue = value.trim()

  if (normalizedValue.startsWith('#')) {
    const hex = normalizedValue.slice(1)
    const expandedHex = hex.length === 3 ? hex.split('').map((char) => char + char).join('') : hex
    const numericValue = Number.parseInt(expandedHex, 16)

    return {
      r: (numericValue >> 16) & 255,
      g: (numericValue >> 8) & 255,
      b: numericValue & 255,
      a: 1,
    }
  }

  const rgbaMatch = normalizedValue.match(/rgba?\(([^)]+)\)/i)
  if (!rgbaMatch) {
    throw new Error(`Unsupported color format: ${normalizedValue}`)
  }

  const [r, g, b, a = '1'] = rgbaMatch[1].split(',').map((part) => part.trim())

  return {
    r: Number(r),
    g: Number(g),
    b: Number(b),
    a: Number(a),
  }
}

function blendColors(foreground, background) {
  const alpha = foreground.a ?? 1

  return {
    r: foreground.r * alpha + background.r * (1 - alpha),
    g: foreground.g * alpha + background.g * (1 - alpha),
    b: foreground.b * alpha + background.b * (1 - alpha),
    a: 1,
  }
}

function resolveBackgroundColor(themeTokens, pairDefinition) {
  const backgroundColor = parseColor(themeTokens[pairDefinition.background])

  if ((backgroundColor.a ?? 1) >= 1 || !pairDefinition.backgroundBase) {
    return backgroundColor
  }

  const baseColor = parseColor(themeTokens[pairDefinition.backgroundBase])
  return blendColors(backgroundColor, baseColor)
}

function relativeLuminance(color) {
  const normalizeChannel = (channel) => {
    const unit = channel / 255
    return unit <= 0.03928 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4
  }

  const red = normalizeChannel(color.r)
  const green = normalizeChannel(color.g)
  const blue = normalizeChannel(color.b)

  return 0.2126 * red + 0.7152 * green + 0.0722 * blue
}

function computeContrastRatio(foreground, background) {
  const foregroundColor = (foreground.a ?? 1) < 1 ? blendColors(foreground, background) : foreground
  const luminances = [relativeLuminance(foregroundColor), relativeLuminance(background)].sort((a, b) => b - a)

  return (luminances[0] + 0.05) / (luminances[1] + 0.05)
}

async function main() {
  const tokensContent = await readFile(tokensPath, 'utf8')
  const failures = []
  const reports = []

  for (const themeName of themeNames) {
    const themeTokens = parseThemeBlock(tokensContent, themeName)

    for (const pairDefinition of pairDefinitions) {
      const backgroundColor = resolveBackgroundColor(themeTokens, pairDefinition)
      const foregroundColor = parseColor(themeTokens[pairDefinition.foreground])
      const contrastRatio = computeContrastRatio(foregroundColor, backgroundColor)

      reports.push({
        themeName,
        label: pairDefinition.label,
        foreground: pairDefinition.foreground,
        background: pairDefinition.background,
        contrastRatio,
      })

      if (contrastRatio < contrastThreshold) {
        failures.push({
          themeName,
          label: pairDefinition.label,
          foreground: pairDefinition.foreground,
          background: pairDefinition.background,
          contrastRatio,
        })
      }
    }
  }

  if (failures.length > 0) {
    console.error('Color contrast check failed')
    for (const failure of failures) {
      console.error(
        `- [${failure.themeName}] --${failure.foreground} on --${failure.background} (${failure.label}): ${failure.contrastRatio.toFixed(2)} < ${contrastThreshold.toFixed(1)}`,
      )
    }
    process.exitCode = 1
    return
  }

  console.log('Color contrast check passed')
  for (const report of reports) {
    console.log(
      `- [${report.themeName}] --${report.foreground} on --${report.background} (${report.label}): ${report.contrastRatio.toFixed(2)}`,
    )
  }
}

await main()
