<script setup lang="ts">
import { Sparkles, Trash2, Zap } from 'lucide-vue-next'
import { UiBadge, UiButton } from '@octopus/ui'

const props = defineProps<{
  id: string
  name: string
  role: string
  summary: string
  recentTask: string
  avatar: string
  statusLabel: string
  statusTone: 'success' | 'warning' | 'default'
  skills: string[]
  metrics: Array<{ label: string; value: string }>
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
    class="group relative flex min-h-[110px] w-full flex-col justify-between gap-4 rounded-2xl border border-border/30 bg-background/50 p-4 backdrop-blur-md transition-all duration-300 hover:border-primary/30 hover:bg-accent/40 hover:shadow-xl hover:shadow-primary/5 dark:border-white/[0.05] dark:hover:border-white/[0.12]"
    @click="emit('open', props.id)"
    role="button"
    tabindex="0"
  >
    <div class="flex items-start gap-4">
      <!-- Left: Avatar with Ring -->
      <div class="relative flex size-12 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-primary/20 to-transparent p-[1.5px] transition-transform group-hover:scale-105">
        <div class="flex size-full items-center justify-center overflow-hidden rounded-[calc(var(--radius-xl)-2px)] bg-background shadow-sm">
          <img v-if="props.avatar.startsWith('data:image/')" :src="props.avatar" alt="" class="size-full object-cover opacity-90 transition-opacity group-hover:opacity-100">
          <span v-else class="text-[14px] font-black tracking-tight text-primary/70">{{ props.avatar.slice(0, 2).toUpperCase() }}</span>
        </div>
        <!-- Soft status glow -->
        <div 
          class="absolute -bottom-0.5 -right-0.5 size-3.5 rounded-full border-2 border-background shadow-xs transition-colors"
          :class="props.statusTone === 'success' ? 'bg-emerald-500 shadow-emerald-500/20' : 'bg-slate-400 shadow-slate-400/20'"
        />
      </div>

      <!-- Center: Primary Info -->
      <div class="flex flex-1 flex-col min-w-0 pt-0.5">
        <div class="flex items-center gap-2 mb-1">
          <h3 class="truncate text-[15px] font-bold tracking-tight text-text-primary group-hover:text-primary transition-colors">{{ props.name }}</h3>
          <UiBadge v-if="props.originLabel" :label="props.originLabel" variant="outline" class="h-4 px-1 text-[9px] font-black bg-primary/5 border-primary/20 text-primary/80" />
        </div>
        
        <span class="mb-1.5 inline-block text-[10px] font-bold uppercase tracking-widest text-primary/60 truncate">{{ props.role }}</span>
        
        <p class="line-clamp-2 text-[12px] leading-relaxed text-text-secondary/80">
          {{ props.summary }}
        </p>
      </div>
    </div>

    <!-- Bottom: Metrics & Tags -->
    <div class="flex items-center justify-between border-t border-border/20 pt-3">
      <div class="flex items-center gap-4">
        <div v-for="metric in props.metrics" :key="metric.label" class="flex flex-col">
          <span class="text-[8px] font-black uppercase tracking-[0.1em] text-text-tertiary/40 leading-none mb-1">{{ metric.label }}</span>
          <span class="text-[12px] font-bold tabular-nums text-text-primary/70 leading-none">{{ metric.value }}</span>
        </div>
      </div>
      
      <div class="flex gap-1.5">
        <span v-for="skill in props.skills.slice(0, 2)" :key="skill" class="rounded-md bg-primary/[0.04] px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider text-primary/60 border border-primary/10">
          {{ skill }}
        </span>
      </div>
    </div>

    <!-- Float Actions -->
    <div class="absolute right-2 top-2 flex scale-90 gap-1 opacity-0 transition-all duration-300 group-hover:scale-100 group-hover:opacity-100">
      <UiButton
        variant="ghost"
        size="icon"
        class="size-7 rounded-full bg-background/80 text-text-tertiary shadow-sm backdrop-blur-sm hover:text-primary"
        :aria-label="props.openLabel || '打开'"
        :data-testid="props.openTestId"
        @click.stop="emit('open', props.id)"
      >
        <Sparkles :size="14" />
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
