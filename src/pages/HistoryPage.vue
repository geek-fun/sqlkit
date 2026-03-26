<script setup lang="ts">
import type { HistoryEntry, HistoryEntryStatus } from '@/store'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import AppLayout from '@/components/layout/AppLayout.vue'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { toast } from '@/composables/useNotifications'
import { useConnectionStore, useHistoryStore, useTabStore } from '@/store'

const { t } = useI18n()
const router = useRouter()
const historyStore = useHistoryStore()
const tabStore = useTabStore()
const connectionStore = useConnectionStore()

const searchQuery = ref('')
const statusFilter = ref<HistoryEntryStatus | 'all'>('all')

type ClearMode = 'all' | 'nonFavorites'
const clearDialogOpen = ref(false)
const clearMode = ref<ClearMode>('all')

const displayedEntries = computed(() =>
  historyStore.filteredEntries(
    searchQuery.value,
    statusFilter.value === 'all' ? '' : statusFilter.value,
  ),
)

const clearDialogMessage = computed(() =>
  clearMode.value === 'all'
    ? t('pages.history.clearAllConfirm')
    : t('pages.history.clearNonFavoritesConfirm'),
)

const formatTimestamp = (timestamp: number): string =>
  new Date(timestamp).toLocaleString()

const formatDuration = (ms?: number): string => {
  if (ms === undefined)
    return '-'
  if (ms < 1000)
    return `${ms}ms`
  return `${(ms / 1000).toFixed(2)}s`
}

const truncateSql = (sql: string, maxLength = 80): string => {
  const single = sql.replace(/\s+/g, ' ').trim()
  return single.length > maxLength ? `${single.slice(0, maxLength)}…` : single
}

const openClearDialog = (mode: ClearMode) => {
  clearMode.value = mode
  clearDialogOpen.value = true
}

const confirmClear = () => {
  if (clearMode.value === 'all') {
    historyStore.clearAll()
  }
  else {
    historyStore.clearNonFavorites()
  }
  clearDialogOpen.value = false
  toast.success(t('pages.history.notifications.cleared'))
}

const handleRerun = async (entry: HistoryEntry) => {
  let connectionId = entry.connectionId
  if (!connectionStore.getConnectionById(connectionId)) {
    connectionId = connectionStore.activeConnectionId || ''
  }
  if (!connectionId) {
    await router.push('/queries')
    return
  }
  const database = entry.database
  const tab = tabStore.createTab(connectionId, database)
  tabStore.updateTabContent(tab.id, entry.sql)
  const createdTab = tabStore.tabs.find(t => t.id === tab.id)
  if (createdTab)
    createdTab.hasUnsavedChanges = false
  await router.push('/queries')
}

const handleDelete = (id: string) => {
  historyStore.deleteEntry(id)
  toast.success(t('pages.history.notifications.deleted'))
}

const handleToggleFavorite = (id: string) => {
  historyStore.toggleFavorite(id)
}
</script>

<template>
  <AppLayout>
    <div class="p-6 flex flex-col gap-6 h-full overflow-auto">
      <!-- Page Header -->
      <div class="flex gap-3 items-center justify-between">
        <div class="flex gap-3 items-center">
          <h1 class="text-xl font-semibold">
            {{ t('pages.history.title') }}
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">{{ t('pages.history.subtitle') }}</span>
        </div>
        <div class="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            :disabled="!historyStore.entries.some(entry => !entry.isFavorite)"
            @click="openClearDialog('nonFavorites')"
          >
            {{ t('pages.history.clearNonFavorites') }}
          </Button>
          <Button
            variant="destructive"
            size="sm"
            :disabled="historyStore.entries.length === 0"
            @click="openClearDialog('all')"
          >
            {{ t('pages.history.clearAll') }}
          </Button>
        </div>
      </div>

      <Card class="flex flex-1 flex-col min-h-0">
        <CardHeader>
          <CardTitle>{{ t('pages.history.recent') }}</CardTitle>
          <CardDescription>
            {{ t('pages.history.recentDescription') }}
          </CardDescription>
        </CardHeader>
        <CardContent class="flex flex-1 flex-col gap-4 min-h-0">
          <!-- Filters -->
          <div class="flex gap-3 items-center">
            <Input
              v-model="searchQuery"
              class="flex-[3]"
              :placeholder="t('pages.history.search')"
            />
            <Select v-model="statusFilter">
              <SelectTrigger class="flex-[1]">
                <SelectValue :placeholder="t('pages.history.filterAll')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">
                  {{ t('pages.history.filterAll') }}
                </SelectItem>
                <SelectItem value="success">
                  {{ t('pages.history.filterSuccess') }}
                </SelectItem>
                <SelectItem value="error">
                  {{ t('pages.history.filterError') }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <!-- Table -->
          <div class="border rounded-md flex-1 overflow-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead class="w-8" />
                  <TableHead>{{ t('pages.history.columns.sql') }}</TableHead>
                  <TableHead class="w-36">
                    {{ t('pages.history.columns.connection') }}
                  </TableHead>
                  <TableHead class="w-32">
                    {{ t('pages.history.columns.database') }}
                  </TableHead>
                  <TableHead class="w-24">
                    {{ t('pages.history.columns.status') }}
                  </TableHead>
                  <TableHead class="w-44">
                    {{ t('pages.history.columns.time') }}
                  </TableHead>
                  <TableHead class="w-24">
                    {{ t('pages.history.columns.executionTime') }}
                  </TableHead>
                  <TableHead class="text-right w-28">
                    {{ t('pages.history.columns.actions') }}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                <template v-if="displayedEntries.length > 0">
                  <TableRow
                    v-for="entry in displayedEntries"
                    :key="entry.id"
                    :class="entry.isFavorite ? 'bg-yellow-50 dark:bg-yellow-900/10' : ''"
                  >
                    <!-- Favorite toggle -->
                    <TableCell class="px-2">
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger as-child>
                            <button
                              class="rounded flex h-6 w-6 cursor-pointer transition-colors items-center justify-center hover:text-yellow-500"
                              :class="entry.isFavorite ? 'text-yellow-500' : 'text-muted-foreground'"
                              @click="handleToggleFavorite(entry.id)"
                            >
                              <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                :fill="entry.isFavorite ? 'currentColor' : 'none'"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                              >
                                <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
                              </svg>
                            </button>
                          </TooltipTrigger>
                          <TooltipContent>
                            {{ entry.isFavorite ? t('pages.history.actions.unfavorite') : t('pages.history.actions.favorite') }}
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </TableCell>

                    <!-- SQL -->
                    <TableCell class="text-xs font-mono max-w-xs">
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger as-child>
                            <span class="block cursor-default truncate">{{ truncateSql(entry.sql) }}</span>
                          </TooltipTrigger>
                          <TooltipContent class="text-xs font-mono max-w-lg whitespace-pre-wrap break-all">
                            {{ entry.sql }}
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </TableCell>

                    <!-- Connection -->
                    <TableCell class="text-sm max-w-[9rem] truncate">
                      {{ entry.connectionName }}
                    </TableCell>

                    <!-- Database -->
                    <TableCell class="text-sm max-w-[8rem] truncate">
                      {{ entry.database ?? '-' }}
                    </TableCell>

                    <!-- Status -->
                    <TableCell>
                      <Badge :variant="entry.status === 'success' ? 'success' : 'destructive'">
                        {{ t(`pages.history.status.${entry.status}`) }}
                      </Badge>
                    </TableCell>

                    <!-- Timestamp -->
                    <TableCell class="text-xs text-muted-foreground whitespace-nowrap">
                      {{ formatTimestamp(entry.timestamp) }}
                    </TableCell>

                    <!-- Duration -->
                    <TableCell class="text-xs text-muted-foreground">
                      {{ formatDuration(entry.executionTime) }}
                    </TableCell>

                    <!-- Actions -->
                    <TableCell class="text-right">
                      <div class="flex gap-1 justify-end">
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger as-child>
                              <Button
                                variant="ghost"
                                size="icon"
                                class="h-7 w-7"
                                @click="handleRerun(entry)"
                              >
                                <svg
                                  xmlns="http://www.w3.org/2000/svg"
                                  width="14"
                                  height="14"
                                  viewBox="0 0 24 24"
                                  fill="none"
                                  stroke="currentColor"
                                  stroke-width="2"
                                  stroke-linecap="round"
                                  stroke-linejoin="round"
                                >
                                  <polygon points="5 3 19 12 5 21 5 3" />
                                </svg>
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>{{ t('pages.history.actions.rerun') }}</TooltipContent>
                          </Tooltip>
                        </TooltipProvider>

                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger as-child>
                              <Button
                                variant="ghost"
                                size="icon"
                                class="text-destructive h-7 w-7 hover:text-destructive"
                                @click="handleDelete(entry.id)"
                              >
                                <svg
                                  xmlns="http://www.w3.org/2000/svg"
                                  width="14"
                                  height="14"
                                  viewBox="0 0 24 24"
                                  fill="none"
                                  stroke="currentColor"
                                  stroke-width="2"
                                  stroke-linecap="round"
                                  stroke-linejoin="round"
                                >
                                  <polyline points="3 6 5 6 21 6" />
                                  <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6" />
                                  <path d="M10 11v6" />
                                  <path d="M14 11v6" />
                                  <path d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2" />
                                </svg>
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>{{ t('pages.history.actions.delete') }}</TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                      </div>
                    </TableCell>
                  </TableRow>
                </template>
                <template v-else>
                  <TableRow>
                    <TableCell colspan="8" class="text-muted-foreground py-8 text-center">
                      {{ t('pages.history.empty') }}
                    </TableCell>
                  </TableRow>
                </template>
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Clear Confirmation Dialog -->
    <AlertDialog v-model:open="clearDialogOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {{ clearMode === 'all' ? t('pages.history.clearAll') : t('pages.history.clearNonFavorites') }}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {{ clearDialogMessage }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{{ t('common.buttons.cancel') }}</AlertDialogCancel>
          <AlertDialogAction
            class="text-destructive-foreground bg-destructive hover:bg-destructive/90"
            @click="confirmClear"
          >
            {{ t('common.buttons.confirm') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </AppLayout>
</template>
