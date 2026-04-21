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
    <div class="space-y-3">
      <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Select Format</Label>
      <RadioGroup v-model="selectedFormat" class="gap-4 grid grid-cols-1 md:grid-cols-2">
        <label
          v-for="format in formats"
          :key="format.value"
          class="p-4 border rounded-lg flex cursor-pointer transition-all duration-200 items-start space-x-3 hover:bg-muted/50"
          :class="selectedFormat === format.value ? 'border-primary bg-primary/5 shadow-sm' : 'border-border bg-card'"
        >
          <RadioGroupItem :id="`format-${format.value}`" :value="format.value" class="mt-1" />
          <div class="flex-1 space-y-1">
            <div class="text-sm tracking-tight font-semibold">{{ format.label }}</div>
            <div class="text-xs text-muted-foreground">
              {{ format.defaults }}
            </div>
          </div>
        </label>
      </RadioGroup>
    </div>

    <Button variant="ghost" class="text-xs text-muted-foreground w-full" @click="showAdvanced = !showAdvanced">
      <span :class="showAdvanced ? 'i-carbon-chevron-up' : 'i-carbon-chevron-down'" class="mr-2" />
      {{ showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options' }}
    </Button>

    <div v-if="showAdvanced && selectedFormat === 'csv'" class="pt-6 border-t gap-5 grid grid-cols-1 md:grid-cols-3 sm:grid-cols-2">
      <div class="space-y-2.5">
        <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Delimiter</Label>
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
      <div class="space-y-2.5">
        <Label class="text-xs text-muted-foreground tracking-wider font-semibold uppercase">Encoding</Label>
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
      <div class="flex items-center space-x-2 sm:mt-8">
        <Checkbox id="export-csv-header" v-model:checked="csvIncludeHeader" />
        <Label for="export-csv-header" class="text-sm leading-none font-medium cursor-pointer">Include header row</Label>
      </div>
    </div>
  </div>
</template>
