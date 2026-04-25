<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { SendHorizontal, Sparkles } from 'lucide-vue-next'

import type { PetMessage, PetProfile } from '@octopus/schema'

import { UiBadge, UiButton, UiInput, UiSurface } from '@octopus/ui'

const props = defineProps<{
  pet: PetProfile
  messages: PetMessage[]
  onSend: (content: string) => Promise<boolean> | boolean
}>()

const draft = ref('')
const trimmedDraft = computed(() => draft.value.trim())
const sending = ref(false)

watch(() => props.pet.id, () => {
  draft.value = ''
})

async function submit() {
  if (!trimmedDraft.value || sending.value) {
    return
  }

  const content = trimmedDraft.value
  sending.value = true
  try {
    const submitted = await props.onSend(content)
    if (submitted) {
      draft.value = ''
    }
  } finally {
    sending.value = false
  }
}
</script>

<template>
  <UiSurface
    variant="glass"
    padding="md"
    class="pet-chat-panel border-primary/20 shadow-2xl highlight-border backdrop-blur-2xl"
    data-testid="desktop-pet-chat"
    :title="pet.displayName"
    :subtitle="pet.summary"
  >
    <template #actions>
       <div class="flex gap-2">
         <UiBadge :label="pet.species" class="bg-primary/10 text-primary border-primary/20 text-[9px]" />
         <UiBadge v-if="pet.favoriteSnack" :label="pet.favoriteSnack" class="bg-black/20 text-text-tertiary border-border/30 text-[9px]" />
       </div>
    </template>

    <div class="pet-chat-messages scroll-y pr-1" data-testid="desktop-pet-messages">
      <div
        v-for="message in messages"
        :key="message.id"
        class="flex flex-col gap-1"
        :class="message.sender === 'user' ? 'items-end' : 'items-start'"
      >
        <div 
          class="px-3 py-2 rounded-2xl text-[13px] leading-relaxed max-w-[90%] transition-all"
          :class="[
            message.sender === 'user' 
              ? 'bg-primary text-primary-foreground shadow-sm' 
              : 'bg-black/30 border border-primary/10 text-text-primary shadow-inner highlight-border'
          ]"
        >
          <span class="block text-[9px] font-bold uppercase tracking-tighter opacity-50 mb-0.5">
            {{ message.sender === 'user' ? 'You' : pet.displayName }}
          </span>
          {{ message.content }}
        </div>
      </div>
    </div>

    <div v-if="pet.promptHints?.length" class="pet-chat-hints flex flex-wrap gap-2 mt-4 pt-4 border-t border-border/20">
      <button
        v-for="hint in pet.promptHints"
        :key="hint"
        type="button"
        class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-primary/5 border border-primary/10 text-[11px] font-bold text-primary/80 transition-all hover:bg-primary/10 hover:border-primary/30"
        @click="draft = hint"
      >
        <Sparkles :size="10" />
        <span>{{ hint }}</span>
      </button>
    </div>

    <div class="pet-chat-composer mt-4 flex items-center gap-3">
      <UiInput
        v-model="draft"
        data-testid="desktop-pet-input"
        class="flex-1 h-10 bg-black/20 border-border/30 rounded-xl text-sm"
        :placeholder="`和 ${pet.displayName} 交流...`"
        @keydown.enter="submit"
      />
      <UiButton 
        size="icon" 
        data-testid="desktop-pet-send" 
        :disabled="!trimmedDraft" 
        class="h-10 w-10 shrink-0 rounded-xl shadow-lg shadow-primary/20"
        @click="submit"
      >
        <SendHorizontal :size="18" stroke-width="2.5" />
      </UiButton>
    </div>
  </UiSurface>
</template>

<style scoped>
.pet-chat-panel {
  width: min(24rem, calc(100vw - 2rem));
}

.pet-chat-messages {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  max-height: 18rem;
  overflow-y: auto;
}
</style>
