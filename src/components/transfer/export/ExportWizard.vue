<script setup lang="ts">
import type { ExportRequest } from '@/types/transfer'

import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'

import { useTransferStore } from '@/store/transferStore'
import WizardStepper from '../shared/WizardStepper.vue'
import ExportExecuteStep from './ExportExecuteStep.vue'
import ExportFormatStep from './ExportFormatStep.vue'
import ExportPreviewStep from './ExportPreviewStep.vue'

import ExportSourceStep from './ExportSourceStep.vue'

const { t } = useI18n()
const transferStore = useTransferStore()

const steps = computed(() => [t('transfer.export.step.source'), t('transfer.export.step.format'), t('transfer.export.step.preview'), t('transfer.export.step.execute')])
const currentStep = ref(0)

const isRunning = computed(() => transferStore.isRunning)

const canGoBack = computed(() => currentStep.value > 0 && !isRunning.value)

const canGoNext = computed(() => {
  if (currentStep.value === 0) {
    return !!transferStore.exportRequest.connectionId
      && !!transferStore.exportRequest.source?.table
      && transferStore.exportRequest.source?.columns?.length > 0
  }
  if (currentStep.value === 1) {
    return !!transferStore.exportRequest.format
  }
  if (currentStep.value === 2) {
    return !!transferStore.exportRequest.outputPath
  }
  return false
})

function handleBack() {
  if (canGoBack.value) {
    currentStep.value--
  }
}

function handleNext() {
  if (canGoNext.value && currentStep.value < steps.value.length - 1) {
    currentStep.value++
  }
}

function handleExecute() {
  currentStep.value = 3
}

function handleSourceUpdate(value: Partial<ExportRequest>) {
  transferStore.exportRequest = { ...transferStore.exportRequest, ...value }
}

watch(currentStep, (val) => {
  transferStore.exportStep = val
})
</script>

<template>
  <Card>
    <CardContent class="pt-6">
      <WizardStepper :steps="steps" :current-step="currentStep" />

      <div class="mt-6 min-h-400px">
        <ExportSourceStep
          v-if="currentStep === 0"
          :model-value="transferStore.exportRequest"
          @update:model-value="handleSourceUpdate"
        />
        <ExportFormatStep v-if="currentStep === 1" />
        <ExportPreviewStep v-if="currentStep === 2" @execute="handleExecute" />
        <ExportExecuteStep v-if="currentStep === 3" />
      </div>

      <div v-if="currentStep < 3" class="mt-6 flex gap-2 justify-end">
        <Button
          variant="outline"
          :disabled="!canGoBack"
          @click="handleBack"
        >
          {{ t('transfer.export.back') }}
        </Button>
        <Button
          :disabled="!canGoNext"
          @click="handleNext"
        >
          {{ t('transfer.export.next') }}
        </Button>
      </div>
    </CardContent>
  </Card>
</template>
