<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'

import { useTransferStore } from '@/store/transferStore'
import WizardStepper from '../shared/WizardStepper.vue'
import ImportExecuteStep from './ImportExecuteStep.vue'
import ImportFileStep from './ImportFileStep.vue'
import ImportMappingStep from './ImportMappingStep.vue'

import ImportOptionsStep from './ImportOptionsStep.vue'

const transferStore = useTransferStore()

const steps = ['File', 'Target & Mapping', 'Options', 'Import']
const currentStep = ref(0)

const isRunning = computed(() => transferStore.isRunning)

const canGoBack = computed(() => currentStep.value > 0 && !isRunning.value)

const canGoNext = computed(() => {
  if (currentStep.value === 0) {
    return !!transferStore.importRequest.filePath
  }
  if (currentStep.value === 1) {
    return !!transferStore.importRequest.connectionId
      && !!transferStore.importRequest.table
  }
  if (currentStep.value === 2) {
    return true
  }
  return false
})

function handleBack() {
  if (canGoBack.value) {
    currentStep.value--
  }
}

function handleNext() {
  if (canGoNext.value && currentStep.value < steps.length - 1) {
    currentStep.value++
  }
}

watch(currentStep, (val) => {
  transferStore.importStep = val
})
</script>

<template>
  <Card>
    <CardContent class="pt-6">
      <WizardStepper :steps="steps" :current-step="currentStep" />

      <div class="mt-6 min-h-400px">
        <ImportFileStep v-if="currentStep === 0" />
        <ImportMappingStep v-if="currentStep === 1" />
        <ImportOptionsStep v-if="currentStep === 2" />
        <ImportExecuteStep v-if="currentStep === 3" />
      </div>

      <div v-if="currentStep < 3" class="mt-6 flex gap-2 justify-end">
        <Button
          variant="outline"
          :disabled="!canGoBack"
          @click="handleBack"
        >
          Back
        </Button>
        <Button
          :disabled="!canGoNext"
          @click="handleNext"
        >
          Next
        </Button>
      </div>
    </CardContent>
  </Card>
</template>
