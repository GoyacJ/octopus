import { readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const tauriConfigPath = path.join(repoRoot, 'apps/desktop/src-tauri/tauri.conf.json')
const updaterConfigPath = path.join(repoRoot, 'apps/desktop/src-tauri/updater.config.json')
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

  it('enables updater artifacts for desktop release builds', () => {
    const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8')) as {
      bundle?: { createUpdaterArtifacts?: boolean | string }
    }

    expect(tauriConfig.bundle?.createUpdaterArtifacts).toBe(true)
  })

  it('declares a concrete updater plugin config so desktop dev startup does not crash', () => {
    const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8')) as {
      plugins?: {
        updater?: {
          endpoints?: string[]
          pubkey?: string
        }
      }
    }

    expect(tauriConfig.plugins?.updater).toEqual({
      endpoints: [],
      pubkey: '',
    })
  })

  it('tracks product updater defaults in repo so end users do not need local env configuration', () => {
    const updaterConfig = JSON.parse(readFileSync(updaterConfigPath, 'utf8')) as {
      formalEndpoint?: string
      previewEndpoint?: string
      pubkey?: string
      releaseRepo?: string
    }

    expect(updaterConfig.formalEndpoint).toBe('https://goyacj.github.io/octopus/updates/formal/latest.json')
    expect(updaterConfig.previewEndpoint).toBe('https://goyacj.github.io/octopus/updates/preview/latest.json')
    expect(updaterConfig.releaseRepo).toBe('GoyacJ/octopus')
    expect(typeof updaterConfig.pubkey).toBe('string')
    expect((updaterConfig.pubkey ?? '').trim().length).toBeGreaterThan(0)
  })
})
