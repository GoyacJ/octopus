<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { SendHorizontal, Sparkles } from 'lucide-vue-next'

import type { PetMessage, PetProfile } from '@octopus/schema'

import { UiBadge, UiButton, UiInput, UiSurface } from '@octopus/ui'

const props = defineProps<{
  pet: PetProfile
  messages: PetMessage[]
}>()

const emit = defineEmits<{
  send: [content: string]
}>()

const draft = ref('')
const trimmedDraft = computed(() => draft.value.trim())

watch(() => props.pet.id, () => {
  draft.value = ''
})

function submit() {
  if (!trimmedDraft.value) {
    return
  }

  emit('send', trimmedDraft.value)
  draft.value = ''
}
</script>

<template>
  <UiSurface
    variant="overlay"
    padding="sm"
    class="pet-chat-panel"
    data-testid="desktop-pet-chat"
    :title="pet.displayName"
    :subtitle="pet.summary"
  >
    <div class="pet-chat-meta">
      <UiBadge :label="pet.species" tone="info" />
      <UiBadge :label="pet.favoriteSnack" subtle />
    </div>

    <div class="pet-chat-messages" data-testid="desktop-pet-messages">
      <div
        v-for="message in messages"
        :key="message.id"
        class="pet-chat-bubble"
        :class="message.sender === 'user' ? 'is-user' : 'is-pet'"
      >
        <span class="pet-chat-sender">{{ message.sender === 'user' ? '你' : pet.displayName }}</span>
        <p>{{ message.content }}</p>
      </div>
    </div>

    <div class="pet-chat-hints">
      <button
        v-for="hint in pet.promptHints"
        :key="hint"
        type="button"
        class="pet-chat-hint"
        @click="draft = hint"
      >
        <Sparkles :size="12" />
        <span>{{ hint }}</span>
      </button>
    </div>

    <div class="pet-chat-composer">
      <UiInput
        v-model="draft"
        data-testid="desktop-pet-input"
        :placeholder="`和 ${pet.displayName} 说点什么`"
        @keydown.enter="submit"
      />
      <UiButton size="icon" data-testid="desktop-pet-send" :disabled="!trimmedDraft" @click="submit">
        <SendHorizontal :size="16" />
      </UiButton>
    </div>
  </UiSurface>
</template>

<style scoped>
.pet-chat-panel {
  width: min(22rem, calc(100vw - 2rem));
}

.pet-chat-meta,
.pet-chat-hints,
.pet-chat-composer {
  display: flex;
  gap: 0.5rem;
}

.pet-chat-meta {
  flex-wrap: wrap;
  margin-bottom: 0.7rem;
}

.pet-chat-messages {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  max-height: 16rem;
  margin-bottom: 0.8rem;
  overflow-y: auto;
}

.pet-chat-bubble {
  max-width: 88%;
  padding: 0.7rem 0.8rem;
  border-radius: 1rem;
  box-shadow: var(--shadow-xs);
}

.pet-chat-bubble.is-pet {
  align-self: flex-start;
  background: color-mix(in srgb, var(--bg-surface) 94%, white 6%);
}

.pet-chat-bubble.is-user {
  align-self: flex-end;
  background: color-mix(in srgb, var(--brand-primary) 14%, var(--bg-surface));
}

.pet-chat-sender {
  display: block;
  margin-bottom: 0.2rem;
  color: var(--text-tertiary);
  font-size: 0.68rem;
  font-weight: 700;
}

.pet-chat-bubble p {
  margin: 0;
  font-size: 0.84rem;
  line-height: 1.5;
}

.pet-chat-hints {
  flex-wrap: wrap;
  margin-bottom: 0.8rem;
}

.pet-chat-hint {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.45rem 0.65rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-subtle) 86%, transparent);
  color: var(--text-secondary);
  font-size: 0.74rem;
}

.pet-chat-composer {
  align-items: center;
}
</style>
