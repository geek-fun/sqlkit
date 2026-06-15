<script setup lang="ts">
import type { IndexInfo } from '@/datasources/browseApi'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Spinner } from '@/components/ui/spinner'
import { browseApi } from '@/datasources/browseApi'

const props = defineProps<{
  connectionId: string
  database: string
  schema: string | null
  tableName: string
}>()

const { t } = useI18n()

const indexes = ref<IndexInfo[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

async function fetchIndexes() {
  loading.value = true
  error.value = null
  try {
    indexes.value = await browseApi.listIndexes(
      props.connectionId,
      props.database,
      props.schema,
      props.tableName,
    )
  }
  catch (err) {
    error.value = String(err)
  }
  finally {
    loading.value = false
  }
}

onMounted(fetchIndexes)
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
    <div v-else-if="indexes.length === 0" class="text-sm text-muted-foreground py-8 text-center">
      {{ t('components.indexesTab.empty') }}
    </div>
    <div v-else class="border rounded-md">
      <table class="text-xs w-full">
        <thead>
          <tr class="text-left bg-muted/50">
            <th class="font-medium px-3 py-2">
              {{ t('components.indexesTab.name') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.indexesTab.columns') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.indexesTab.type') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.indexesTab.unique') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.indexesTab.primary') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="idx in indexes"
            :key="idx.name"
            class="border-t border-border/50 hover:bg-accent/30"
          >
            <td class="font-medium px-3 py-1.5">
              {{ idx.name }}
            </td>
            <td class="text-muted-foreground font-mono px-3 py-1.5">
              {{ idx.columns.join(', ') }}
            </td>
            <td class="text-muted-foreground font-mono px-3 py-1.5">
              {{ idx.index_type }}
            </td>
            <td class="px-3 py-1.5">
              <span v-if="idx.is_unique" class="text-green-500 font-medium">YES</span>
              <span v-else class="text-muted-foreground">NO</span>
            </td>
            <td class="px-3 py-1.5">
              <span v-if="idx.is_primary" class="text-amber-500 font-medium">YES</span>
              <span v-else class="text-muted-foreground">NO</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
