<script setup lang="ts">
import type { DbCapabilities } from './dbCapabilities'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { DatabaseType, useConnectionStore, useDatabaseStore } from '@/store'
import { getDbCapabilities } from './dbCapabilities'

type Props = {
  connectionId: string | null
  modelValue: string | null
  loading?: boolean
}

type ActionKind
  = | 'newDatabase'
    | 'newSchema'
    | 'newTable'
    | 'newView'
    | 'newFunction'
    | 'newProcedure'
    | 'backupDatabase'
    | 'exportDatabase'
    | 'dropDatabase'

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | null): void
  (e: 'refresh'): void
  (e: 'action', kind: ActionKind): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()

const activeConnection = computed(() =>
  props.connectionId ? connectionStore.getConnectionById(props.connectionId) : null,
)

const userDatabases = computed(() => databaseStore.userDatabases)
const systemDatabases = computed(() => databaseStore.systemDatabases)

const capabilities = computed<DbCapabilities>(() => {
  if (!activeConnection.value)
    return getDbCapabilities(DatabaseType.POSTGRESQL)
  return getDbCapabilities(activeConnection.value.type)
})

function handleSelect(value: string) {
  emit('update:modelValue', value === '' ? null : value)
}

function handleAction(kind: ActionKind) {
  emit('action', kind)
}
</script>

<template>
  <div v-if="props.connectionId" class="px-2 py-1.5 border-b flex gap-1 items-center">
    <Select :model-value="props.modelValue ?? ''" class="flex-1" @update:model-value="handleSelect">
      <SelectTrigger class="text-xs h-8 min-w-0">
        <SelectValue :placeholder="t('sidebar.database.placeholder')">
          <template v-if="props.modelValue">
            <div class="flex gap-1.5 items-center">
              <span class="i-carbon-data-base text-yellow-500 shrink-0 h-3.5 w-3.5" />
              <span class="truncate">{{ props.modelValue }}</span>
            </div>
          </template>
          <template v-else>
            <div class="flex gap-1.5 items-center">
              <span class="i-carbon-data-base text-yellow-500 shrink-0 h-3.5 w-3.5" />
              <span class="truncate">{{ t('sidebar.database.placeholder') }}</span>
            </div>
          </template>
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="">
          {{ t('sidebar.database.placeholder') }}
        </SelectItem>
        <SelectSeparator v-if="userDatabases.length > 0" />
        <SelectGroup v-if="userDatabases.length > 0">
          <SelectLabel class="text-xs text-muted-foreground">
            {{ t('sidebar.database.selectDatabase') }}
          </SelectLabel>
          <SelectItem v-for="db in userDatabases" :key="db.name" :value="db.name" class="text-xs">
            <div class="flex gap-1.5 items-center">
              <span class="i-carbon-data-base text-yellow-500 shrink-0 h-3.5 w-3.5" />
              <span class="truncate">{{ db.name }}</span>
            </div>
          </SelectItem>
        </SelectGroup>
        <SelectGroup v-if="systemDatabases.length > 0">
          <SelectLabel class="text-xs text-muted-foreground">
            {{ t('components.databaseBrowser.systemDatabases') }}
          </SelectLabel>
          <SelectItem v-for="db in systemDatabases" :key="db.name" :value="db.name" class="text-xs">
            {{ db.name }}
          </SelectItem>
        </SelectGroup>
      </SelectContent>
    </Select>

    <Button
      variant="ghost"
      size="icon"
      class="shrink-0 h-7 w-7"
      :disabled="props.loading"
      :title="t('sidebar.database.refreshTooltip')"
      @click="emit('refresh')"
    >
      <span class="i-carbon-renew h-3.5 w-3.5" :class="{ 'animate-spin': props.loading }" />
    </Button>

    <DropdownMenu>
      <DropdownMenuTrigger as-child>
        <Button variant="ghost" size="icon" class="shrink-0 h-7 w-7" :title="t('sidebar.actionMenu.label')">
          <span class="i-carbon-overflow-menu-horizontal h-3.5 w-3.5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <div class="text-xs font-semibold px-2 py-1.5">
          {{ t('sidebar.actionMenu.label') }}
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem v-if="capabilities.newDatabase" @click="handleAction('newDatabase')">
          <span class="i-carbon-data-base mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.newDatabase') }}
        </DropdownMenuItem>
        <DropdownMenuItem v-if="capabilities.newSchema" @click="handleAction('newSchema')">
          <span class="i-carbon-folder-open mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.newSchema') }}
        </DropdownMenuItem>
        <DropdownMenuItem @click="handleAction('newTable')">
          <span class="i-carbon-table mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.newTable') }}
        </DropdownMenuItem>
        <DropdownMenuItem v-if="capabilities.views" @click="handleAction('newView')">
          <span class="i-carbon-view mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.newView') }}
        </DropdownMenuItem>
        <DropdownMenuItem v-if="capabilities.functions" @click="handleAction('newFunction')">
          <span class="i-carbon-function-math mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.newFunction') }}
        </DropdownMenuItem>
        <DropdownMenuItem v-if="capabilities.procedures" @click="handleAction('newProcedure')">
          <span class="i-carbon-document mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.newProcedure') }}
        </DropdownMenuItem>
        <DropdownMenuSeparator v-if="capabilities.backup || capabilities.export" />
        <DropdownMenuItem v-if="capabilities.backup" @click="handleAction('backupDatabase')">
          <span class="i-carbon-download mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.backupDatabase') }}
        </DropdownMenuItem>
        <DropdownMenuItem v-if="capabilities.export" @click="handleAction('exportDatabase')">
          <span class="i-carbon-export mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.exportDatabase') }}
        </DropdownMenuItem>
        <DropdownMenuSeparator v-if="capabilities.dropDatabase" />
        <DropdownMenuItem
          v-if="capabilities.dropDatabase"
          class="text-destructive focus:text-destructive"
          @click="handleAction('dropDatabase')"
        >
          <span class="i-carbon-trash-can mr-2 h-3.5 w-3.5" /> {{ t('sidebar.actionMenu.dropDatabase') }}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>
</template>
