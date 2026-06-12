<script setup lang="ts">
import type { ExportRequest } from '@/types/transfer'

import { ref, watch } from 'vue'

import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import ColumnSelector from '../shared/ColumnSelector.vue'
import ConnectionSelector from '../shared/ConnectionSelector.vue'

import TableSelector from '../shared/TableSelector.vue'

const props = defineProps<{
  modelValue: Partial<ExportRequest>
}>()

const emit = defineEmits<{
  'update:modelValue': [value: Partial<ExportRequest>]
}>()

const connectionId = ref('')
const database = ref('')
const schema = ref('')
const table = ref('')
const columns = ref<string[]>([])
const whereClause = ref('')
const orderBy = ref('')
const limit = ref<number | undefined>()

watch([connectionId, database, schema, table, columns, whereClause, orderBy, limit], () => {
  emit('update:modelValue', {
    ...props.modelValue,
    connectionId: connectionId.value || undefined,
    database: database.value || undefined,
    schema: schema.value || undefined,
    sources: [{
      table: table.value,
      columns: columns.value,
    }],
  })
})

// Watch for external changes to modelValue
watch(() => props.modelValue, (newValue) => {
  if (newValue.connectionId !== connectionId.value)
    connectionId.value = newValue.connectionId || ''
  if (newValue.database !== database.value)
    database.value = newValue.database || ''
  if (newValue.schema !== schema.value)
    schema.value = newValue.schema || ''
  if (newValue.sources?.[0]?.table !== table.value)
    table.value = newValue.sources?.[0]?.table || ''
  if (JSON.stringify(newValue.sources?.[0]?.columns) !== JSON.stringify(columns.value))
    columns.value = newValue.sources?.[0]?.columns || []
}, { deep: true })
</script>

<template>
  <div class="space-y-3">
    <ConnectionSelector
      v-model:connection-id="connectionId"
      v-model:database="database"
      v-model:schema="schema"
      show-schema
    />

    <TableSelector
      v-model:table="table"
      :connection-id="connectionId"
      :database="database"
      :schema="schema"
    />

    <div v-if="table" class="pt-3 border-t border-border/40 space-y-3">
      <ColumnSelector
        v-model:columns="columns"
        :connection-id="connectionId"
        :database="database"
        :schema="schema"
        :table="table"
      />

      <div class="gap-2.5 grid grid-cols-1 md:grid-cols-2">
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">{{ $t('transfer.export.whereClause') }}</Label>
          <Input
            v-model="whereClause"
            :placeholder="$t('transfer.export.whereClausePlaceholder')"
            class="text-xs font-mono h-8 tabular-nums"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">{{ $t('transfer.export.orderBy') }}</Label>
          <Input
            v-model="orderBy"
            :placeholder="$t('transfer.export.orderByPlaceholder')"
            class="text-xs font-mono h-8 tabular-nums"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">{{ $t('transfer.export.limit') }}</Label>
          <Input
            v-model.number="limit"
            type="number"
            min="1"
            :placeholder="$t('transfer.export.limitPlaceholder')"
            class="text-xs font-mono h-8 tabular-nums"
          />
        </div>
      </div>
    </div>
  </div>
</template>
