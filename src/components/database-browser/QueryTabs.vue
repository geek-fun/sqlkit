<script setup lang="ts">
import type { QueryTab } from '@/store/tabStore'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
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
import { Button } from '@/components/ui/button'

interface Props {
  tabs: QueryTab[]
  activeTabId: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'select', tabId: string): void
  (e: 'close', tabId: string): void
  (e: 'closeForce', tabId: string): void
  (e: 'new'): void
}>()

const { t } = useI18n()

const showCloseDialog = ref(false)
const tabToClose = ref<QueryTab | null>(null)

const getTabTitle = (tab: QueryTab) => tab.name

function handleTabClick(tabId: string) {
  emit('select', tabId)
}

function handleCloseClick(event: MouseEvent, tab: QueryTab) {
  event.stopPropagation()

  if (tab.hasUnsavedChanges) {
    tabToClose.value = tab
    showCloseDialog.value = true
  }
  else {
    emit('close', tab.id)
  }
}

function confirmClose() {
  if (tabToClose.value) {
    emit('closeForce', tabToClose.value.id)
    tabToClose.value = null
  }
  showCloseDialog.value = false
}

function cancelClose() {
  tabToClose.value = null
  showCloseDialog.value = false
}

function triggerClose(tabId: string) {
  const tab = props.tabs.find(t => t.id === tabId)
  if (!tab)
    return
  if (tab.hasUnsavedChanges) {
    tabToClose.value = tab
    showCloseDialog.value = true
  }
  else {
    emit('close', tab.id)
  }
}

defineExpose({ triggerClose })

function handleNewTab() {
  emit('new')
}

const isActiveTab = (tabId: string) => props.activeTabId === tabId
</script>

<template>
  <div class="query-tabs border-b bg-muted/30 flex items-center overflow-x-auto">
    <!-- Tabs -->
    <div class="flex flex-1 min-w-0 items-center">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="tab-item group px-3 py-1.5 border-r flex gap-1 cursor-pointer transition-colors items-center"
        :class="{
          'bg-background': isActiveTab(tab.id),
          'hover:bg-accent/50': !isActiveTab(tab.id),
        }"
        @click="handleTabClick(tab.id)"
      >
        <!-- Tab icon: table grid icon for table-view tabs, file icon for query tabs -->
        <svg
          v-if="tab.tableView"
          xmlns="http://www.w3.org/2000/svg"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="text-muted-foreground flex-shrink-0"
        >
          <rect width="18" height="18" x="3" y="3" rx="2" />
          <path d="M3 9h18" />
          <path d="M3 15h18" />
          <path d="M9 3v18" />
        </svg>
        <svg
          v-else
          xmlns="http://www.w3.org/2000/svg"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="text-muted-foreground flex-shrink-0"
        >
          <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
          <polyline points="14 2 14 8 20 8" />
        </svg>

        <!-- Tab title -->
        <span class="text-sm max-w-32 truncate" :title="getTabTitle(tab)">
          {{ getTabTitle(tab) }}
        </span>

        <!-- Unsaved indicator -->
        <span
          v-if="tab.hasUnsavedChanges"
          class="rounded-full bg-orange-500 flex-shrink-0 h-2 w-2"
          :title="t('components.queryTabs.unsaved')"
        />

        <!-- Executing indicator -->
        <svg
          v-if="tab.isExecuting"
          class="text-primary flex-shrink-0 h-3 w-3 animate-spin"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
        </svg>

        <!-- Close button -->
        <button
          class="rounded opacity-0 flex flex-shrink-0 h-4 w-4 transition-opacity items-center justify-center hover:bg-accent group-hover:opacity-100"
          :class="{ 'opacity-100': isActiveTab(tab.id) }"
          @click="handleCloseClick($event, tab)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 6 6 18" />
            <path d="m6 6 12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- New tab button -->
    <Button
      variant="ghost"
      size="icon"
      class="mx-1 flex-shrink-0 h-7 w-7"
      :title="t('components.queryTabs.new')"
      @click="handleNewTab"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M5 12h14" />
        <path d="M12 5v14" />
      </svg>
    </Button>

    <!-- Unsaved changes dialog -->
    <AlertDialog v-model:open="showCloseDialog">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t('components.queryTabs.closeDialog.title') }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{ t('components.queryTabs.closeDialog.message', { name: tabToClose?.name || '' }) }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel @click="cancelClose">
            {{ t('common.buttons.cancel') }}
          </AlertDialogCancel>
          <AlertDialogAction
            class="text-destructive-foreground bg-destructive hover:bg-destructive/90"
            @click="confirmClose"
          >
            {{ t('components.queryTabs.closeDialog.discard') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>

<style scoped>
.query-tabs {
  min-height: 32px;
}

.tab-item {
  min-width: 0;
  max-width: 200px;
}
</style>
