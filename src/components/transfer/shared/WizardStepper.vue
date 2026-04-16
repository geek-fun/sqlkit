<script setup lang="ts">
const props = defineProps<{
  steps: string[]
  currentStep: number
}>()
</script>

<template>
  <div class="mb-6 flex gap-2 items-center">
    <div
      v-for="(step, index) in steps"
      :key="index"
      class="flex-1"
    >
      <div class="flex items-center">
        <div
          class="text-sm font-medium rounded-full flex h-8 w-8 transition-colors items-center justify-center"
          :class="[
            index <= props.currentStep
              ? 'bg-primary text-primary-foreground'
              : 'bg-muted text-muted-foreground',
          ]"
        >
          {{ index + 1 }}
        </div>
        <div
          v-if="index < props.steps.length - 1"
          class="mx-2 flex-1 h-1 transition-colors"
          :class="[
            index < props.currentStep
              ? 'bg-primary'
              : 'bg-muted',
          ]"
        />
      </div>
      <div
        class="text-sm mt-2 text-center" :class="[
          index === props.currentStep ? 'text-foreground font-medium' : 'text-muted-foreground',
        ]"
      >
        {{ step }}
      </div>
    </div>
  </div>
</template>
