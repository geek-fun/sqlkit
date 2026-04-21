<script setup lang="ts">
const props = defineProps<{
  steps: string[]
  currentStep: number
}>()
</script>

<template>
  <div class="mb-8 flex w-full items-start">
    <div
      v-for="(step, index) in steps"
      :key="index"
      class="flex flex-1 flex-col items-center relative"
    >
      <!-- Connecting Line (extends to next step) -->
      <div
        v-if="index < props.steps.length - 1"
        class="h-[2px] w-full transition-colors left-1/2 top-4 absolute -translate-y-1/2"
        :class="[
          index < props.currentStep
            ? 'bg-primary'
            : 'bg-muted',
        ]"
      />

      <!-- Step Circle -->
      <div
        class="text-sm font-medium rounded-full flex h-8 w-8 transition-colors items-center justify-center relative z-10"
        :class="[
          index <= props.currentStep
            ? 'bg-primary text-primary-foreground shadow-sm ring-4 ring-background'
            : 'bg-muted text-muted-foreground ring-4 ring-background',
          index === props.currentStep && 'ring-primary/20 ring-offset-2 ring-offset-background',
        ]"
      >
        {{ index + 1 }}
      </div>

      <!-- Step Label -->
      <div
        class="text-xs tracking-tight mt-3 text-center transition-colors"
        :class="[
          index <= props.currentStep ? 'text-foreground font-medium' : 'text-muted-foreground',
        ]"
      >
        {{ step }}
      </div>
    </div>
  </div>
</template>
