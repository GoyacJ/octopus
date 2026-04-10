<script setup lang="ts">
import { User, Users, Building2 } from 'lucide-vue-next'
const { t } = useI18n()

useHead({
  title: t('pages.scenarios.title')
})

const segments = [
  {
    icon: User,
    title: '个人效能：私人数字宠物',
    desc: '在桌面端提供无时无刻的智能化陪伴。不仅是处理日常任务（写代码、查文档、订机票），更是具有个性化情感与记忆的数字宠物，让交互更有温度。',
    features: ['多模态交互体验', '端侧模型极速响应', '个人知识库自动化管理']
  },
  {
    icon: Users,
    title: '团队协作：数字员工池',
    desc: '为您的团队注入 AI 原生动力。您可以为项目快速分配具有不同职责的 AI Agent（如：代码审计员、文案策划师、项目督办员），让他们与人类成员并行协作。',
    features: ['Agent 任务分配中心', '多 Agent 协作流', '团队知识大脑共享']
  },
  {
    icon: Building2,
    title: '企业部署：全链路安全可控',
    desc: '面向大中型企业、政务单位、金融机构的标准化私有化部署方案。完全兼容信创环境，提供严谨的权限审计与数据安全围栏。',
    features: ['信创全栈深度适配', 'RBAC 与精细化管控', '全链路合规审计日志']
  }
]
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <!-- Hero Section -->
    <UiSectionHero
      align="left"
      badge="应用场景 (Scenarios)"
      :title="t('pages.scenarios.title')"
      :subtitle="t('pages.scenarios.body')"
    />

    <!-- Scenarios List -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-10">
          <UiCard v-for="seg in segments" :key="seg.title" variant="default" padding="lg" hover v-reveal class="flex flex-col card-shine border-[var(--website-border-strong)]">
            <div class="w-16 h-16 rounded-2xl bg-[var(--website-accent)]/10 flex items-center justify-center text-[var(--website-accent)] mb-10 transition-transform duration-500 hover:rotate-12">
              <component :is="seg.icon" class="w-8 h-8" />
            </div>
            <h3 class="text-3xl font-bold mb-6 tracking-tight">{{ seg.title }}</h3>
            <p class="text-[var(--website-text-muted)] text-base md:text-lg leading-relaxed mb-12 flex-grow font-medium">
              {{ seg.desc }}
            </p>
            <div class="space-y-5 pt-8 border-t border-[var(--website-border)]">
              <div v-for="feat in seg.features" :key="feat" class="flex items-center gap-4">
                <div class="w-2 h-2 rounded-full bg-[var(--website-accent)] shadow-[0_0_10px_var(--website-accent)]"></div>
                <span class="text-sm font-bold uppercase tracking-widest text-[var(--website-text)]">{{ feat }}</span>
              </div>
            </div>
          </UiCard>
        </div>
      </div>
    </section>
  </div>
</template>
