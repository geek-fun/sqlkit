<script setup lang="ts">
import type { ForeignKeyInfo } from '@/datasources/browseApi'
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

const foreignKeys = ref<ForeignKeyInfo[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

async function fetchForeignKeys() {
  loading.value = true
  error.value = null
  try {
    foreignKeys.value = await browseApi.listForeignKeys(
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

onMounted(fetchForeignKeys)
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
    <div v-else-if="foreignKeys.length === 0" class="text-sm text-muted-foreground py-8 text-center">
      {{ t('components.foreignKeysTab.empty') }}
    </div>
    <div v-else class="border rounded-md">
      <table class="text-xs w-full">
        <thead>
          <tr class="text-left bg-muted/50">
            <th class="font-medium px-3 py-2">
              {{ t('components.foreignKeysTab.constraint') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.foreignKeysTab.columns') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.foreignKeysTab.references') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.foreignKeysTab.onUpdate') }}
            </th>
            <th class="font-medium px-3 py-2">
              {{ t('components.foreignKeysTab.onDelete') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="fk in foreignKeys"
            :key="fk.constraint_name"
            class="border-t border-border/50 hover:bg-accent/30"
          >
            <td class="font-medium px-3 py-1.5">
              {{ fk.constraint_name }}
            </td>
            <td class="text-muted-foreground font-mono px-3 py-1.5">
              {{ fk.columns.join(', ') }}
            </td>
            <td class="text-muted-foreground font-mono px-3 py-1.5">
              <template v-if="fk.referenced_schema">
                {{ fk.referenced_schema }}.
              </template>{{ fk.referenced_table }}({{ fk.referenced_columns.join(', ') }})
            </td>
            <td class="text-muted-foreground px-3 py-1.5">
              {{ fk.on_update ?? '—' }}
            </td>
            <td class="text-muted-foreground px-3 py-1.5">
              {{ fk.on_delete ?? '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
