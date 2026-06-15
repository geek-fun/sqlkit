<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ForeignKeyInfo } from '@/datasources/browseApi'
import { browseApi } from '@/datasources/browseApi'
import { Spinner } from '@/components/ui/spinner'

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
    <div v-if="loading" class="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
      <Spinner size="sm" />
      {{ t('common.loading') }}
    </div>
    <div v-else-if="error" class="py-8 text-sm text-destructive text-center">
      {{ error }}
    </div>
    <div v-else-if="foreignKeys.length === 0" class="py-8 text-sm text-muted-foreground text-center">
      {{ t('components.foreignKeysTab.empty') }}
    </div>
    <div v-else class="rounded-md border">
      <table class="w-full text-xs">
        <thead>
          <tr class="text-left bg-muted/50">
            <th class="px-3 py-2 font-medium">
              {{ t('components.foreignKeysTab.constraint') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.foreignKeysTab.columns') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.foreignKeysTab.references') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.foreignKeysTab.onUpdate') }}
            </th>
            <th class="px-3 py-2 font-medium">
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
            <td class="px-3 py-1.5 font-medium">
              {{ fk.constraint_name }}
            </td>
            <td class="px-3 py-1.5 font-mono text-muted-foreground">
              {{ fk.columns.join(', ') }}
            </td>
            <td class="px-3 py-1.5 font-mono text-muted-foreground">
              <template v-if="fk.referenced_schema">{{ fk.referenced_schema }}.</template>{{ fk.referenced_table }}({{ fk.referenced_columns.join(', ') }})
            </td>
            <td class="px-3 py-1.5 text-muted-foreground">
              {{ fk.on_update ?? '—' }}
            </td>
            <td class="px-3 py-1.5 text-muted-foreground">
              {{ fk.on_delete ?? '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
