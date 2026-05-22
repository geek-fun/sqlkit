<script setup lang="ts">
import type { ExportFormat, ObjectSelection } from '@/types/transfer'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import ServerObjectTree from '@/components/transfer/ServerObjectTree.vue'
import WizardStepper from '@/components/transfer/shared/WizardStepper.vue'

import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useTransferStore } from '@/store/transferStore'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const transferStore = useTransferStore()

const serverId = computed(() => route.params.serverId as string)

const steps = computed(() => [
  t('transfer.surface.backupServer.options', 'Backup options'),
  t('transfer.surface.backupServer.destination', 'Destination'),
  t('transfer.surface.backupServer.parallelism', 'Parallel jobs'),
  t('transfer.surface.backupServer.review', 'Review'),
])

const currentStep = ref(0)
const selectedFormat = ref<ExportFormat>('csv')
const destination = ref('')
const parallelism = ref(2)

const selection = ref<ObjectSelection>({
  serverId: serverId.value,
  databases: [],
  schemas: {},
  tables: {},
})

async function pickDestination() {
  const path = await open({ directory: true })
  if (path) {
    destination.value = typeof path === 'string' ? path : path[0]
  }
}

async function handleBackup() {
  if (!destination.value)
    return

  try {
    await transferStore.startBackupServer({
      connectionId: serverId.value,
      name: t('transfer.surface.backupServer.jobName'),
      selection: selection.value,
      format: selectedFormat.value,
      destination: destination.value,
      options: { parallelism: parallelism.value },
    })

    router.push('/transfer')
  }
  catch (error) {
    console.error('Failed to start server backup:', error)
  }
}

const tableCount = computed(() => {
  return Object.values(selection.value.tables).reduce((acc, curr) => acc + curr.length, 0)
})
</script>

<template>
  <div class="bg-background flex flex-col h-full">
    <!-- Header -->
    <header class="px-6 py-4 border-b border-border/40 bg-card flex shrink-0 gap-4 items-center">
      <Button variant="ghost" size="sm" class="p-0 h-8 w-8" @click="router.push('/transfer')">
        <span class="i-carbon-arrow-left text-lg" />
      </Button>
      <div>
        <h1 class="text-xl tracking-tight font-semibold">
          {{ t('transfer.surface.backupServer.title', 'Backup server') }}
        </h1>
        <p class="text-sm text-muted-foreground mt-0.5">
          {{ t('transfer.surface.backupServer.subtitle', 'Back up selected databases, schemas, and tables') }}
        </p>
      </div>
    </header>

    <!-- Main Content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Left: Tree -->
      <div class="border-r border-border/40 bg-muted/5 flex flex-col w-1/2">
        <div class="text-sm font-medium p-4 border-b border-border/10">
          {{ t('transfer.surface.backupServer.selectObjects', 'Select objects') }}
        </div>
        <div class="p-4 flex-1 overflow-auto">
          <ServerObjectTree
            v-model:selection="selection"
            :connection-id="serverId"
          />
        </div>
      </div>

      <!-- Right: Wizard -->
      <div class="bg-card flex flex-col w-1/2">
        <div class="px-8 py-6 border-b border-border/5 shrink-0">
          <WizardStepper :steps="steps" :current-step="currentStep" />
        </div>

        <div class="px-8 py-6 flex-1 overflow-y-auto">
          <!-- Step 0: Options -->
          <div v-show="currentStep === 0" class="max-w-md space-y-6">
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
            </div>
          </div>

          <!-- Step 1: Destination -->
          <div v-show="currentStep === 1" class="max-w-md space-y-6">
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
            </div>
          </div>

          <!-- Step 2: Parallelism -->
          <div v-show="currentStep === 2" class="max-w-md space-y-8">
            <div class="space-y-4">
              <div class="flex items-center justify-between">
                <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">
                  Concurrent Jobs
                </Label>
                <span class="text-sm font-medium font-mono">{{ parallelism }}</span>
              </div>
              <input
                v-model.number="parallelism"
                type="range"
                min="1"
                max="8"
                class="accent-primary w-full"
              >
              <div class="text-xs text-muted-foreground flex justify-between">
                <span>1 (Safest)</span>
                <span>8 (Fastest)</span>
              </div>
            </div>
          </div>

          <!-- Step 3: Review -->
          <div v-show="currentStep === 3" class="max-w-lg space-y-6">
            <div class="text-sm border border-border/40 rounded-md divide-border/40 divide-y">
              <div class="px-4 py-3 bg-muted/10 flex justify-between">
                <span class="text-muted-foreground">Databases selected</span>
                <span class="font-medium">{{ selection.databases.length }}</span>
              </div>
              <div class="px-4 py-3 flex justify-between">
                <span class="text-muted-foreground">Tables selected</span>
                <span class="font-medium">{{ tableCount }}</span>
              </div>
              <div class="px-4 py-3 bg-muted/10 flex justify-between">
                <span class="text-muted-foreground">Format</span>
                <span class="font-medium uppercase">{{ selectedFormat }}</span>
              </div>
              <div class="px-4 py-3 flex justify-between">
                <span class="text-muted-foreground">Parallel jobs</span>
                <span class="font-medium">{{ parallelism }}</span>
              </div>
              <div class="px-4 py-3 bg-muted/10 flex justify-between">
                <span class="text-muted-foreground">Destination</span>
                <span class="text-xs font-medium font-mono text-right max-w-[250px] truncate" :title="destination">{{ destination || '-' }}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="px-8 py-4 border-t border-border/40 bg-card flex shrink-0 items-center justify-between">
          <Button variant="ghost" @click="router.push('/transfer')">
            {{ t('transfer.surface.backupServer.cancel', 'Cancel') }}
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
              :disabled="selection.databases.length === 0"
              @click="currentStep++"
            >
              Next
            </Button>
            <Button
              v-if="currentStep === 3"
              :disabled="!destination"
              @click="handleBackup"
            >
              {{ t('transfer.surface.backupServer.submit', 'Start backup') }}
            </Button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
