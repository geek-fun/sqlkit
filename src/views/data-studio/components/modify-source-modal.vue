<script setup lang="ts">
import type { DatabaseType } from '@/store/connectionStore'
import type { DataSourcePermissions, SourcePermissionsMode } from '@/store/dataStudioStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { useDataStudioStore } from '@/store/dataStudioStore'

const props = defineProps<{
  open: boolean
  sourceIdx: number
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const { t } = useI18n()
const dataStudioStore = useDataStudioStore()
const { getDatabaseIcon } = useDatabaseIcon()

const currentSource = computed(() => {
  const session = dataStudioStore.activeSession
  const activeSources = session?.sources.filter(s => !s.detached) ?? []
  return activeSources[props.sourceIdx] ?? null
})

const sessionPermissionsMode = computed(
  () => dataStudioStore.activeSession?.permissionsMode ?? 'Ask',
)

const permissionKeys: (keyof DataSourcePermissions)[] = ['read', 'create', 'update', 'delete']

const localMode = ref<SourcePermissionsMode>('inherit')
const localPermissions = ref<DataSourcePermissions>({
  read: true,
  create: false,
  update: false,
  delete: false,
})
const detachConfirming = ref(false)

const activeInheritedPerms = computed<(keyof DataSourcePermissions)[]>(() => {
  const write = sessionPermissionsMode.value === 'Auto'
  return permissionKeys.filter(p => p === 'read' || write)
})

const permColors: Record<string, string> = {
  read: 'text-primary',
  create: 'text-primary',
  update: 'text-amber-600 dark:text-amber-500',
  delete: 'text-red-600 dark:text-red-500',
}

const permTagColors: Record<string, string> = {
  read: 'bg-primary/10 text-primary',
  create: 'bg-primary/10 text-primary',
  update: 'bg-amber-500/10 text-amber-600 dark:text-amber-500',
  delete: 'bg-red-500/10 text-red-600 dark:text-red-500',
}

watch(() => props.open, (open) => {
  if (open && currentSource.value) {
    localMode.value = currentSource.value.permissionsMode ?? 'inherit'
    localPermissions.value = { ...currentSource.value.permissions }
    detachConfirming.value = false
  }
})

function togglePermission(perm: keyof DataSourcePermissions) {
  if (perm === 'read')
    return
  localPermissions.value = { ...localPermissions.value, [perm]: !localPermissions.value[perm] }
}

function handleSave() {
  const session = dataStudioStore.activeSession
  const source = currentSource.value
  if (!session || !source)
    return

  if (localMode.value === 'inherit') {
    dataStudioStore.updateSessionSourceMode(source.sourceId, 'inherit')
  }
  else {
    dataStudioStore.updateSessionSourcePermissions(source.sourceId, {
      ...localPermissions.value,
      read: true,
    })
  }
  emit('update:open', false)
}

function handleDetach() {
  const session = dataStudioStore.activeSession
  const source = currentSource.value
  if (!session || !source)
    return
  dataStudioStore.detachSourceFromSession(source.sourceId)
  emit('update:open', false)
}
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-[480px]">
      <DialogHeader>
        <div class="flex gap-3 items-center">
          <div
            class="border border-border rounded-xl bg-muted flex shrink-0 h-10 w-10 items-center justify-center"
          >
            <img
              v-if="currentSource"
              :src="getDatabaseIcon(currentSource.databaseType as DatabaseType)"
              class="h-5 w-5 object-contain"
              :alt="currentSource.databaseType"
            >
          </div>
          <div>
            <DialogTitle>{{ t('dataStudio.modifySource.title') }}</DialogTitle>
            <p class="text-sm text-muted-foreground mt-0.5">
              {{ currentSource?.alias }}
            </p>
          </div>
        </div>
      </DialogHeader>

      <div class="py-2 flex flex-col gap-3 min-h-[180px]">
        <p class="text-xs text-muted-foreground tracking-wide font-medium px-0.5 uppercase">
          {{ t('dataStudio.modifySource.accessPermissions') }}
        </p>

        <!-- Inherit card -->
        <button
          type="button"
          class="p-4 text-left border-2 rounded-xl w-full transition-colors focus:outline-none"
          :class="
            localMode === 'inherit'
              ? 'border-primary bg-primary/5'
              : 'border-border hover:border-border/80 hover:bg-muted/30'
          "
          @click="localMode = 'inherit'"
        >
          <div class="flex gap-3 items-start">
            <div
              class="mt-0.5 border-2 rounded-full flex shrink-0 h-4 w-4 transition-colors items-center justify-center"
              :class="localMode === 'inherit' ? 'border-primary' : 'border-muted-foreground/40'"
            >
              <div v-if="localMode === 'inherit'" class="rounded-full bg-primary h-2 w-2" />
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex gap-2 items-center">
                <span class="text-sm font-medium">
                  {{ t('dataStudio.modifySource.inheritTitle') }}
                </span>
                <span
                  class="text-xs text-muted-foreground font-medium px-1.5 py-0.5 rounded-md bg-muted inline-flex gap-1 items-center"
                >
                  <span
                    class="h-3 w-3"
                    :class="
                      sessionPermissionsMode === 'Auto'
                        ? 'i-carbon-flash text-amber-500'
                        : 'i-carbon-user-activity text-blue-500'
                    "
                  />
                  {{ sessionPermissionsMode }}
                </span>
              </div>
              <p class="text-xs text-muted-foreground mt-0.5">
                {{ t('dataStudio.modifySource.inheritDesc') }}
              </p>
              <div v-if="localMode === 'inherit'" class="mt-2.5 flex flex-wrap gap-1.5">
                <span
                  v-for="perm in activeInheritedPerms"
                  :key="perm"
                  class="text-xs font-medium px-2 py-0.5 rounded-md inline-flex gap-1 items-center"
                  :class="permTagColors[perm]"
                >
                  <span class="i-carbon-checkmark h-3.5 w-3.5" />
                  {{ t(`dataStudio.modifySource.${perm}`) }}
                </span>
              </div>
            </div>
          </div>
        </button>

        <!-- Custom card -->
        <button
          type="button"
          class="p-4 text-left border-2 rounded-xl w-full transition-colors focus:outline-none"
          :class="
            localMode === 'custom'
              ? 'border-primary bg-primary/5'
              : 'border-border hover:border-border/80 hover:bg-muted/30'
          "
          @click="localMode = 'custom'"
        >
          <div class="flex gap-3 items-start">
            <div
              class="mt-0.5 border-2 rounded-full flex shrink-0 h-4 w-4 transition-colors items-center justify-center"
              :class="localMode === 'custom' ? 'border-primary' : 'border-muted-foreground/40'"
            >
              <div v-if="localMode === 'custom'" class="rounded-full bg-primary h-2 w-2" />
            </div>
            <div class="flex-1 min-w-0">
              <span class="text-sm font-medium">
                {{ t('dataStudio.modifySource.customTitle') }}
              </span>
              <p class="text-xs text-muted-foreground mt-0.5">
                {{ t('dataStudio.modifySource.customDesc') }}
              </p>

              <div v-if="localMode === 'custom'" class="mt-3" @click.stop>
                <div class="gap-1 grid grid-cols-2">
                  <label
                    v-for="perm in permissionKeys"
                    :key="perm"
                    class="px-1 py-1.5 rounded-lg flex gap-3 select-none items-center"
                    :class="[
                      perm === 'read'
                        ? 'opacity-60 cursor-not-allowed'
                        : 'cursor-pointer hover:bg-muted/50',
                      permColors[perm],
                    ]"
                  >
                    <input
                      type="checkbox"
                      :checked="localPermissions[perm]"
                      :disabled="perm === 'read'"
                      class="accent-primary h-4 w-4"
                      @change="togglePermission(perm)"
                    >
                    <span class="text-sm">{{ t(`dataStudio.modifySource.${perm}`) }}</span>
                  </label>
                </div>
              </div>
            </div>
          </div>
        </button>
      </div>

      <DialogFooter class="flex items-center justify-between sm:justify-between">
        <div class="flex flex-col gap-1">
          <div v-if="detachConfirming">
            <span class="text-xs text-muted-foreground">
              {{ t('dataStudio.detachSource.message') }}
            </span>
            <div class="mt-1.5 flex gap-1.5">
              <Button variant="destructive" size="sm" @click="handleDetach">
                <span class="i-carbon-unlink mr-1 h-3.5 w-3.5" />
                {{ t('dataStudio.detachSource.confirm') }}
              </Button>
              <Button variant="outline" size="sm" @click="detachConfirming = false">
                {{ t('common.buttons.cancel') }}
              </Button>
            </div>
          </div>
          <Button v-else variant="ghost" class="text-destructive" @click="detachConfirming = true">
            <span class="i-carbon-unlink mr-1.5 h-3.5 w-3.5" />
            {{ t('dataStudio.detachSource.title') }}
          </Button>
        </div>
        <div class="flex gap-2">
          <Button variant="outline" @click="emit('update:open', false)">
            {{ t('common.buttons.cancel') }}
          </Button>
          <Button @click="handleSave">
            {{ t('dataStudio.modifySource.saveChanges') }}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
