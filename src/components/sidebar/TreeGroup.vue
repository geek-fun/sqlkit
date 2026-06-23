<script setup lang="ts">
import { ref } from 'vue'

type Props = {
  label: string
  icon: string
  iconColor: string
  count?: number
  defaultOpen?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  count: undefined,
  defaultOpen: false,
})

const emit = defineEmits<{
  (e: 'toggle', open: boolean): void
}>()

const open = ref(props.defaultOpen ?? false)

function toggle() {
  open.value = !open.value
  emit('toggle', open.value)
}
</script>

<template>
  <div>
    <button
      class="text-xs text-muted-foreground font-medium px-2 py-1 rounded-none flex gap-1.5 w-full cursor-pointer items-center hover:bg-accent/40"
      @click="toggle"
    >
      <span
        class="i-carbon-chevron-right shrink-0 h-3 w-3 transition-transform"
        :class="{ 'rotate-90': open }"
      />
      <span class="shrink-0 h-3.5 w-3.5" :class="[icon, iconColor]" />
      <span class="text-left flex-1 truncate">{{ label }}</span>
      <span
        v-if="count !== undefined"
        class="text-xs text-muted-foreground/70 shrink-0"
      >
        {{ count }}
      </span>
    </button>
    <div v-if="open" class="py-0.5">
      <slot />
    </div>
  </div>
</template>
