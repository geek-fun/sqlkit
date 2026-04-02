<script setup lang="ts">
import type { ServerConnection } from '@/store'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { ConnectionStatus, DatabaseType } from '@/store'

const props = defineProps<{
  connection: ServerConnection
  connectionStatus: ConnectionStatus
}>()

const emit = defineEmits<{
  (e: 'connect', connection: ServerConnection): void
  (e: 'dblclick', connection: ServerConnection): void
  (e: 'edit', connection: ServerConnection): void
  (e: 'delete', connection: ServerConnection): void
  (e: 'duplicate', connection: ServerConnection): void
}>()

const { t } = useI18n()
const { getDatabaseIcon, getDatabaseColor } = useDatabaseIcon()

const statusColor = computed(() => {
  switch (props.connectionStatus) {
    case ConnectionStatus.CONNECTED:
      return 'bg-green-600 dark:bg-green-500'
    case ConnectionStatus.CONNECTING:
      return 'bg-yellow-600 dark:bg-yellow-500 animate-pulse'
    case ConnectionStatus.ERROR:
      return 'bg-red-600 dark:bg-red-500'
    default:
      return 'bg-muted-foreground/50'
  }
})

const statusText = computed(() => {
  switch (props.connectionStatus) {
    case ConnectionStatus.CONNECTED:
      return t('common.status.connected')
    case ConnectionStatus.CONNECTING:
      return t('common.status.connecting')
    case ConnectionStatus.ERROR:
      return t('common.status.error')
    default:
      return t('common.status.disconnected')
  }
})

const connectionUrl = computed(() => {
  const { host, port, database, type } = props.connection
  if (type === DatabaseType.SQLITE) {
    return host
  }
  const portStr = port ? `:${port}` : ''
  const dbStr = database ? `/${database}` : ''
  return `${host}${portStr}${dbStr}`
})

const handleConnect = () => emit('connect', props.connection)
const handleDoubleClick = () => emit('dblclick', props.connection)
const handleEdit = () => emit('edit', props.connection)
const handleDelete = () => emit('delete', props.connection)
const handleDuplicate = () => emit('duplicate', props.connection)
</script>

<template>
  <Card class="cursor-pointer transition-shadow relative overflow-hidden hover:shadow-md" @dblclick="handleDoubleClick">
    <!-- Status indicator dot -->
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger as-child>
          <div
            class="rounded-full h-2.5 w-2.5 right-4 top-4 absolute"
            :class="statusColor"
          />
        </TooltipTrigger>
        <TooltipContent>
          <p>{{ statusText }}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>

    <div class="p-4 space-y-3">
      <!-- Database type icon -->
      <div class="flex items-start justify-between">
        <div
          class="p-1.5 rounded-lg flex h-10 w-10 items-center justify-center"
          :class="getDatabaseColor(connection.type)"
        >
          <img
            :src="getDatabaseIcon(connection.type)"
            :alt="connection.type"
            class="h-full w-full object-contain"
          >
        </div>
      </div>

      <!-- Connection name -->
      <div>
        <h3 class="text-lg leading-tight font-semibold truncate">
          {{ connection.name }}
        </h3>
        <p class="text-sm text-muted-foreground mt-1 truncate" :title="connectionUrl">
          {{ connectionUrl }}
        </p>
      </div>

      <!-- Tags -->
      <div class="flex flex-wrap gap-1.5">
        <Badge variant="outline" class="text-xs">
          {{ connection.type }}
        </Badge>
        <Badge v-if="connection.ssl" variant="outline" class="text-xs">
          SSL
        </Badge>
      </div>

      <!-- Actions -->
      <div class="pt-2 border-t flex gap-2 items-center justify-end">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="icon" class="h-8 w-8" @click="handleConnect">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="h-4 w-4"
                >
                  <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" />
                  <polyline points="10 17 15 12 10 7" />
                  <line x1="15" x2="3" y1="12" y2="12" />
                </svg>
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>{{ t('components.serverCard.actions.connect') }}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>

        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button variant="ghost" size="icon" class="h-8 w-8">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="h-4 w-4"
              >
                <circle cx="12" cy="12" r="1" />
                <circle cx="19" cy="12" r="1" />
                <circle cx="5" cy="12" r="1" />
              </svg>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem @click="handleEdit">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="mr-2 h-4 w-4"
              >
                <path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" />
                <path d="m15 5 4 4" />
              </svg>
              {{ t('components.serverCard.actions.edit') }}
            </DropdownMenuItem>
            <DropdownMenuItem @click="handleDuplicate">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="mr-2 h-4 w-4"
              >
                <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
                <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
              </svg>
              {{ t('components.serverCard.actions.duplicate') }}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem class="text-destructive focus:text-destructive" @click="handleDelete">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="mr-2 h-4 w-4"
              >
                <path d="M3 6h18" />
                <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                <line x1="10" x2="10" y1="11" y2="17" />
                <line x1="14" x2="14" y1="11" y2="17" />
              </svg>
              {{ t('components.serverCard.actions.delete') }}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  </Card>
</template>
