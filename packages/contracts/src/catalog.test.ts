import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'vitest'

import * as exportedContracts from './index'
import { contractCatalog } from './catalog'

interface CoreObjectFile {
  version: string
  objects: Array<{
    name: string
    bounded_context: string
    required_fields: string[]
    notes: string
  }>
}

interface EnumFile {
  version: string
  enums: Record<string, string[]>
}

interface EventFile {
  version: string
  events: Array<{
    name: string
    category: string
    required_fields: string[]
  }>
}

const readJson = <T>(relativePath: string): T =>
  JSON.parse(readFileSync(new URL(relativePath, import.meta.url), 'utf8')) as T

describe('contractCatalog', () => {
  it('includes KnowledgeCandidate in the frozen core object catalog', () => {
    expect(contractCatalog.coreObjects.some((entry) => entry.name === 'KnowledgeCandidate')).toBe(true)
  })

  it('mirrors the canonical enum catalog', () => {
    const enumFile = readJson<EnumFile>('../../../contracts/v1/enums.json')

    expect(contractCatalog.enums).toEqual(enumFile.enums)
  })

  it('mirrors the canonical object names', () => {
    const coreObjectFile = readJson<CoreObjectFile>('../../../contracts/v1/core-objects.json')

    expect(contractCatalog.coreObjects.map((entry) => entry.name)).toEqual(
      coreObjectFile.objects.map((entry) => entry.name),
    )
    expect(contractCatalog.coreObjects).toEqual(coreObjectFile.objects)
  })

  it('mirrors the canonical event names', () => {
    const eventFile = readJson<EventFile>('../../../contracts/v1/events.json')

    expect(contractCatalog.events.map((entry) => entry.name)).toEqual(
      eventFile.events.map((entry) => entry.name),
    )
    expect(contractCatalog.events).toEqual(eventFile.events)
  })

  it('exports explicit run-flow enum catalogs for approvals and inbox state', () => {
    expect(exportedContracts).toMatchObject({
      approvalDecisionValues: ['approved', 'rejected'],
      approvalStateValues: ['pending', 'approved', 'rejected', 'expired', 'cancelled'],
      inboxStateValues: ['open', 'acknowledged', 'resolved', 'dismissed', 'expired'],
    })
  })
})
