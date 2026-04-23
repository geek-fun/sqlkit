<!--
  Visual Role: Progress indicator for multi-step workflows.
  Uses small circles, hairline connectors, and muted text for upcoming steps.
-->
<script setup lang="ts">
const props = defineProps<{
  steps: string[]
  currentStep: number
}>()
</script>

<template>
  <div class="mb-6 flex w-full items-start">
    <div
      v-for="(step, index) in steps"
      :key="index"
      class="flex flex-1 flex-col items-center relative"
    >
      <!-- Connecting Line (extends to next step) -->
      <div
        v-if="index < props.steps.length - 1"
        class="h-px w-full transition-colors left-1/2 top-3 absolute -translate-y-1/2"
        :class="[
          index < props.currentStep
            ? 'bg-primary/80'
            : 'bg-border/60',
        ]"
      />

      <!-- Step Circle -->
      <div
        class="text-[11px] font-mono rounded-full flex h-6 w-6 ring-4 ring-background transition-colors items-center justify-center relative z-10"
        :class="[
          index === props.currentStep
            ? 'bg-primary text-primary-foreground shadow-none'
            : index < props.currentStep
              ? 'bg-primary/80 text-primary-foreground'
              : 'bg-muted text-muted-foreground border border-border',
        ]"
      >
        {{ index + 1 }}
      </div>

      <!-- Step Label -->
      <div
        class="text-[11px] mt-2 text-center transition-colors"
        :class="[
          index === props.currentStep ? 'text-foreground font-medium' : index < props.currentStep ? 'text-foreground font-normal' : 'text-muted-foreground',
        ]"
      >
        {{ step }}
      </div>
    </div>
  </div>
</template>
