<script setup lang="ts">
import { 
  Puzzle, 
  Terminal, 
  Activity, 
  Layers, 
  Monitor, 
  ShieldAlert,
  ArrowRight
} from 'lucide-vue-next'

const { t } = useI18n()

useHead({
  title: t('pages.product.title')
})

const capabilities = [
  { key: 'mcp', icon: Puzzle, img: '/images/mcp.png' },
  { key: 'sandbox', icon: Terminal, img: '/images/builtin.png' },
  { key: 'telemetry', icon: Activity, img: '/images/project-setting.png' },
  { key: 'plugin', icon: Layers, img: '/images/model.png' },
  { key: 'desktop', icon: Monitor, img: '/images/dashboard.png' },
  { key: 'enterprise', icon: ShieldAlert, img: '/images/rbac.png' }
]
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <!-- Global Decorative Elements -->
    <div class="glow-orb w-[600px] h-[600px] bg-orange-500/20 top-[20%] right-[-300px]"></div>

    <!-- Hero Section -->
    <UiSectionHero
      align="split"
      badge="内核能力 (Octopus Core)"
      title="硬核 AI"
      highlight="基础设施"
      :subtitle="t('pages.product.body')"
    >
      <template #visual>
        <div class="relative group">
          <div class="absolute -inset-4 bg-gradient-to-tr from-orange-600 to-amber-400 rounded-[2rem] blur-2xl opacity-10"></div>
          <UiCard variant="glass" padding="none" class="shadow-2xl border-[var(--website-border-strong)] rounded-3xl overflow-hidden">
            <img src="/images/mcp.png" alt="Octopus MCP" class="w-full h-auto" />
          </UiCard>
        </div>
      </template>
    </UiSectionHero>

    <!-- Capabilities Grid -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-10">
          <div 
            v-for="(cap, index) in capabilities" 
            :key="cap.key"
            class="group"
            v-reveal
          >
            <UiCard padding="none" hover class="h-full flex flex-col overflow-hidden card-shine border-[var(--website-border-strong)]">
              <!-- Visual Preview -->
              <div class="aspect-video bg-[var(--website-surface-soft)] overflow-hidden relative border-b border-[var(--website-border)]">
                <img :src="cap.img" class="w-full h-full object-cover transition-transform duration-700 group-hover:scale-110" :alt="cap.key" />
                <div class="absolute inset-0 bg-gradient-to-t from-[var(--website-surface)]/60 to-transparent"></div>
              </div>
              
              <!-- Content -->
              <div class="p-6 flex-grow">
                <div class="w-10 h-10 rounded-lg bg-[var(--website-accent)]/10 flex items-center justify-center text-[var(--website-accent)] mb-4">
                  <component :is="cap.icon" class="w-5 h-5" />
                </div>
                <h3 class="text-lg font-bold mb-2">{{ t(`pages.product.features.${cap.key}.title`) }}</h3>
                <p class="text-[var(--website-text-muted)] text-sm leading-relaxed mb-6">
                  {{ t(`pages.product.features.${cap.key}.desc`) }}
                </p>
                <div class="mt-auto flex items-center text-xs font-bold text-[var(--website-accent)] opacity-0 group-hover:opacity-100 transition-opacity">
                  查看详细协议 <ArrowRight class="ml-1 w-3 h-3" />
                </div>
              </div>
            </UiCard>
          </div>
        </div>
      </div>
    </section>

    <!-- Advanced Governance Section -->
    <section class="section-padding bg-[var(--website-surface-soft)]/50">
      <div class="container-custom">
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <div>
            <h2 class="text-3xl font-bold mb-8 tracking-tight">可信、可控、可审计</h2>
            <div class="space-y-8">
              <div v-for="i in [
                { t: '沙箱隔离机制', d: 'AI 生成的每一行脚本均在独立的容器沙箱中运行，无法访问宿主受保护资源。' },
                { t: '全链路 Telemetry', d: '集成的链路追踪能力，让 Agent 的决策链透明化，轻松回溯任意时刻的推理路径。' },
                { t: 'RBAC 精细化授权', d: '对接企业级 LDAP/SSO，确保每个数字员工只能在被授权的范围内使用工具。' }
              ]" :key="i.t" class="flex gap-4" v-reveal>
                <div class="mt-1 w-5 h-5 rounded-full bg-green-500/20 flex items-center justify-center text-green-500 shrink-0">
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
                </div>
                <div>
                  <h4 class="font-bold mb-1">{{ i.t }}</h4>
                  <p class="text-sm text-[var(--website-text-muted)] leading-relaxed">{{ i.d }}</p>
                </div>
              </div>
            </div>
          </div>
          <div class="relative" v-reveal>
            <UiCard variant="glass" padding="none" class="shadow-2xl rotate-1 group transition-transform hover:rotate-0 border-[var(--website-border-strong)]">
              <img src="/images/rbac.png" alt="Octopus RBAC" class="rounded-[var(--radius-l)]" />
              <!-- Badge Decoration -->
              <div class="absolute -bottom-6 -left-6 glass px-6 py-4 rounded-2xl border border-[var(--website-border-strong)] shadow-xl">
                <div class="flex items-center gap-3">
                  <div class="w-3 h-3 rounded-full bg-green-500 animate-pulse"></div>
                  <span class="text-sm font-bold tracking-tight">System Secure</span>
                </div>
              </div>
            </UiCard>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
