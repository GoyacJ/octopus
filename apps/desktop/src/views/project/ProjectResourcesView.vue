<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { FileIcon, FolderIcon, MoreVerticalIcon, Trash2Icon, PowerOffIcon, UploadIcon } from 'lucide-vue-next'

import { UiBadge, UiButton, UiEmptyState, UiInput, UiListRow, UiSectionHeading } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useResourceStore } from '@/stores/resource'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'
import type { ProjectResourceKind } from '@octopus/schema'

const { t } = useI18n()
const route = useRoute()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const resourceStore = useResourceStore()
const searchQuery = ref('')
const activeActionMenu = ref<string | null>(null)

async function loadResources() {
  const projectId = typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId
  if (!projectId) {
    return
  }
  await resourceStore.loadProjectResources(projectId)
}

watch(
  () => [shell.activeWorkspaceConnectionId, route.params.projectId],
  ([connectionId]) => {
    if (typeof connectionId === 'string' && connectionId) {
      void loadResources()
    }
  },
  { immediate: true },
)

const filteredResources = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return resourceStore.activeProjectResources.filter((resource) => {
    if (!query) {
      return true
    }

    return [
      resource.name,
      resource.location ?? '',
      resource.kind,
      resource.origin,
      ...resource.tags,
    ].join(' ').toLowerCase().includes(query)
  })
})

function toggleActionMenu(resourceId: string) {
  activeActionMenu.value = activeActionMenu.value === resourceId ? null : resourceId
}

async function handleDelete(resourceId: string) {
  const projectId = workspaceStore.currentProjectId
  if (!projectId) return
  if (!confirm(t('resources.actions.confirmDelete'))) return
  activeActionMenu.value = null
  await resourceStore.deleteProjectResource(projectId, resourceId)
}

async function handleDeactivate(resourceId: string) {
  const projectId = workspaceStore.currentProjectId
  if (!projectId) return
  activeActionMenu.value = null
  await resourceStore.updateProjectResource(projectId, resourceId, { status: 'attention' })
}

async function handleUploadFile() {
  const projectId = workspaceStore.currentProjectId
  if (!projectId) return
  const input = document.createElement('input')
  input.type = 'file'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = async () => {
      const base64 = (reader.result as string).split(',')[1]
      await resourceStore.createProjectResource(projectId, {
        projectId,
        kind: 'file' as ProjectResourceKind,
        name: file.name,
        tags: [],
      })
    }
    reader.readAsDataURL(file)
  }
  input.click()
}

async function handleUploadFolder() {
  const projectId = workspaceStore.currentProjectId
  if (!projectId) return
  const input = document.createElement('input')
  input.type = 'file'
  input.webkitdirectory = true
  input.onchange = async () => {
    const files = input.files
    if (!files?.length) return
    const entries: { fileName: string; contentType: string; dataBase64: string; byteSize: number; relativePath: string }[] = []
    for (const file of Array.from(files)) {
      const reader = new FileReader()
      const base64 = await new Promise<string>((resolve) => {
        reader.onload = () => resolve((reader.result as string).split(',')[1])
        reader.readAsDataURL(file)
      })
      entries.push({
        fileName: file.name,
        contentType: file.type || 'application/octet-stream',
        dataBase64: base64,
        byteSize: file.size,
        relativePath: file.webkitRelativePath || file.name,
      })
    }
    await resourceStore.createProjectResourceFolder(projectId, {
      projectId,
      files: entries,
    })
  }
  input.click()
}
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="space-y-4 px-2">
      <UiSectionHeading
        :eyebrow="t('resources.header.eyebrow')"
        :title="workspaceStore.activeProject?.name ?? t('resources.header.projectTitleFallback')"
        :subtitle="resourceStore.error || workspaceStore.activeProject?.description || t('resources.header.subtitle')"
      />
      <div class="flex items-center gap-3">
        <UiInput v-model="searchQuery" :placeholder="t('resources.filters.searchPlaceholder')" class="max-w-md" />
        <div class="flex gap-2">
          <UiButton variant="outline" size="sm" @click="handleUploadFile">
            <FileIcon :size="14" />
            {{ t('resources.actions.uploadFile') }}
          </UiButton>
          <UiButton variant="outline" size="sm" @click="handleUploadFolder">
            <FolderIcon :size="14" />
            {{ t('resources.actions.uploadFolder') }}
          </UiButton>
        </div>
      </div>
    </header>

    <main class="px-2">
      <div v-if="filteredResources.length" class="space-y-2">
        <UiListRow
          v-for="resource in filteredResources"
          :key="resource.id"
          :title="resource.name"
          :subtitle="resource.location || resource.origin"
        >
          <template #meta>
            <UiBadge :label="resource.kind" subtle />
            <UiBadge :label="resource.origin" subtle />
            <UiBadge v-if="resource.status !== 'healthy'" :label="resource.status" subtle />
            <span class="text-xs text-text-tertiary">{{ formatDateTime(resource.updatedAt) }}</span>
          </template>
          <template #actions>
            <UiButton
              variant="ghost"
              size="icon"
              class="h-7 w-7"
              @click="toggleActionMenu(resource.id)"
            >
              <MoreVerticalIcon :size="14" />
            </UiButton>
            <div
              v-if="activeActionMenu === resource.id"
              class="absolute right-2 top-8 z-50 flex flex-col gap-1 rounded-md border border-border/40 bg-background p-1 shadow-lg"
            >
              <UiButton
                v-if="resource.status === 'healthy'"
                variant="ghost"
                size="sm"
                class="w-full justify-start text-xs"
                @click="handleDeactivate(resource.id)"
              >
                <PowerOffIcon :size="12" />
                {{ t('resources.actions.deactivate') }}
              </UiButton>
              <UiButton
                variant="ghost"
                size="sm"
                class="w-full justify-start text-xs text-destructive"
                @click="handleDelete(resource.id)"
              >
                <Trash2Icon :size="12" />
                {{ t('resources.actions.delete') }}
              </UiButton>
            </div>
          </template>
        </UiListRow>
      </div>
      <UiEmptyState v-else :title="t('resources.empty.title')" :description="t('resources.empty.description')" />
    </main>
  </div>
</template>
