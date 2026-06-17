<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Spinner } from '@/components/ui/spinner'
import { toast } from '@/composables/useNotifications'

const props = withDefaults(defineProps<{
  open: boolean
  row: Record<string, unknown> | null
  columns: string[]
  columnTypes: Record<string, string>
  primaryKeys: string[]
  isNewRow?: boolean
  connectionId: string
  database?: string
  schema?: string
  tableName: string
}>(), {
  isNewRow: false,
  database: undefined,
  schema: undefined,
})

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void
  (e: 'saved'): void
  (e: 'close'): void
}>()

const { t } = useI18n()

// editForm: col -> { value: string, setNull: boolean }
const editForm = ref<Record<string, { value: string, setNull: boolean }>>({})
const editErrors = ref<Record<string, string>>({})
const saveError = ref<string | null>(null)
const isSaving = ref(false)

/** Initialize form when dialog opens */
watch(() => props.open, (opening) => {
  if (opening && props.row) {
    editForm.value = Object.fromEntries(
      props.columns.map(col => [
        col,
        {
          value: rawValueToString(props.row![col]),
          setNull: props.row![col] === null || props.row![col] === undefined,
        },
      ]),
    )
    editErrors.value = {}
    saveError.value = null
    isSaving.value = false
  }
})

/** True when any field differs from the original row values */
const isDirty = computed(() => {
  if (!props.row)
    return false
  return props.columns.some((col) => {
    const field = editForm.value[col]
    if (!field)
      return false
    const original = rawValueToString(props.row![col])
    const originalNull = props.row![col] === null || props.row![col] === undefined
    return field.value !== original || field.setNull !== originalNull
  })
})

function rawValueToString(v: unknown): string {
  if (v === null || v === undefined)
    return ''
  if (typeof v === 'object')
    return JSON.stringify(v)
  return String(v)
}

function isNumericType(type: string): boolean {
  return type.includes('int')
    || type.includes('serial')
    || type.includes('numeric')
    || type.includes('decimal')
    || type.includes('float')
    || type.includes('double')
    || type.includes('real')
    || type.includes('money')
    || type.includes('number')
}

function isValidNumber(value: string): boolean {
  return /^-?(?:\d+(?:\.\d+)?|\.\d+)(?:e[+-]?\d+)?$/i.test(value) && !Number.isNaN(Number(value))
}

function isValidBoolean(value: string): boolean {
  const lower = value.toLowerCase()
  return ['true', 'false', '1', '0', 't', 'f', 'y', 'n', 'yes', 'no'].includes(lower)
}

function coerceEditValue(col: string, value: string): unknown {
  const type = (props.columnTypes[col] ?? '').toLowerCase()
  if (type.includes('int') || type.includes('serial') || type.includes('numeric') || type.includes('decimal') || type.includes('float') || type.includes('double') || type.includes('real') || type.includes('money')) {
    const n = Number(value)
    if (!Number.isNaN(n))
      return n
  }
  if (type === 'bool' || type === 'boolean') {
    if (value.toLowerCase() === 'true')
      return true
    if (value.toLowerCase() === 'false')
      return false
  }
  return value
}

function validate(): boolean {
  const errors: Record<string, string> = {}
  for (const col of props.columns) {
    const field = editForm.value[col]
    if (!field)
      continue
    // Skip if set to NULL
    if (field.setNull)
      continue
    const value = field.value.trim()
    if (value === '')
      continue
    const type = (props.columnTypes[col] ?? '').toLowerCase()
    if (isNumericType(type)) {
      if (!isValidNumber(value)) {
        errors[col] = t('components.dataGrid.edit.validation.invalidNumber')
        continue
      }
    }
    if (type === 'bool' || type === 'boolean') {
      if (!isValidBoolean(value)) {
        errors[col] = t('components.dataGrid.edit.validation.invalidBoolean')
        continue
      }
    }
  }
  editErrors.value = errors
  return Object.keys(errors).length === 0
}

async function save() {
  if (!validate())
    return
  isSaving.value = true
  saveError.value = null
  try {
    const updates = Object.fromEntries(
      props.columns
        .filter(col => !props.primaryKeys.includes(col))
        .map((col) => {
          const field = editForm.value[col]
          if (!field)
            return null
          return [col, field.setNull ? null : coerceEditValue(col, field.value)] as [string, unknown]
        })
        .filter((entry): entry is [string, unknown] => entry !== null),
    )
    const pkValues = Object.fromEntries(
      props.primaryKeys.map(col => [col, props.row?.[col] ?? null]),
    )
    await invoke('update_table_row', {
      connectionId: props.connectionId,
      database: props.database ?? null,
      table: props.tableName,
      schema: props.schema ?? null,
      pkValues: props.isNewRow ? {} : pkValues,
      updates,
    })
    toast.success(`${t('components.dataGrid.edit.title')} → ${t('common.status.success')}`)
    emit('saved')
    emit('update:open', false)
  }
  catch (err) {
    saveError.value = String(err)
  }
  finally {
    isSaving.value = false
  }
}

function handleClose(open: boolean) {
  if (!open) {
    // eslint-disable-next-line no-alert
    if (isDirty.value && !window.confirm(t('components.dataGrid.edit.confirmCancelMessage')))
      return
    emit('update:open', false)
    emit('close')
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="handleClose">
    <DialogContent class="p-0 flex flex-col gap-0 max-h-[80vh] max-w-lg">
      <div class="px-6 pb-3 pt-5 border-b">
        <DialogTitle>
          {{ isNewRow ? t('components.dataGrid.edit.duplicateTitle') : t('components.dataGrid.edit.title') }}
        </DialogTitle>
      </div>

      <div class="px-6 py-4 flex-1 overflow-y-auto space-y-3">
        <div v-for="col in columns" :key="col" class="space-y-1">
          <div class="flex gap-2 items-center">
            <Label class="text-xs font-medium whitespace-nowrap">
              {{ col }}
              <span v-if="primaryKeys.includes(col)" class="text-[10px] text-amber-500 ml-1">(PK)</span>
              <span v-if="columnTypes[col]" class="text-[10px] text-muted-foreground font-mono ml-1">{{ columnTypes[col] }}</span>
            </Label>
            <div class="flex-1" />
            <!-- NULL toggle for non-PK columns -->
            <label
              v-if="!primaryKeys.includes(col)"
              class="text-xs text-muted-foreground flex gap-1 cursor-pointer select-none items-center"
            >
              <input
                type="checkbox"
                class="h-3 w-3 cursor-pointer"
                :checked="editForm[col]?.setNull"
                @change="editForm[col] = { ...editForm[col], setNull: !editForm[col]?.setNull, value: editForm[col]?.value ?? '' }"
              >
              {{ t('components.dataGrid.edit.nullToggle') }}
            </label>
            <span v-else class="text-xs text-muted-foreground italic">{{ t('components.dataGrid.edit.pkReadonly') }}</span>
          </div>
          <Input
            :model-value="editForm[col]?.setNull ? '' : (editForm[col]?.value ?? '')"
            :disabled="editForm[col]?.setNull || primaryKeys.includes(col)"
            :placeholder="editForm[col]?.setNull ? 'NULL' : ''"
            class="text-xs font-mono h-7"
            :class="{ 'border-destructive': editErrors[col], 'opacity-60 bg-muted': primaryKeys.includes(col) }"
            @update:model-value="(v: string | number) => editForm[col] = { ...editForm[col], value: String(v), setNull: editForm[col]?.setNull ?? false }"
          />
          <p v-if="editErrors[col]" class="text-xs text-destructive">
            {{ editErrors[col] }}
          </p>
        </div>

        <div v-if="saveError" class="text-sm text-destructive p-3 border border-destructive/30 rounded-md bg-destructive/10">
          {{ saveError }}
        </div>
      </div>

      <div class="px-6 py-3 border-t flex gap-2 justify-end">
        <Button variant="outline" size="sm" @click="handleClose(false)">
          {{ t('components.dataGrid.edit.cancel') }}
        </Button>
        <Button size="sm" :disabled="isSaving" @click="save">
          <Spinner v-if="isSaving" class="mr-1 h-3 w-3" />
          {{ isSaving ? t('components.dataGrid.edit.saving') : t('components.dataGrid.edit.save') }}
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>
