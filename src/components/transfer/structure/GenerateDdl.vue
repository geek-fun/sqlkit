<script setup lang="ts">
import type { DdlObject, DdlOptions } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'

import ConnectionSelector from '../shared/ConnectionSelector.vue'

const { t } = useI18n()

const connectionId = ref('')
const database = ref('')
const schema = ref('')
const objects = ref<DdlObject[]>([])
const selectedObjects = ref<string[]>([])
const generatedDdl = ref('')
const loading = ref(false)
const previewing = ref(false)

const ddlOptions = ref<DdlOptions>({
  includeCreateTable: true,
  includePrimaryKeys: true,
  includeForeignKeys: true,
  includeIndexes: true,
  includeConstraints: true,
  includeComments: false,
  includeStorageOptions: false,
  includeDropIfExists: true,
  includeIfNotExists: false,
  includeData: false,
})

async function loadObjects() {
  if (!connectionId.value || !database.value)
    return

  loading.value = true
  try {
    const tables = await invoke<{ name: string, schema?: string }[]>('list_tables', {
      connectionId: connectionId.value,
      database: database.value,
      schema: schema.value || null,
    })

    objects.value = tables.map(table => ({
      name: table.name,
      objectType: 'table',
      schema: table.schema,
    }))
  }
  catch (error) {
    console.error('Failed to load objects:', error)
  }
  finally {
    loading.value = false
  }
}

async function generateDdl() {
  if (!connectionId.value || objects.value.length === 0)
    return

  previewing.value = true
  try {
    const result = await invoke<string>('generate_ddl', {
      request: {
        connectionId: connectionId.value,
        database: database.value || null,
        schema: schema.value || null,
        objects: objects.value.filter(o => selectedObjects.value.includes(o.name)),
        options: ddlOptions.value,
      },
    })
    generatedDdl.value = result
  }
  catch (error) {
    console.error('Failed to generate DDL:', error)
  }
  finally {
    previewing.value = false
  }
}

const isObjectSelected = (name: string) => selectedObjects.value.includes(name)

function toggleObject(name: string) {
  const current = [...selectedObjects.value]
  const index = current.indexOf(name)
  if (index > -1) {
    current.splice(index, 1)
  }
  else {
    current.push(name)
  }
  selectedObjects.value = current
}

function selectAll() {
  selectedObjects.value = objects.value.map(o => o.name)
}

function deselectAll() {
  selectedObjects.value = []
}

async function copyToClipboard() {
  await navigator.clipboard.writeText(generatedDdl.value)
}

async function saveToFile() {
  try {
    const path = await invoke<string>('save_file_dialog', {
      defaultPath: 'ddl.sql',
      filters: [{ name: 'SQL', extensions: ['sql'] }],
    })
    if (path) {
      await invoke('write_text_file', { path, content: generatedDdl.value })
    }
  }
  catch (error) {
    console.error('Failed to save file:', error)
  }
}

const canGenerate = computed(() =>
  connectionId.value !== '' && selectedObjects.value.length > 0,
)
</script>

<template>
  <div class="space-y-6">
    <ConnectionSelector
      v-model:connection-id="connectionId"
      v-model:database="database"
      v-model:schema="schema"
      show-schema
      @update:database="loadObjects"
    />

    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <Label>{{ t('pages.transfer.structure.objects') }}</Label>
        <div class="flex gap-2">
          <Button variant="outline" size="sm" @click="selectAll">
            {{ t('pages.transfer.structure.selectAll') }}
          </Button>
          <Button variant="outline" size="sm" @click="deselectAll">
            {{ t('pages.transfer.structure.deselectAll') }}
          </Button>
        </div>
      </div>

      <div v-if="loading" class="text-sm text-muted-foreground">
        {{ t('pages.transfer.structure.loadingObjects') }}
      </div>

      <div v-else-if="objects.length === 0 && connectionId" class="text-sm text-muted-foreground">
        {{ t('pages.transfer.structure.noObjects') }}
      </div>

      <div v-else class="border rounded max-h-300px overflow-auto">
        <div
          v-for="obj in objects"
          :key="obj.name"
          class="p-2 border-b flex cursor-pointer items-center space-x-2 hover:bg-secondary/50"
          :class="isObjectSelected(obj.name) ? 'bg-secondary' : ''"
          @click="toggleObject(obj.name)"
        >
          <Checkbox :checked="isObjectSelected(obj.name)" />
          <span class="text-sm">{{ obj.name }}</span>
          <span class="text-xs text-muted-foreground">{{ obj.objectType }}</span>
        </div>
      </div>

      <div class="text-sm text-muted-foreground">
        {{ selectedObjects.length }} {{ t('pages.transfer.structure.objectsSelected') }}
      </div>
    </div>

    <div class="space-y-4">
      <Label>{{ t('pages.transfer.structure.options') }}</Label>

      <div class="gap-4 grid grid-cols-2">
        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="ddlOptions.includeCreateTable" />
          <Label class="text-sm">{{ t('pages.transfer.structure.includeCreateTable') }}</Label>
        </div>
        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="ddlOptions.includePrimaryKeys" />
          <Label class="text-sm">{{ t('pages.transfer.structure.includePrimaryKeys') }}</Label>
        </div>
        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="ddlOptions.includeForeignKeys" />
          <Label class="text-sm">{{ t('pages.transfer.structure.includeForeignKeys') }}</Label>
        </div>
        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="ddlOptions.includeIndexes" />
          <Label class="text-sm">{{ t('pages.transfer.structure.includeIndexes') }}</Label>
        </div>
        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="ddlOptions.includeDropIfExists" />
          <Label class="text-sm">{{ t('pages.transfer.structure.includeDropIfExists') }}</Label>
        </div>
        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="ddlOptions.includeIfNotExists" />
          <Label class="text-sm">{{ t('pages.transfer.structure.includeIfNotExists') }}</Label>
        </div>
      </div>
    </div>

    <div class="flex gap-2 justify-end">
      <Button :disabled="!canGenerate" :loading="previewing" @click="generateDdl">
        {{ t('pages.transfer.structure.generate') }}
      </Button>
    </div>

    <div v-if="generatedDdl" class="space-y-4">
      <Label>{{ t('pages.transfer.structure.preview') }}</Label>

      <div class="p-4 border rounded bg-muted/30 max-h-400px overflow-auto">
        <pre class="text-sm font-mono whitespace-pre-wrap">{{ generatedDdl }}</pre>
      </div>

      <div class="flex gap-2">
        <Button variant="outline" @click="copyToClipboard">
          {{ t('pages.transfer.structure.copyToClipboard') }}
        </Button>
        <Button variant="outline" @click="saveToFile">
          {{ t('pages.transfer.structure.saveToFile') }}
        </Button>
      </div>
    </div>
  </div>
</template>
