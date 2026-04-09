// @vitest-environment jsdom

import fs from 'node:fs'
import path from 'node:path'

import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import type { PetProfile } from '@octopus/schema'
import { petAssetMap } from '@octopus/assets/pets'

import DesktopPetAvatar from '@/components/pet/DesktopPetAvatar.vue'

const basePetProfile: PetProfile = {
  id: 'pet-octopus',
  displayName: '小章',
  species: 'octopus',
  ownerUserId: 'user-owner',
  avatarLabel: 'Octopus mascot',
  summary: 'Octopus 首席吉祥物，负责卖萌和加油。',
  greeting: '嗨！我是小章，今天也要加油哦！',
  mood: 'happy',
  favoriteSnack: '新鲜小虾',
  promptHints: ['最近有什么好消息？'],
  fallbackAsset: 'octopus',
}

function mountAvatar(pet: PetProfile) {
  return mount(DesktopPetAvatar, {
    props: {
      pet,
      motionState: 'idle',
      unreadCount: 0,
    },
  })
}

describe('pet assets', () => {
  it('exposes a shared asset for every supported pet species', () => {
    expect(Object.keys(petAssetMap).sort()).toEqual([
      'axolotl',
      'blob',
      'cactus',
      'capybara',
      'cat',
      'chonk',
      'dragon',
      'duck',
      'ghost',
      'goose',
      'mushroom',
      'octopus',
      'owl',
      'penguin',
      'rabbit',
      'robot',
      'snail',
      'turtle',
    ])
  })

  it('renders the shared species asset for the current pet', () => {
    const wrapper = mountAvatar(basePetProfile)

    expect(wrapper.get('[data-testid="desktop-pet-image"]').attributes('src')).toContain('/octopus.svg')
  })

  it('resolves fallbackAsset through the shared pet asset map before treating it as a raw URL', () => {
    const wrapper = mountAvatar({
      ...basePetProfile,
      species: 'unknown' as PetProfile['species'],
      fallbackAsset: 'duck',
    })

    expect(wrapper.get('[data-testid="desktop-pet-image"]').attributes('src')).toContain('/duck.svg')
  })

  it('uses a raw fallbackAsset URL when it does not match a shared pet species', () => {
    const wrapper = mountAvatar({
      ...basePetProfile,
      species: 'unknown' as PetProfile['species'],
      fallbackAsset: 'https://cdn.example.com/pets/custom.svg',
    })

    expect(wrapper.get('[data-testid="desktop-pet-image"]').attributes('src')).toBe('https://cdn.example.com/pets/custom.svg')
  })

  it('renders the compact sidebar avatar variant through the component size prop', () => {
    const wrapper = mount(DesktopPetAvatar, {
      props: {
        pet: basePetProfile,
        motionState: 'idle',
        unreadCount: 0,
        size: 'sidebar',
      },
    })

    expect(wrapper.get('[data-testid="desktop-pet-trigger"]').classes()).toContain('pet-avatar-button--sidebar')
  })

  it('keeps the shared pet assets free of baked background fills', () => {
    const petsDir = path.resolve(import.meta.dirname, '../../../packages/assets/pets')
    const petFiles = fs.readdirSync(petsDir).filter(file => file.endsWith('.svg'))

    for (const petFile of petFiles) {
      const asset = fs.readFileSync(path.join(petsDir, petFile), 'utf8')
      const normalizedAsset = asset.replace(/\s+/g, ' ')

      expect(normalizedAsset).not.toMatch(/^<svg[^>]*>\s*(?:<g[^>]*>\s*)?<(?:rect|circle)\b[^>]*fill="#[0-9A-Fa-f]{3,8}"[^>]*\/>/)
    }
  })
})
