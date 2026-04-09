import { describe, expect, it } from 'vitest'

import {
  createSkillDocument,
  createSkillFileDocument,
  normalizeImportedFiles,
  normalizeSkillFrontmatterName,
  skillSlugFromRelativePath,
} from './support/workspace-fixture-skill-helpers'

describe('workspace fixture skill helpers', () => {
  it('extracts workspace skill slugs from managed paths', () => {
    expect(skillSlugFromRelativePath('data/skills/research/SKILL.md')).toBe('research')
    expect(skillSlugFromRelativePath('.codex/skills/research/SKILL.md')).toBe('research')
    expect(skillSlugFromRelativePath('docs/skill.md')).toBe('')
  })

  it('normalizes imported skill archives with a single root folder', () => {
    expect(normalizeImportedFiles([
      {
        relativePath: '/my-skill/SKILL.md',
        dataBase64: btoa('---\nname: skill\n---\n'),
        byteSize: 20,
        contentType: 'text/markdown',
      },
      {
        relativePath: 'my-skill/assets/guide.md',
        dataBase64: btoa('# Guide'),
        byteSize: 7,
        contentType: 'text/markdown',
      },
    ])).toEqual([
      {
        relativePath: 'SKILL.md',
        dataBase64: btoa('---\nname: skill\n---\n'),
        byteSize: 20,
        contentType: 'text/markdown',
      },
      {
        relativePath: 'assets/guide.md',
        dataBase64: btoa('# Guide'),
        byteSize: 7,
        contentType: 'text/markdown',
      },
    ])
  })

  it('updates or inserts frontmatter names when importing skills', () => {
    expect(normalizeSkillFrontmatterName('---\ndescription: test\n---\n', 'renamed')).toBe(
      '---\ndescription: test\nname: renamed\n---\n',
    )
    expect(normalizeSkillFrontmatterName('---\nname: old\n---\n', 'renamed')).toBe(
      '---\nname: renamed\n---\n',
    )
  })

  it('builds skill documents from file maps', () => {
    const files = {
      'SKILL.md': createSkillFileDocument('skill-1', 'skill:skill-1', 'data/skills/skill-1', 'SKILL.md', {
        content: '---\nname: skill-1\n---\n',
      }),
      'assets/guide.md': createSkillFileDocument('skill-1', 'skill:skill-1', 'data/skills/skill-1', 'assets/guide.md', {
        content: '# Guide',
      }),
    }

    expect(createSkillDocument(
      'skill-1',
      'skill:skill-1',
      'Skill 1',
      'Fixture skill',
      'data/skills/skill-1/SKILL.md',
      true,
      files,
      'data/skills/skill-1/SKILL.md',
    )).toMatchObject({
      id: 'skill-1',
      rootPath: 'data/skills/skill-1',
      workspaceOwned: true,
      tree: [
        {
          kind: 'directory',
          path: 'assets',
        },
        {
          kind: 'file',
          path: 'SKILL.md',
        },
      ],
    })
  })
})
