<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import DbTypeIcon from '@/components/database-browser/DbTypeIcon.vue'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { ConnectionStatus, useConnectionStore } from '@/store'

type Props = {
  modelValue: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | null): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()

const connections = computed(() => connectionStore.connections)

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
    <Select :model-value="props.modelValue || undefined" @update:model-value="handleSelect">
      <SelectTrigger class="text-xs flex-1 h-8 min-w-0">
        <SelectValue :placeholder="t('sidebar.connection.placeholder')" class="truncate">
          <template v-if="selectedConnection">
            <div class="flex gap-1.5 items-center">
              <DbTypeIcon :type="selectedConnection.type" :size="14" />
              <span class="truncate">{{ selectedConnection.name }}</span>
            </div>
          </template>
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectItem
          v-for="conn in connections"
          :key="conn.id!"
          :value="conn.id!"
          class="text-xs"
        >
          <div class="flex gap-1.5 items-center">
            <DbTypeIcon :type="conn.type" :size="14" />
            <span class="flex-1 truncate">{{ conn.name }}</span>
            <span
              v-if="connectionStore.getConnectionStatus(conn.id!) === ConnectionStatus.CONNECTED"
              class="text-[10px] text-green-600 font-medium px-1 rounded bg-green-500/10 uppercase"
            >
              {{ t('sidebar.connection.connected') }}
            </span>
          </div>
        </SelectItem>
      </SelectContent>
    </Select>
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
