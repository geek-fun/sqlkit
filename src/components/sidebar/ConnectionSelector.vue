<script setup lang="ts">
import type { ComboboxOption } from '@/components/ui/combobox'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import DbTypeIcon from '@/components/database-browser/DbTypeIcon.vue'
import { SearchableSelect } from '@/components/ui/combobox'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { ConnectionStatus, DatabaseType, useConnectionStore } from '@/store'

type Props = {
  modelValue: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | null): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()

const options = computed<ComboboxOption[]>(() => {
  const seen = new Set<string>()
  return connectionStore.connections
    .filter((conn) => {
      if (!conn.id || seen.has(conn.name))
        return false
      seen.add(conn.name)
      return true
    })
    .map(conn => ({
      label: conn.name,
      value: conn.id!,
    }))
})

const selectedConnection = computed(() =>
  props.modelValue ? connectionStore.getConnectionById(props.modelValue) : null,
)

const status = computed(() => {
  if (!props.modelValue)
    return ConnectionStatus.DISCONNECTED
  return connectionStore.getConnectionStatus(props.modelValue)
})

type StatusInfo = { dotClass: string, tooltipKey: string }

const statusInfo = computed<StatusInfo>(() => {
  switch (status.value) {
    case ConnectionStatus.CONNECTED:
      return { dotClass: 'i-carbon-circle-filled text-green-500', tooltipKey: 'sidebar.connection.connected' }
    case ConnectionStatus.ERROR:
      return { dotClass: 'i-carbon-circle-filled text-amber-500', tooltipKey: 'sidebar.connection.lost' }
    case ConnectionStatus.CONNECTING:
      return { dotClass: 'i-carbon-circle-filled text-amber-500 animate-pulse', tooltipKey: 'sidebar.connection.connecting' }
    default:
      return { dotClass: 'i-carbon-circle-filled text-muted-foreground/40', tooltipKey: 'sidebar.connection.disconnected' }
  }
})

function handleSelect(value: string) {
  emit('update:modelValue', value || null)
}
</script>

<template>
  <div class="p-2 border-b flex gap-1.5 items-center">
    <SearchableSelect
      :model-value="props.modelValue || ''"
      :options="options"
      :search-threshold="1"
      :placeholder="t('sidebar.connection.placeholder')"
      :search-placeholder="t('sidebar.search')"
      class="text-xs flex-1 h-8"
      @update:model-value="handleSelect"
    >
      <template #selected-prepend>
        <DbTypeIcon v-if="selectedConnection" :type="selectedConnection.type" :size="14" class="mr-1.5 shrink-0" />
      </template>
      <template #option="{ option }">
        <div class="flex gap-1.5 w-full items-center">
          <DbTypeIcon :type="connectionStore.getConnectionById(option.value)?.type ?? DatabaseType.POSTGRESQL" :size="14" class="shrink-0" />
          <span class="text-left flex-1 truncate">{{ option.label }}</span>
          <span
            v-if="connectionStore.getConnectionStatus(option.value) === ConnectionStatus.CONNECTED"
            class="text-[10px] text-green-600 font-medium px-1 rounded bg-green-500/10 shrink-0 uppercase"
          >
            {{ t('sidebar.connection.connected') }}
          </span>
        </div>
      </template>
    </SearchableSelect>
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger as-child>
          <span class="shrink-0 h-3.5 w-3.5 cursor-help" :class="[statusInfo.dotClass]" />
        </TooltipTrigger>
        <TooltipContent side="right">
          {{ t(statusInfo.tooltipKey) }}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  </div>
</template>
