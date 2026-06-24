<script setup lang="ts">
import type { SavedQueryInfo, SavedQueryMetadata } from '@/datasources/fileApi'
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

type Props = {
  query: SavedQueryInfo
  metadata: SavedQueryMetadata | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'action', kind: string): void
}>()

const { t, locale } = useI18n()

function relativeTime(timestamp: number): string {
  if (timestamp == null || !Number.isFinite(timestamp))
    return ''
  const now = Date.now()
  const diff = timestamp * 1000 - now
  const seconds = Math.round(diff / 1000)
  const minutes = Math.round(seconds / 60)
  const hours = Math.round(minutes / 60)
  const days = Math.round(hours / 24)
  const weeks = Math.round(days / 7)
  const months = Math.round(days / 30)
  const years = Math.round(days / 365)

  // Intl API requires BCP 47 tags (en-US, zh-CN), but vue-i18n locale uses enUS, zhCN
  const localeTag = `${locale.value.slice(0, 2)}-${locale.value.slice(2)}`
  const rtf = new Intl.RelativeTimeFormat(localeTag, { style: 'narrow' })
  if (Math.abs(years) >= 1)
    return rtf.format(years, 'year')
  if (Math.abs(months) >= 1)
    return rtf.format(months, 'month')
  if (Math.abs(weeks) >= 1)
    return rtf.format(weeks, 'week')
  if (Math.abs(days) >= 1)
    return rtf.format(days, 'day')
  if (Math.abs(hours) >= 1)
    return rtf.format(hours, 'hour')
  if (Math.abs(minutes) >= 1)
    return rtf.format(minutes, 'minute')
  return rtf.format(seconds, 'second')
}

const createdAtText = computed(() => {
  if (!props.metadata?.createdAt)
    return null
  return relativeTime(props.metadata.createdAt)
})

const modifiedAtText = computed(() => {
  const ts = props.metadata?.modifiedAt ?? props.query.modified_at
  return relativeTime(ts)
})

const connectionName = computed(() => {
  if (!props.metadata?.connectionName)
    return null
  return props.metadata.connectionName
})

type ActionKind = 'open' | 'rename' | 'changeConnection' | 'reveal' | 'delete'

function handleAction(kind: ActionKind) {
  emit('action', kind)
}
</script>

<template>
  <div class="group px-2 py-1 cursor-pointer hover:bg-accent/30">
    <div class="flex gap-1.5 items-center">
      <span class="i-carbon-document text-muted-foreground shrink-0 h-3.5 w-3.5" />
      <span class="text-sm font-medium flex-1 truncate">{{ query.file_name }}</span>
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button
            variant="ghost"
            size="icon"
            class="opacity-0 shrink-0 h-5 w-5 group-hover:opacity-100"
          >
            <span class="i-carbon-overflow-menu-horizontal h-3 w-3" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem @select="handleAction('open')">
            <span class="i-carbon-document mr-2 h-3.5 w-3.5" /> {{ t('sidebar.savedQueries.actions.open') }}
          </DropdownMenuItem>
          <DropdownMenuItem @select="handleAction('rename')">
            <span class="i-carbon-edit mr-2 h-3.5 w-3.5" /> {{ t('sidebar.savedQueries.actions.rename') }}
          </DropdownMenuItem>
          <DropdownMenuItem @select="handleAction('changeConnection')">
            <span class="i-carbon-data-base mr-2 h-3.5 w-3.5" /> {{ t('sidebar.savedQueries.actions.changeConnection') }}
          </DropdownMenuItem>
          <DropdownMenuItem @select="handleAction('reveal')">
            <span class="i-carbon-folder-open mr-2 h-3.5 w-3.5" /> {{ t('sidebar.savedQueries.actions.reveal') }}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem class="text-destructive" @select="handleAction('delete')">
            <span class="i-carbon-trash-can mr-2 h-3.5 w-3.5" /> {{ t('sidebar.savedQueries.actions.delete') }}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
    <div class="mt-0.5 pl-[calc(14px+6px)] space-y-0.5">
      <div class="text-xs text-muted-foreground truncate">
        <span v-if="createdAtText">{{ t('sidebar.savedQueries.created') }} {{ createdAtText }}</span>
        <span v-if="createdAtText && modifiedAtText"> · </span>
        <span v-if="modifiedAtText">{{ t('sidebar.savedQueries.modified') }} {{ modifiedAtText }}</span>
      </div>
      <div v-if="connectionName" class="text-xs text-muted-foreground flex gap-1 truncate items-center">
        <span class="i-carbon-arrow-right shrink-0 h-2.5 w-2.5" />
        <span class="truncate">{{ connectionName }}</span>
      </div>
      <div v-else class="text-xs text-muted-foreground/60 truncate italic">
        {{ t('sidebar.savedQueries.connectionNone') }}
      </div>
    </div>
  </div>
</template>
