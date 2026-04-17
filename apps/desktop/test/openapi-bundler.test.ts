import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const nodeExecutable = process.execPath
const bundleScriptPath = path.join(repoRoot, 'scripts', 'bundle-openapi.mjs')
const tempDirectories: string[] = []

function createTempDir(prefix: string) {
  const directory = mkdtempSync(path.join(os.tmpdir(), prefix))
  tempDirectories.push(directory)
  return directory
}

function writeFile(filePath: string, contents: string) {
  mkdirSync(path.dirname(filePath), { recursive: true })
  writeFileSync(filePath, contents)
}

afterEach(() => {
  for (const directory of tempDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true })
  }
})

function bundleFixture(rootPath: string, outputPath: string) {
  execFileSync(nodeExecutable, [bundleScriptPath, '--root', rootPath, '--output', outputPath], {
    cwd: repoRoot,
    stdio: 'pipe',
  })

  return readFileSync(outputPath, 'utf8')
}

describe('OpenAPI bundler', () => {
  it('keeps the repo bundled contract aligned with deliverable-first transport schemas', () => {
    const legacySummaryAliasSchema = ['Artifact', 'Record:'].join('')
    const legacyWorkspaceArtifactsPath = ['/api/v1', 'artifacts:'].join('/')
    const bundledContract = readFileSync(
      path.join(repoRoot, 'contracts', 'openapi', 'octopus.openapi.yaml'),
      'utf8',
    )

    expect(bundledContract).toContain('DeliverableSummary:')
    expect(bundledContract).toContain('DeliverableDetail:')
    expect(bundledContract).toContain('DeliverableVersionSummary:')
    expect(bundledContract).toContain('DeliverableVersionContent:')
    expect(bundledContract).toContain('ArtifactVersionReference:')
    expect(bundledContract).toContain('CreateDeliverableVersionInput:')
    expect(bundledContract).toContain('PromoteDeliverableInput:')
    expect(bundledContract).toContain('ForkDeliverableInput:')
    expect(bundledContract).toContain('/api/v1/workspace/deliverables:')
    expect(bundledContract).toContain('/api/v1/projects/{projectId}/deliverables:')
    expect(bundledContract).toContain('/api/v1/deliverables/{deliverableId}:')
    expect(bundledContract).toContain('/api/v1/deliverables/{deliverableId}/versions:')
    expect(bundledContract).toContain('/api/v1/deliverables/{deliverableId}/versions/{version}:')
    expect(bundledContract).toContain('/api/v1/deliverables/{deliverableId}/promote:')
    expect(bundledContract).toContain('/api/v1/deliverables/{deliverableId}/fork:')
    expect(bundledContract).not.toContain(legacySummaryAliasSchema)
    expect(bundledContract).not.toContain(legacyWorkspaceArtifactsPath)
  })

  it('keeps the repo bundled contract aligned with project task transport schemas', () => {
    const bundledContract = readFileSync(
      path.join(repoRoot, 'contracts', 'openapi', 'octopus.openapi.yaml'),
      'utf8',
    )

    expect(bundledContract).toContain('TaskSummary:')
    expect(bundledContract).toContain('TaskDetail:')
    expect(bundledContract).toContain('TaskRunSummary:')
    expect(bundledContract).toContain('TaskContextBundle:')
    expect(bundledContract).toContain('TaskContextRef:')
    expect(bundledContract).toContain('TaskFailureCategory:')
    expect(bundledContract).toContain('TaskStateTransitionSummary:')
    expect(bundledContract).toContain('TaskAnalyticsSummary:')
    expect(bundledContract).toContain('pendingApprovalId:')
    expect(bundledContract).toContain('approvalId:')
    expect(bundledContract).toContain('/api/v1/projects/{projectId}/tasks:')
    expect(bundledContract).toContain('/api/v1/projects/{projectId}/tasks/{taskId}:')
    expect(bundledContract).toContain('/api/v1/projects/{projectId}/tasks/{taskId}/launch:')
    expect(bundledContract).toContain('/api/v1/projects/{projectId}/tasks/{taskId}/rerun:')
    expect(bundledContract).toContain('/api/v1/projects/{projectId}/tasks/{taskId}/runs:')
    expect(bundledContract).toContain('/api/v1/projects/{projectId}/tasks/{taskId}/interventions:')
    expect(bundledContract).toContain('- change_actor')
  })

  it('keeps project dashboard task summaries and task permission modules in the bundled contract', () => {
    const bundledContract = readFileSync(
      path.join(repoRoot, 'contracts', 'openapi', 'octopus.openapi.yaml'),
      'utf8',
    )

    expect(bundledContract).toContain('recentTasks:')
    expect(bundledContract).toContain('taskCount:')
    expect(bundledContract).toContain('activeTaskCount:')
    expect(bundledContract).toContain('attentionTaskCount:')
    expect(bundledContract).toContain('scheduledTaskCount:')
    expect(bundledContract).toContain('tasks:')
  })

  it('bundles multi-file contract sources into a stable single OpenAPI artifact', () => {
    const tempDir = createTempDir('octopus-openapi-bundle-')
    const sourceDir = path.join(tempDir, 'contracts', 'openapi', 'src')
    const outputPath = path.join(tempDir, 'contracts', 'openapi', 'octopus.openapi.yaml')

    writeFile(path.join(sourceDir, 'info.yaml'), [
      'title: Fixture API',
      'version: 1.2.3',
      'description: Fixture contract.',
      '',
    ].join('\n'))
    writeFile(path.join(sourceDir, 'paths', 'host.yaml'), [
      '/api/v1/host/health:',
      '  get:',
      '    operationId: getHostHealth',
      '    responses:',
      "      '200':",
      '        description: Host health payload.',
      '        content:',
      '          application/json:',
      '            schema:',
      "              $ref: '#/components/schemas/HealthcheckStatus'",
      '',
    ].join('\n'))
    writeFile(path.join(sourceDir, 'paths', 'misc.yaml'), [
      '/api/v1/apps:',
      '  get:',
      '    operationId: listApps',
      '    responses:',
      "      '200':",
      '        description: App list payload.',
      '        content:',
      '          application/json:',
      '            schema:',
      '              type: array',
      '              items:',
      "                $ref: '#/components/schemas/ClientAppRecord'",
      '',
    ].join('\n'))
    writeFile(path.join(sourceDir, 'components', 'schemas', 'shared.yaml'), [
      'ClientAppRecord:',
      '  type: object',
      '  required: [id, name]',
      '  properties:',
      '    id:',
      '      type: string',
      '    name:',
      '      type: string',
      '',
    ].join('\n'))
    writeFile(path.join(sourceDir, 'components', 'schemas', 'host.yaml'), [
      'HealthcheckStatus:',
      '  type: object',
      '  required: [status]',
      '  properties:',
      '    status:',
      '      type: string',
      '',
    ].join('\n'))
    writeFile(path.join(sourceDir, 'components', 'parameters', 'common.yaml'), '{}\n')
    writeFile(path.join(sourceDir, 'components', 'responses', 'errors.yaml'), '{}\n')
    writeFile(path.join(sourceDir, 'root.yaml'), [
      'openapi: 3.1.0',
      'info:',
      '  $ref: ./info.yaml',
      'servers:',
      '  - url: http://127.0.0.1:15421',
      'paths:',
      '  x-octopus-merge:',
      '    - ./paths/misc.yaml',
      '    - ./paths/host.yaml',
      'components:',
      '  schemas:',
      '    x-octopus-merge:',
      '      - ./components/schemas/shared.yaml',
      '      - ./components/schemas/host.yaml',
      '  parameters:',
      '    $ref: ./components/parameters/common.yaml',
      '  responses:',
      '    $ref: ./components/responses/errors.yaml',
      '',
    ].join('\n'))

    const firstOutput = bundleFixture(path.join(sourceDir, 'root.yaml'), outputPath)
    const secondOutput = bundleFixture(path.join(sourceDir, 'root.yaml'), outputPath)

    expect(firstOutput).toBe(secondOutput)
    expect(firstOutput).toContain('openapi: 3.1.0')
    expect(firstOutput).toContain('title: Fixture API')
    expect(firstOutput.indexOf('/api/v1/apps:')).toBeLessThan(firstOutput.indexOf('/api/v1/host/health:'))
    expect(firstOutput.indexOf('ClientAppRecord:')).toBeLessThan(firstOutput.indexOf('HealthcheckStatus:'))
  })

  it('fails when merged sources define duplicate schema keys', () => {
    const tempDir = createTempDir('octopus-openapi-duplicate-')
    const sourceDir = path.join(tempDir, 'contracts', 'openapi', 'src')
    const outputPath = path.join(tempDir, 'contracts', 'openapi', 'octopus.openapi.yaml')

    writeFile(path.join(sourceDir, 'info.yaml'), 'title: Fixture API\nversion: 1.2.3\n')
    writeFile(path.join(sourceDir, 'components', 'schemas', 'shared.yaml'), [
      'DuplicateRecord:',
      '  type: object',
      '  properties:',
      '    id:',
      '      type: string',
      '',
    ].join('\n'))
    writeFile(path.join(sourceDir, 'components', 'schemas', 'misc.yaml'), [
      'DuplicateRecord:',
      '  type: object',
      '  properties:',
      '    slug:',
      '      type: string',
      '',
    ].join('\n'))
    writeFile(path.join(sourceDir, 'components', 'parameters', 'common.yaml'), '{}\n')
    writeFile(path.join(sourceDir, 'components', 'responses', 'errors.yaml'), '{}\n')
    writeFile(path.join(sourceDir, 'root.yaml'), [
      'openapi: 3.1.0',
      'info:',
      '  $ref: ./info.yaml',
      'paths: {}',
      'components:',
      '  schemas:',
      '    x-octopus-merge:',
      '      - ./components/schemas/shared.yaml',
      '      - ./components/schemas/misc.yaml',
      '  parameters:',
      '    $ref: ./components/parameters/common.yaml',
      '  responses:',
      '    $ref: ./components/responses/errors.yaml',
      '',
    ].join('\n'))

    expect(() =>
      execFileSync(nodeExecutable, [bundleScriptPath, '--root', path.join(sourceDir, 'root.yaml'), '--output', outputPath], {
        cwd: repoRoot,
        encoding: 'utf8',
        stdio: 'pipe',
      }),
    ).toThrowError(/duplicate/i)
  })

  it('fails when a source file contains a dangling external ref', () => {
    const tempDir = createTempDir('octopus-openapi-dangling-ref-')
    const sourceDir = path.join(tempDir, 'contracts', 'openapi', 'src')
    const outputPath = path.join(tempDir, 'contracts', 'openapi', 'octopus.openapi.yaml')

    writeFile(path.join(sourceDir, 'root.yaml'), [
      'openapi: 3.1.0',
      'info:',
      '  $ref: ./missing-info.yaml',
      'paths: {}',
      'components:',
      '  schemas: {}',
      '  parameters: {}',
      '  responses: {}',
      '',
    ].join('\n'))

    expect(() =>
      execFileSync(nodeExecutable, [bundleScriptPath, '--root', path.join(sourceDir, 'root.yaml'), '--output', outputPath], {
        cwd: repoRoot,
        encoding: 'utf8',
        stdio: 'pipe',
      }),
    ).toThrowError(/missing-info\.yaml|ref/i)
  })
})
