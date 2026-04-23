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
  <div class="space-y-4">
    <div class="space-y-2">
      <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Select Format</Label>
      <RadioGroup v-model="selectedFormat" class="gap-2.5 grid grid-cols-1 md:grid-cols-2">
        <label
          v-for="format in formats"
          :key="format.value"
          class="p-2.5 border rounded-md flex cursor-pointer transition-all duration-200 items-start space-x-2.5 hover:bg-muted/40"
          :class="selectedFormat === format.value ? 'border-primary/60 bg-primary/[0.04] shadow-sm' : 'border-border/40 bg-card'"
        >
          <RadioGroupItem :id="`format-${format.value}`" :value="format.value" class="mt-0.5" />
          <div class="flex-1 space-y-1">
            <div class="text-xs tracking-tight font-semibold">{{ format.label }}</div>
            <div class="text-[11px] text-muted-foreground">
              {{ format.defaults }}
            </div>
          </div>
        </label>
      </RadioGroup>
    </div>

    <Button variant="ghost" size="sm" class="text-[11px] text-muted-foreground h-8 w-full" @click="showAdvanced = !showAdvanced">
      <span :class="showAdvanced ? 'i-carbon-chevron-up' : 'i-carbon-chevron-down'" class="mr-1.5" />
      {{ showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options' }}
    </Button>

    <div v-if="showAdvanced && selectedFormat === 'csv'" class="p-3 border border-border/40 rounded-md bg-muted/20 gap-2.5 grid grid-cols-1 md:grid-cols-3">
      <div class="space-y-1.5">
        <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Delimiter</Label>
        <Select v-model="csvDelimiter">
          <SelectTrigger class="text-xs font-mono h-8">
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
      <div class="space-y-1.5">
        <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Encoding</Label>
        <Select v-model="csvEncoding">
          <SelectTrigger class="text-xs font-mono h-8">
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
      <div class="flex items-center space-x-2 sm:mt-5">
        <Checkbox id="export-csv-header" v-model:checked="csvIncludeHeader" />
        <Label for="export-csv-header" class="text-xs leading-none font-medium cursor-pointer">Include header row</Label>
      </div>
    </div>
  </div>
</template>
