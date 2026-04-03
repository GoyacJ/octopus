// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import DesktopPetChat from '@/components/pet/DesktopPetChat.vue'
import DesktopPetHost from '@/components/pet/DesktopPetHost.vue'
import { useWorkbenchStore } from '@/stores/workbench'

const petFixture = {
  id: 'pet-user-admin',
  species: 'duck',
  displayName: 'Bubbles Duck',
  ownerUserId: 'user-admin',
  avatarLabel: 'DU',
  summary: 'A friendly floating companion.',
  greeting: '你好呀，我是 Bubbles Duck。',
  mood: 'happy',
  favoriteSnack: 'blueberry cookie',
  promptHints: ['给我一个轻松提醒'],
  fallbackAsset: '/src/assets/pets/duck.svg',
  stateMachine: 'PetState',
} as const

describe('Desktop pet widget', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders pet chat messages and emits send events', async () => {
    const wrapper = mount(DesktopPetChat, {
      props: {
        pet: petFixture,
        messages: [
          {
            id: 'pet-msg-1',
            petId: petFixture.id,
            sender: 'pet',
            content: petFixture.greeting,
            timestamp: 1,
          },
        ],
      },
    })

    expect(wrapper.text()).toContain('Bubbles Duck')
    expect(wrapper.text()).toContain('blueberry cookie')

    const hint = wrapper.get('button.pet-chat-hint')
    await hint.trigger('click')
    expect((wrapper.get('[data-testid="desktop-pet-input"]').element as HTMLInputElement).value).toContain('轻松提醒')

    await wrapper.get('[data-testid="desktop-pet-send"]').trigger('click')
    expect(wrapper.emitted('send')?.[0]).toEqual(['给我一个轻松提醒'])
  })

  it('uses the store-backed host to open pet chat and send a reply', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useWorkbenchStore()
    const wrapper = mount(DesktopPetHost, {
      global: {
        plugins: [pinia],
      },
    })

    expect(wrapper.get('[data-testid="desktop-pet-host"]').exists()).toBe(true)
    await wrapper.get('[data-testid="desktop-pet-trigger"]').trigger('click')
    expect(store.currentUserPetPresence?.chatOpen).toBe(true)

    await wrapper.get('[data-testid="desktop-pet-input"]').setValue('你好，鸭鸭')
    await wrapper.get('[data-testid="desktop-pet-send"]').trigger('click')

    expect(store.currentUserPetMessages.at(-2)).toMatchObject({ sender: 'user', content: '你好，鸭鸭' })
    expect(store.currentUserPetMessages.at(-1)?.sender).toBe('pet')
  })
})
