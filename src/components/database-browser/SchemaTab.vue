<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
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
    <div v-if="loading" class="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
      <Spinner size="sm" />
      {{ t('common.loading') }}
    </div>
    <div v-else-if="error" class="py-8 text-sm text-destructive text-center">
      {{ error }}
    </div>
    <div v-else class="rounded-md border">
      <table class="w-full text-xs">
        <thead>
          <tr class="text-left bg-muted/50">
            <th class="px-3 py-2 font-medium">
              {{ t('components.schemaTab.column') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.schemaTab.type') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.schemaTab.nullable') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.schemaTab.default') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.schemaTab.key') }}
            </th>
            <th class="px-3 py-2 font-medium">
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
            <td class="px-3 py-1.5 font-medium">
              {{ col.name }}
            </td>
            <td class="px-3 py-1.5 font-mono text-muted-foreground">
              {{ col.data_type }}
              <template v-if="col.max_length">({{ col.max_length }})</template>
              <template v-else-if="col.precision != null">
                ({{ col.precision }}{{ col.scale != null ? `, ${col.scale}` : '' }})
              </template>
            </td>
            <td class="px-3 py-1.5">
              <span v-if="col.nullable" class="text-muted-foreground">YES</span>
              <span v-else class="text-destructive font-medium">NO</span>
            </td>
            <td class="px-3 py-1.5 font-mono text-muted-foreground max-w-[150px] truncate">
              {{ col.default_value ?? '—' }}
            </td>
            <td class="px-3 py-1.5">
              <span v-if="col.is_primary_key" class="text-amber-500 font-medium">PK</span>
              <span v-else class="text-muted-foreground">—</span>
            </td>
            <td class="px-3 py-1.5">
              <span v-if="col.is_auto_increment" class="text-green-500 font-medium">YES</span>
              <span v-else class="text-muted-foreground">—</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
