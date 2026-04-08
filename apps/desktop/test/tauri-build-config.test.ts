import { readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const tauriConfigPath = path.join(repoRoot, 'apps/desktop/src-tauri/tauri.conf.json')
const packageJsonPath = path.join(repoRoot, 'package.json')

describe('desktop release build configuration', () => {
  it('exposes a dedicated desktop release build script', () => {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
      scripts?: Record<string, string>
    }

    expect(packageJson.scripts?.['build:desktop']).toBe('pnpm tauri build --config apps/desktop/src-tauri/tauri.conf.json')
  })

  it('prepares the desktop sidecar before tauri build packages the app', () => {
    const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8')) as {
      build?: { beforeBuildCommand?: string }
    }
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
      scripts?: Record<string, string>
    }

    expect(packageJson.scripts?.['prepare:desktop-backend:sidecar']).toBeDefined()
    expect(tauriConfig.build?.beforeBuildCommand).toContain('pnpm prepare:desktop-backend:sidecar')
  })
})
