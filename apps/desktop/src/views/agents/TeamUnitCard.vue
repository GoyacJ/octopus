<script setup lang="ts">
import { ArrowRight, Orbit, Trash2, UsersRound, Shield } from 'lucide-vue-next'
import { UiBadge, UiButton } from '@octopus/ui'

const props = defineProps<{
  id: string
  name: string
  title: string
  description: string
  leadLabel: string
  members: string[]
  workflow: string[]
  recentOutcome: string
  originLabel?: string
  openLabel?: string
  removeLabel?: string
  openTestId?: string
  removeTestId?: string
}>()

const emit = defineEmits<{
  open: [id: string]
  remove: [id: string]
}>()
</script>

<template>
  <div 
    class="group relative flex min-h-[110px] w-full flex-col justify-between gap-4 rounded-2xl border border-border/30 bg-background/50 p-4 backdrop-blur-md transition-all duration-300 hover:border-indigo-500/30 hover:bg-accent/40 hover:shadow-xl hover:shadow-indigo-500/5 dark:border-white/[0.05] dark:hover:border-white/[0.12]"
    @click="emit('open', props.id)"
    role="button"
    tabindex="0"
  >
    <div class="flex items-start gap-4">
      <!-- Left: Icon with Ring -->
      <div class="relative flex size-12 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-500/20 to-transparent p-[1.5px] transition-transform group-hover:scale-105">
        <div class="flex size-full items-center justify-center overflow-hidden rounded-[calc(var(--radius-xl)-2px)] bg-background shadow-sm">
          <UsersRound :size="22" class="text-indigo-600/70" />
        </div>
      </div>

      <!-- Center: Primary Info -->
      <div class="flex flex-1 flex-col min-w-0 pt-0.5">
        <div class="flex items-center gap-2 mb-1">
          <h3 class="truncate text-[15px] font-bold tracking-tight text-text-primary group-hover:text-indigo-600 transition-colors">{{ props.name }}</h3>
          <UiBadge v-if="props.originLabel" :label="props.originLabel" variant="outline" class="h-4 px-1 text-[9px] font-black bg-indigo-500/5 border-indigo-500/20 text-indigo-600/80" />
        </div>
        
        <span class="mb-1.5 inline-block text-[10px] font-bold uppercase tracking-widest text-indigo-600/60 truncate">{{ props.title || '数字员工团队' }}</span>
        
        <p class="line-clamp-2 text-[12px] leading-relaxed text-text-secondary/80">
          {{ props.description }}
        </p>
      </div>
    </div>

    <!-- Bottom: Members & Leader -->
    <div class="flex items-center justify-between border-t border-border/20 pt-3">
      <div class="flex items-center gap-3">
        <div class="flex -space-x-2 transition-transform group-hover:translate-x-1">
          <div v-for="i in Math.min(props.members.length, 3)" :key="i" class="flex size-6 items-center justify-center rounded-full border-2 border-background bg-indigo-50 text-[8px] font-black text-indigo-600 shadow-sm">
            {{ props.members[i-1].slice(0, 1).toUpperCase() }}
          </div>
          <div v-if="props.members.length > 3" class="flex size-6 items-center justify-center rounded-full border-2 border-background bg-accent text-[8px] font-black text-text-tertiary shadow-sm">
            +{{ props.members.length - 3 }}
          </div>
        </div>
        <div class="flex flex-col">
          <span class="text-[8px] font-black uppercase tracking-[0.1em] text-text-tertiary/40 leading-none mb-1">负责人</span>
          <span class="text-[11px] font-bold text-text-primary/70 leading-none truncate max-w-[100px]">{{ props.leadLabel }}</span>
        </div>
      </div>
      
      <div class="flex gap-1.5">
        <span v-for="tag in props.workflow.slice(0, 2)" :key="tag" class="rounded-md bg-indigo-500/[0.04] px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider text-indigo-600/60 border border-indigo-500/10">
          {{ tag }}
        </span>
      </div>
    </div>

    <!-- Float Actions -->
    <div class="absolute right-2 top-2 flex scale-90 gap-1 opacity-0 transition-all duration-300 group-hover:scale-100 group-hover:opacity-100">
      <UiButton
        variant="ghost"
        size="icon"
        class="size-7 rounded-full bg-background/80 text-text-tertiary shadow-sm backdrop-blur-sm hover:text-indigo-600"
        :aria-label="props.openLabel || '打开'"
        :data-testid="props.openTestId"
        @click.stop="emit('open', props.id)"
      >
        <ArrowRight :size="14" />
        <span class="sr-only">{{ props.openLabel || '打开' }}</span>
      </UiButton>
      <UiButton
        variant="ghost"
        size="icon"
        class="size-7 rounded-full bg-background/80 text-text-tertiary shadow-sm backdrop-blur-sm hover:text-error"
        :aria-label="props.removeLabel || '删除'"
        :data-testid="props.removeTestId"
        @click.stop="emit('remove', props.id)"
      >
        <Trash2 :size="14" />
        <span class="sr-only">{{ props.removeLabel || '删除' }}</span>
      </UiButton>
    </div>
  </div>
</template>
