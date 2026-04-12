<script setup lang="ts">
import { computed, ref } from 'vue'
import { FolderSearchIcon, FolderUpIcon } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

import { UiButton, UiDialog, UiEmptyState, UiField, UiInput, UiStatusCallout } from '@octopus/ui'

import { pickResourceDirectory } from '@/tauri/client'
import { useShellStore } from '@/stores/shell'
import { resolveWorkspaceClientForConnection } from '@/stores/workspace-scope'

const props = withDefaults(defineProps<{
  modelValue?: string
  pathTestId?: string
  pickTestId?: string
}>(), {
  modelValue: '',
  pathTestId: '',
  pickTestId: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const { t } = useI18n()
const shell = useShellStore()

const dialogOpen = ref(false)
const loading = ref(false)
const error = ref('')
const currentPath = ref('')
const parentPath = ref<string | undefined>(undefined)
const entries = ref<Array<{ name: string, path: string }>>([])

const isLocalConnection = computed(() => shell.activeWorkspaceConnection?.transportSecurity === 'loopback')

async function loadRemoteDirectories(path?: string) {
  const resolved = resolveWorkspaceClientForConnection()
  if (!resolved) {
    error.value = t('resources.remoteBrowser.errors.unavailable')
    return
  }

  loading.value = true
  error.value = ''

  try {
    const payload = await resolved.client.filesystem.listDirectories(path)
    currentPath.value = payload.currentPath
    parentPath.value = payload.parentPath
    entries.value = payload.entries
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : t('resources.remoteBrowser.errors.loadFailed')
  } finally {
    loading.value = false
  }
}

async function openPicker() {
  if (isLocalConnection.value) {
    const selectedPath = await pickResourceDirectory()
    if (selectedPath) {
      emit('update:modelValue', selectedPath)
    }
    return
  }

  dialogOpen.value = true
  await loadRemoteDirectories(props.modelValue || undefined)
}

function chooseCurrentPath() {
  if (!currentPath.value) {
    return
  }
  emit('update:modelValue', currentPath.value)
  dialogOpen.value = false
}
</script>

<template>
  <UiField
    :label="t('projects.fields.resourceDirectory')"
    :hint="t(isLocalConnection ? 'projects.hints.resourceDirectoryLocal' : 'projects.hints.resourceDirectoryRemote')"
  >
    <div class="flex items-stretch gap-3">
      <UiInput
        :model-value="props.modelValue"
        :data-testid="props.pathTestId || undefined"
        readonly
        class="flex-1 font-mono"
        :placeholder="t('projects.placeholders.resourceDirectory')"
      />
      <UiButton
        type="button"
        variant="outline"
        :data-testid="props.pickTestId || undefined"
        @click="openPicker"
      >
        <FolderSearchIcon :size="14" />
        {{ t('projects.actions.pickResourceDirectory') }}
      </UiButton>
    </div>
  </UiField>

  <UiDialog
    v-model:open="dialogOpen"
    :title="t('resources.remoteBrowser.title')"
    :description="t('resources.remoteBrowser.description')"
    content-test-id="remote-resource-directory-dialog"
  >
    <div class="space-y-4">
      <div class="rounded-[var(--radius-l)] border border-border bg-subtle px-3 py-3">
        <div class="text-[11px] font-semibold uppercase tracking-[0.12em] text-text-tertiary">
          {{ t('resources.remoteBrowser.currentPath') }}
        </div>
        <div class="mt-1 break-all font-mono text-[13px] text-text-primary">
          {{ currentPath || t('resources.remoteBrowser.rootLabel') }}
        </div>
      </div>

      <UiStatusCallout v-if="error" tone="error" :description="error" />

      <div class="flex items-center justify-between gap-3">
        <UiButton
          type="button"
          variant="ghost"
          :disabled="loading || !parentPath"
          @click="loadRemoteDirectories(parentPath)"
        >
          <FolderUpIcon :size="14" />
          {{ t('resources.remoteBrowser.actions.goParent') }}
        </UiButton>

        <UiButton type="button" :disabled="loading || !currentPath" @click="chooseCurrentPath">
          {{ t('resources.remoteBrowser.actions.chooseCurrent') }}
        </UiButton>
      </div>

      <div v-if="loading" class="rounded-[var(--radius-l)] border border-border bg-surface px-3 py-4 text-sm text-text-secondary">
        {{ t('resources.remoteBrowser.loading') }}
      </div>

      <div v-else-if="entries.length" class="space-y-2">
        <button
          v-for="entry in entries"
          :key="entry.path"
          type="button"
          class="flex w-full items-center justify-between gap-3 rounded-[var(--radius-l)] border border-border bg-surface px-3 py-3 text-left transition-colors hover:border-border-strong hover:bg-subtle"
          @click="loadRemoteDirectories(entry.path)"
        >
          <div class="min-w-0">
            <div class="truncate text-sm font-semibold text-text-primary">
              {{ entry.name }}
            </div>
            <div class="truncate font-mono text-[12px] text-text-secondary">
              {{ entry.path }}
            </div>
          </div>
          <FolderSearchIcon :size="14" class="shrink-0 text-text-tertiary" />
        </button>
      </div>

      <UiEmptyState
        v-else
        :title="t('resources.remoteBrowser.empty.title')"
        :description="t('resources.remoteBrowser.empty.description')"
      />
    </div>
  </UiDialog>
</template>
