<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Spinner } from '@/components/ui/spinner'

type ColumnInfo = {
  name: string
  data_type: string
  nullable: boolean
  is_primary_key: boolean
  is_auto_increment: boolean
  default_value: string | null
  max_length: number | null
  precision: number | null
  scale: number | null
  description: string | null
}

const props = defineProps<{
  connectionId: string
  database: string
  schema: string | null
  tableName: string
}>()

const { t } = useI18n()

const columns = ref<ColumnInfo[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

async function fetchColumns() {
  loading.value = true
  error.value = null
  try {
    const result = await invoke<ColumnInfo[]>('list_columns', {
      connectionId: props.connectionId,
      database: props.database,
      schema: props.schema,
      tableName: props.tableName,
    })
    columns.value = result
  }
  catch (err) {
    error.value = String(err)
  }
  finally {
    loading.value = false
  }
}

onMounted(fetchColumns)
</script>

<template>
  <div class="p-3 flex-1 overflow-auto">
    <div v-if="loading" class="text-sm text-muted-foreground py-8 flex gap-2 items-center justify-center">
      <Spinner size="sm" />
      {{ t('common.loading') }}
    </div>
    <div v-else-if="error" class="text-sm text-destructive py-8 text-center">
      {{ error }}
    </div>
    <div v-else class="border rounded-md">
      <table class="text-xs w-full">
        <thead>
          <tr class="text-left bg-muted/50">
            <th class="font-medium px-3 py-2">
              {{ t('components.schemaTab.column') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.schemaTab.type') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.schemaTab.nullable') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.schemaTab.default') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.schemaTab.key') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.schemaTab.autoIncrement') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="col in columns"
            :key="col.name"
            class="border-t border-border/50 hover:bg-accent/30"
          >
            <td class="font-medium px-3 py-1.5">
              {{ col.name }}
            </td>
            <td class="text-muted-foreground font-mono px-3 py-1.5">
              {{ col.data_type }}
              <template v-if="col.max_length">
                ({{ col.max_length }})
              </template>
              <template v-else-if="col.precision != null">
                ({{ col.precision }}{{ col.scale != null ? `, ${col.scale}` : '' }})
              </template>
            </td>
            <td class="px-3 py-1.5">
              <span v-if="col.nullable" class="text-muted-foreground">{{ t('components.schemaTab.yes') }}</span>
              <span v-else class="text-destructive font-medium">{{ t('components.schemaTab.no') }}</span>
            </td>
            <td class="text-muted-foreground font-mono px-3 py-1.5 max-w-[150px] truncate">
              {{ col.default_value ?? '—' }}
            </td>
            <td class="px-3 py-1.5">
              <span v-if="col.is_primary_key" class="text-amber-500 font-medium">{{ t('components.schemaTab.pk') }}</span>
              <span v-else class="text-muted-foreground">—</span>
            </td>
            <td class="px-3 py-1.5">
              <span v-if="col.is_auto_increment" class="text-green-500 font-medium">{{ t('components.schemaTab.yes') }}</span>
              <span v-else class="text-muted-foreground">—</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
