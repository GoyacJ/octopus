<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { ImagePlusIcon, RefreshCcwIcon } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

import type { AvatarUploadPayload } from '@octopus/schema'
import {
  UiButton,
  UiField,
  UiInput,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiStatusCallout,
  UiTextarea,
} from '@octopus/ui'

import { useWorkspaceProjectNotifications } from '@/composables/useWorkspaceProjectNotifications'
import * as tauriClient from '@/tauri/client'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const shell = useShellStore()
const notifications = useWorkspaceProjectNotifications()
const workspaceStore = useWorkspaceStore()

const workspace = computed(() => workspaceStore.activeWorkspace)
const isLoopbackWorkspace = computed(() => shell.activeWorkspaceConnection?.transportSecurity === 'loopback')
const workspaceName = ref('')
const mappedDirectory = ref('')
const pendingAvatarUpload = ref<AvatarUploadPayload | null>(null)
const pendingAvatarFileName = ref('')
const saving = ref(false)
const saveMessage = ref('')
const localError = ref('')

const avatarPreview = computed(() => {
  if (pendingAvatarUpload.value) {
    return `data:${pendingAvatarUpload.value.contentType};base64,${pendingAvatarUpload.value.dataBase64}`
  }

  return workspace.value?.avatar ?? ''
})

const avatarFallback = computed(() =>
  (workspaceName.value.trim() || workspace.value?.name || '?').slice(0, 1).toUpperCase(),
)

const avatarFileLabel = computed(() =>
  pendingAvatarFileName.value || t('workspaceSettings.avatar.currentLabel'),
)

const isDirty = computed(() => {
  const nameChanged = workspaceName.value.trim() !== (workspace.value?.name ?? '')
  const directoryChanged = isLoopbackWorkspace.value
    && mappedDirectory.value.trim() !== (workspace.value?.mappedDirectory ?? '')
  return nameChanged || directoryChanged || Boolean(pendingAvatarUpload.value)
})

const canSave = computed(() =>
  Boolean(workspace.value)
  && !saving.value
  && Boolean(workspaceName.value.trim())
  && (!isLoopbackWorkspace.value || Boolean(mappedDirectory.value.trim()))
  && isDirty.value,
)

watch(
  () => shell.activeWorkspaceConnectionId,
  async (connectionId) => {
    if (!connectionId) {
      return
    }
    await workspaceStore.ensureWorkspaceBootstrap(connectionId)
  },
  { immediate: true },
)

watch(
  () => workspace.value,
  (currentWorkspace) => {
    if (!currentWorkspace) {
      return
    }
    workspaceName.value = currentWorkspace.name
    mappedDirectory.value = currentWorkspace.mappedDirectory ?? ''
    pendingAvatarUpload.value = null
    pendingAvatarFileName.value = ''
    saveMessage.value = ''
    localError.value = ''
  },
  { immediate: true },
)

async function pickAvatar() {
  const picked = await tauriClient.pickAvatarImage()
  if (!picked) {
    return
  }

  pendingAvatarUpload.value = picked
  pendingAvatarFileName.value = picked.fileName
  saveMessage.value = ''
  localError.value = ''
}

async function pickMappedDirectory() {
  if (!isLoopbackWorkspace.value) {
    return
  }

  const selectedPath = await tauriClient.pickResourceDirectory()
  if (!selectedPath) {
    return
  }

  mappedDirectory.value = selectedPath
  saveMessage.value = ''
  localError.value = ''
}

function resetDraft() {
  if (!workspace.value) {
    return
  }

  workspaceName.value = workspace.value.name
  mappedDirectory.value = workspace.value.mappedDirectory ?? ''
  pendingAvatarUpload.value = null
  pendingAvatarFileName.value = ''
  saveMessage.value = ''
  localError.value = ''
}

async function saveWorkspaceSettings() {
  if (!canSave.value) {
    return
  }

  saving.value = true
  saveMessage.value = ''
  localError.value = ''

  try {
    const updated = await workspaceStore.updateWorkspace({
      name: workspaceName.value.trim(),
      ...(pendingAvatarUpload.value ? { avatar: pendingAvatarUpload.value } : {}),
      ...(isLoopbackWorkspace.value ? { mappedDirectory: mappedDirectory.value.trim() } : {}),
    })

    if (!updated) {
      localError.value = workspaceStore.error || t('workspaceSettings.feedback.saveError')
      return
    }

    pendingAvatarUpload.value = null
    pendingAvatarFileName.value = ''
    workspaceName.value = updated.name
    mappedDirectory.value = updated.mappedDirectory ?? ''
    saveMessage.value = t('workspaceSettings.feedback.saved')
    await notifications.notifyWorkspaceSettingsSaved(updated.name)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <UiPageShell width="standard" test-id="workspace-settings-view">
    <UiPageHeader
      :eyebrow="t('workspaceSettings.header.eyebrow')"
      :title="t('workspaceSettings.header.title')"
      :description="t('workspaceSettings.header.description')"
    >
      <template #actions>
        <UiButton
          type="button"
          variant="ghost"
          :disabled="saving || !isDirty"
          data-testid="workspace-settings-reset"
          @click="resetDraft"
        >
          <RefreshCcwIcon :size="14" />
          {{ t('common.reset') }}
        </UiButton>
        <UiButton
          type="button"
          :loading="saving"
          :disabled="!canSave"
          data-testid="workspace-settings-save"
          @click="saveWorkspaceSettings"
        >
          {{ t('common.save') }}
        </UiButton>
      </template>
    </UiPageHeader>

    <UiStatusCallout
      v-if="saveMessage"
      tone="success"
      :title="t('workspaceSettings.feedback.savedTitle')"
      :description="saveMessage"
    />

    <UiStatusCallout
      v-if="localError"
      tone="error"
      :title="t('workspaceSettings.feedback.errorTitle')"
      :description="localError"
    />

    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="t('workspaceSettings.general.title')"
      :subtitle="t('workspaceSettings.general.subtitle')"
    >
      <div class="grid gap-6 lg:grid-cols-[220px_minmax(0,1fr)]">
        <div class="space-y-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">
            {{ t('workspaceSettings.avatar.label') }}
          </div>
          <div
            class="flex h-28 w-28 items-center justify-center overflow-hidden rounded-[var(--radius-2xl)] border border-border bg-subtle text-2xl font-semibold text-text-secondary"
            data-testid="workspace-settings-avatar-preview"
          >
            <img v-if="avatarPreview" :src="avatarPreview" alt="" class="h-full w-full object-cover">
            <span v-else>{{ avatarFallback }}</span>
          </div>
          <UiButton
            type="button"
            variant="outline"
            class="w-full justify-center"
            data-testid="workspace-settings-avatar-pick"
            @click="pickAvatar"
          >
            <ImagePlusIcon :size="14" />
            {{ t('workspaceSettings.avatar.actions.pick') }}
          </UiButton>
          <div
            class="rounded-[var(--radius-l)] border border-border bg-subtle px-3 py-2 text-xs text-text-secondary"
            data-testid="workspace-settings-avatar-file-label"
          >
            {{ avatarFileLabel }}
          </div>
        </div>

        <div class="space-y-5">
          <UiField
            :label="t('workspaceSettings.fields.name')"
            :hint="t('workspaceSettings.fields.nameHint')"
          >
            <UiInput
              v-model="workspaceName"
              data-testid="workspace-settings-name-input"
              :placeholder="t('workspaceSettings.placeholders.name')"
            />
          </UiField>

          <UiField
            :label="t('workspaceSettings.fields.mappedDirectory')"
            :hint="t(isLoopbackWorkspace ? 'workspaceSettings.fields.mappedDirectoryHint' : 'workspaceSettings.fields.mappedDirectoryReadonlyHint')"
          >
            <div class="flex items-stretch gap-3">
              <UiInput
                v-model="mappedDirectory"
                readonly
                class="flex-1 font-mono"
                data-testid="workspace-settings-directory-input"
                :placeholder="t('workspaceSettings.placeholders.mappedDirectory')"
              />
              <UiButton
                type="button"
                variant="outline"
                :disabled="!isLoopbackWorkspace"
                data-testid="workspace-settings-directory-pick"
                @click="pickMappedDirectory"
              >
                {{ t('workspaceSettings.fields.pickMappedDirectory') }}
              </UiButton>
            </div>
          </UiField>

          <UiField :label="t('workspaceSettings.fields.mappedDirectoryDefault')">
            <UiTextarea
              :model-value="workspace?.mappedDirectoryDefault ?? ''"
              :rows="2"
              readonly
              data-testid="workspace-settings-directory-default"
              class="font-mono"
            />
          </UiField>
        </div>
      </div>
    </UiPanelFrame>
  </UiPageShell>
</template>
