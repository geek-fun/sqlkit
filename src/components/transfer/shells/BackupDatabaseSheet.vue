<script setup lang="ts">
import type { ExportFormat } from '@/types/transfer'
import { open } from '@tauri-apps/plugin-dialog'
import { DialogOverlay, DialogPortal } from 'radix-vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import MultiTableSelector from '@/components/transfer/shared/MultiTableSelector.vue'
import WizardStepper from '@/components/transfer/shared/WizardStepper.vue'

import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useTransferStore } from '@/store/transferStore'

const props = defineProps<{
  open: boolean
  connectionId: string
  database: string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  'submitted': [jobId: string]
}>()

const { t } = useI18n()
const transferStore = useTransferStore()

const isOpen = computed({
  get: () => props.open,
  set: val => emit('update:open', val),
})

const steps = computed(() => [
  t('transfer.surface.backupDatabase.selectTables', 'Select tables'),
  t('transfer.surface.backupDatabase.options', 'Backup options'),
  t('transfer.surface.backupDatabase.destination', 'Destination'),
  t('transfer.surface.backupDatabase.review', 'Review'),
])

const currentStep = ref(0)
const selectedTables = ref<string[]>([])
const selectedFormat = ref<ExportFormat>('csv')
const destination = ref('')

watch(() => props.open, (newVal) => {
  if (newVal) {
    currentStep.value = 0
    selectedTables.value = []
    selectedFormat.value = 'csv'
    destination.value = ''
  }
})

async function pickDestination() {
  const path = await open({ directory: true })
  if (path) {
    destination.value = typeof path === 'string' ? path : path[0]
  }
}

async function handleBackup() {
  if (!destination.value || selectedTables.value.length === 0)
    return

  const tablesSelection: Record<string, string[]> = {
    [`${props.database}.`]: selectedTables.value,
  }

  try {
    const jobId = await transferStore.startBackupServer({
      connectionId: props.connectionId,
      name: t('transfer.surface.backupDatabase.jobName', { db: props.database }),
      selection: {
        serverId: props.connectionId,
        databases: [props.database],
        schemas: {},
        tables: tablesSelection,
      },
      format: selectedFormat.value,
      destination: destination.value,
      options: {},
    })

    emit('submitted', jobId)
    isOpen.value = false
  }
  catch (error) {
    console.error('Backup failed:', error)
  }
}
</script>

<template>
  <Dialog v-model:open="isOpen">
    <DialogPortal>
      <DialogOverlay class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 bg-background/80 inset-0 fixed z-50" />
      <DialogContent class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right border-l bg-card flex flex-col shadow-xl duration-200 focus:outline-none !rounded-none !h-full !w-full !translate-x-0 !translate-y-0 !inset-y-0 !left-auto !right-0 !fixed sm:!max-w-[40vw]">
        <div class="px-6 py-5 border-b border-border/40 shrink-0">
          <DialogTitle class="text-xl">
            {{ t('transfer.surface.backupDatabase.title', 'Backup database') }}
          </DialogTitle>
          <DialogDescription class="text-sm mt-1">
            {{ t('transfer.surface.backupDatabase.subtitle', 'Back up selected tables from this database') }}
          </DialogDescription>
        </div>

        <div class="px-6 py-4 border-b border-border/10 bg-muted/10 shrink-0">
          <WizardStepper :steps="steps" :current-step="currentStep" />
        </div>

        <div class="p-6 flex-1 min-h-0 overflow-y-auto">
          <!-- Step 0: Tables -->
          <div v-show="currentStep === 0" class="flex flex-col h-full">
            <MultiTableSelector
              v-model:selected-tables="selectedTables"
              :connection-id="connectionId"
              :database="database"
            />
          </div>

          <!-- Step 1: Options -->
          <div v-show="currentStep === 1" class="space-y-6">
            <div class="space-y-2">
              <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">
                Format
              </Label>
              <Select v-model="selectedFormat">
                <SelectTrigger class="h-9">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="csv">
                    CSV (.csv)
                  </SelectItem>
                  <SelectItem value="jsonl">
                    JSONL (.jsonl)
                  </SelectItem>
                  <SelectItem value="sql">
                    SQL (.sql)
                  </SelectItem>
                  <SelectItem value="excel">
                    Excel (.xlsx)
                  </SelectItem>
                </SelectContent>
              </Select>
              <p class="text-[11px] text-muted-foreground pt-1">
                Data will be exported to one file per table.
              </p>
            </div>
          </div>

          <!-- Step 2: Destination -->
          <div v-show="currentStep === 2" class="space-y-6">
            <div class="space-y-3">
              <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">
                Target Directory
              </Label>
              <div class="flex gap-2">
                <Input
                  v-model="destination"
                  readonly
                  class="text-xs font-mono cursor-default"
                  placeholder="Select a folder..."
                />
                <Button variant="secondary" @click="pickDestination">
                  Browse...
                </Button>
              </div>
              <p class="text-[11px] text-muted-foreground">
                A folder named <code class="px-1 rounded bg-muted">{{ database }}</code> will be created here.
              </p>
            </div>
          </div>

          <!-- Step 3: Review -->
          <div v-show="currentStep === 3" class="space-y-6">
            <div class="text-sm border border-border/40 rounded-md divide-border/40 divide-y">
              <div class="px-4 py-3 bg-muted/10 flex justify-between">
                <span class="text-muted-foreground">Database</span>
                <span class="font-medium font-mono">{{ database }}</span>
              </div>
              <div class="px-4 py-3 flex justify-between">
                <span class="text-muted-foreground">Tables selected</span>
                <span class="font-medium">{{ selectedTables.length }}</span>
              </div>
              <div class="px-4 py-3 bg-muted/10 flex justify-between">
                <span class="text-muted-foreground">Format</span>
                <span class="font-medium uppercase">{{ selectedFormat }}</span>
              </div>
              <div class="px-4 py-3 flex justify-between">
                <span class="text-muted-foreground">Destination</span>
                <span class="text-xs font-medium font-mono text-right max-w-[200px] truncate" :title="destination">{{ destination || '-' }}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="px-6 py-4 border-t border-border/40 bg-card flex shrink-0 items-center justify-between">
          <Button variant="ghost" @click="isOpen = false">
            {{ t('transfer.surface.backupDatabase.cancel', 'Cancel') }}
          </Button>
          <div class="flex gap-2">
            <Button
              v-if="currentStep > 0"
              variant="outline"
              @click="currentStep--"
            >
              Back
            </Button>
            <Button
              v-if="currentStep < 3"
              :disabled="currentStep === 0 && selectedTables.length === 0"
              @click="currentStep++"
            >
              Next
            </Button>
            <Button
              v-if="currentStep === 3"
              :disabled="!destination"
              @click="handleBackup"
            >
              {{ t('transfer.surface.backupDatabase.submit', 'Start backup') }}
            </Button>
          </div>
        </div>
      </DialogContent>
    </DialogPortal>
  </Dialog>
</template>
