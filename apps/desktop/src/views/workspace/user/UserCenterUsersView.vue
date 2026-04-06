<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { UserRecordSummary } from '@octopus/schema'
import { UiBadge, UiButton, UiField, UiInput, UiRecordCard, UiSelect, UiTextarea } from '@octopus/ui'

import { useUserCenterStore } from '@/stores/user-center'

const { t } = useI18n()
const userCenterStore = useUserCenterStore()

const selectedUserId = ref('')
const form = reactive({
  username: '',
  displayName: '',
  status: 'active',
  roleIds: '',
  scopeProjectIds: '',
})

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'disabled', label: 'disabled' },
]

const metrics = computed(() => [
  { id: 'total', label: t('userCenter.users.metrics.total'), value: String(userCenterStore.users.length) },
  { id: 'disabled', label: t('userCenter.users.metrics.disabled'), value: String(userCenterStore.users.filter(user => user.status === 'disabled').length) },
])

watch(
  () => userCenterStore.users.map(user => user.id).join('|'),
  () => {
    if (!selectedUserId.value || !userCenterStore.users.some(user => user.id === selectedUserId.value)) {
      applyUser(userCenterStore.users[0]?.id)
      return
    }
    applyUser(selectedUserId.value)
  },
  { immediate: true },
)

function applyUser(userId?: string) {
  const user = userCenterStore.users.find(item => item.id === userId)
  selectedUserId.value = user?.id ?? ''
  form.username = user?.username ?? ''
  form.displayName = user?.displayName ?? ''
  form.status = user?.status ?? 'active'
  form.roleIds = user?.roleIds.join(', ') ?? ''
  form.scopeProjectIds = user?.scopeProjectIds.join(', ') ?? ''
}

async function saveUser() {
  if (!form.username.trim() || !form.displayName.trim()) {
    return
  }

  const record: UserRecordSummary = {
    id: selectedUserId.value || `user-${Date.now()}`,
    username: form.username.trim(),
    displayName: form.displayName.trim(),
    avatar: userCenterStore.users.find(user => user.id === selectedUserId.value)?.avatar,
    status: form.status as UserRecordSummary['status'],
    passwordState: userCenterStore.users.find(user => user.id === selectedUserId.value)?.passwordState ?? 'reset-required',
    roleIds: form.roleIds.split(',').map(item => item.trim()).filter(Boolean),
    scopeProjectIds: form.scopeProjectIds.split(',').map(item => item.trim()).filter(Boolean),
  }

  if (selectedUserId.value) {
    await userCenterStore.updateUser(selectedUserId.value, record)
  } else {
    const created = await userCenterStore.createUser(record)
    selectedUserId.value = created.id
  }
}
</script>

<template>
  <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
    <section class="space-y-3">
      <div class="grid gap-3 md:grid-cols-2">
        <div v-for="metric in metrics" :key="metric.id" class="rounded-xl border border-border-subtle p-4 dark:border-white/[0.05]">
          <div class="text-xs text-text-secondary">{{ metric.label }}</div>
          <div class="mt-2 text-2xl font-semibold text-text-primary">{{ metric.value }}</div>
        </div>
      </div>
      <UiRecordCard
        v-for="user in userCenterStore.users"
        :key="user.id"
        :title="user.displayName"
        :description="user.username"
        interactive
        class="cursor-pointer"
        :class="selectedUserId === user.id ? 'ring-1 ring-primary' : ''"
        @click="applyUser(user.id)"
      >
        <template #leading>
          <div class="flex h-10 w-10 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-accent text-xs font-semibold uppercase text-text-secondary">
            <img v-if="user.avatar" :src="user.avatar" alt="" class="h-full w-full object-cover">
            <span v-else>{{ user.displayName.slice(0, 1) }}</span>
          </div>
        </template>
        <template #badges>
          <UiBadge :label="user.status" subtle />
          <UiBadge :label="user.passwordState" subtle />
        </template>
      </UiRecordCard>
    </section>

    <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
      <h3 class="text-base font-semibold text-text-primary">{{ selectedUserId ? t('userCenter.users.editTitle') : t('userCenter.users.createTitle') }}</h3>
      <UiField :label="t('userCenter.users.fields.username')">
        <UiInput v-model="form.username" />
      </UiField>
      <UiField :label="t('userCenter.users.fields.nickname')">
        <UiInput v-model="form.displayName" />
      </UiField>
      <UiField :label="t('common.status')">
        <UiSelect v-model="form.status" :options="statusOptions" />
      </UiField>
      <UiField :label="t('userCenter.users.fields.roleIds')">
        <UiInput v-model="form.roleIds" />
      </UiField>
      <UiField :label="t('userCenter.users.fields.scopeProjectIds')">
        <UiTextarea v-model="form.scopeProjectIds" :rows="3" />
      </UiField>
      <div class="flex gap-3">
        <UiButton @click="saveUser">{{ t('common.save') }}</UiButton>
        <UiButton variant="ghost" @click="applyUser()">{{ t('common.reset') }}</UiButton>
      </div>
    </section>
  </div>
</template>
