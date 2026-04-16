<script setup lang="ts">
import type { ExportFormat } from '@/types/transfer'

import { ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { useTransferStore } from '@/store/transferStore'

const transferStore = useTransferStore()

const formats: { value: ExportFormat, label: string, defaults: string }[] = [
  { value: 'csv', label: 'CSV (.csv)', defaults: 'Comma delimiter, double-quote, UTF-8, include header' },
  { value: 'jsonl', label: 'JSONL (.jsonl)', defaults: 'One JSON object per line, compact, UTF-8' },
]

const selectedFormat = ref<ExportFormat>('csv')
const showAdvanced = ref(false)

const csvDelimiter = ref(',')
const csvEncoding = ref('UTF-8')
const csvIncludeHeader = ref(true)

watch([selectedFormat, csvDelimiter, csvEncoding, csvIncludeHeader], () => {
  transferStore.exportRequest = {
    ...transferStore.exportRequest,
    format: selectedFormat.value,
    csvOptions: selectedFormat.value === 'csv'
      ? {
          delimiter: csvDelimiter.value,
          encoding: csvEncoding.value,
          includeHeader: csvIncludeHeader.value,
        }
      : undefined,
  }
})
</script>

<template>
  <div class="space-y-6">
    <Label>Select Format</Label>
    <RadioGroup v-model="selectedFormat" class="gap-4 grid grid-cols-2">
      <div
        v-for="format in formats"
        :key="format.value"
        class="p-4 border rounded flex transition-colors items-center space-x-2"
        :class="selectedFormat === format.value ? 'border-primary bg-secondary' : 'border-border'"
      >
        <RadioGroupItem :value="format.value" />
        <div class="flex-1">
          <Label class="cursor-pointer">{{ format.label }}</Label>
          <div class="text-xs text-muted-foreground mt-1">
            {{ format.defaults }}
          </div>
        </div>
      </div>
    </RadioGroup>

    <Button variant="ghost" class="w-full" @click="showAdvanced = !showAdvanced">
      {{ showAdvanced ? 'Hide' : 'Show' }} Advanced Options
    </Button>

    <div v-if="showAdvanced && selectedFormat === 'csv'" class="gap-4 grid grid-cols-3">
      <div class="space-y-2">
        <Label>Delimiter</Label>
        <Select v-model="csvDelimiter">
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value=",">
              Comma (,)
            </SelectItem>
            <SelectItem value=";">
              Semicolon (;)
            </SelectItem>
            <SelectItem value="\t">
              Tab
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="space-y-2">
        <Label>Encoding</Label>
        <Select v-model="csvEncoding">
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="UTF-8">
              UTF-8
            </SelectItem>
            <SelectItem value="ISO-8859-1">
              ISO-8859-1
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="flex items-center space-y-2">
        <Checkbox v-model:checked="csvIncludeHeader" />
        <Label class="ml-2 cursor-pointer">Include header row</Label>
      </div>
    </div>
  </div>
</template>
