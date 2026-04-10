<script setup lang="ts">
import { Search } from 'lucide-vue-next'
import { UiField, UiFilterChipGroup, UiInput, UiSelect, UiTabs, UiToolbarRow } from '@octopus/ui'

interface SelectOption {
  label: string
  value: string
  disabled?: boolean
}

const props = defineProps<{
  searchQuery: string
  activeTab: 'agent' | 'team'
  status: string
  scenario: string
  sort: string
  statusOptions: SelectOption[]
  scenarioOptions: SelectOption[]
  sortOptions: SelectOption[]
  quickTags: string[]
  selectedQuickTag: string
}>()

const emit = defineEmits<{
  'update:searchQuery': [value: string]
  'update:activeTab': [value: 'agent' | 'team']
  'update:status': [value: string]
  'update:scenario': [value: string]
  'update:sort': [value: string]
  'update:selectedQuickTag': [value: string]
}>()

const tabs = [
  { value: 'agent', label: '员工' },
  { value: 'team', label: '数字团队' },
]

function handleTabChange(value: string) {
  emit('update:activeTab', value === 'team' ? 'team' : 'agent')
}
</script>

<template>
  <UiToolbarRow test-id="agent-center-toolbar">
    <template #search>
      <div class="relative min-w-0">
        <Search :size="16" class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-tertiary" />
        <UiInput
          :model-value="props.searchQuery"
          class="pl-10"
          data-testid="agent-center-search"
          placeholder="搜索名称、简介、标签或角色…"
          @update:model-value="emit('update:searchQuery', $event)"
        />
      </div>
    </template>

    <template #tabs>
      <UiTabs
        :model-value="props.activeTab"
        variant="segmented"
        :tabs="tabs"
        @update:model-value="handleTabChange"
      />
    </template>

    <template #filters>
      <div class="grid min-w-0 flex-1 gap-3 md:grid-cols-3">
        <UiField label="状态">
          <UiSelect :model-value="props.status" :options="props.statusOptions" @update:model-value="emit('update:status', $event)" />
        </UiField>
        <UiField label="场景">
          <UiSelect :model-value="props.scenario" :options="props.scenarioOptions" @update:model-value="emit('update:scenario', $event)" />
        </UiField>
        <UiField label="排序">
          <UiSelect :model-value="props.sort" :options="props.sortOptions" @update:model-value="emit('update:sort', $event)" />
        </UiField>
      </div>
    </template>

    <template #chips>
      <UiFilterChipGroup
        :model-value="props.selectedQuickTag"
        :items="props.quickTags.map((tag) => ({ value: tag, label: tag }))"
        test-id="agent-center-quick-tags"
        @update:model-value="emit('update:selectedQuickTag', $event)"
      />
    </template>
  </UiToolbarRow>
</template>
