<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'

type Props = {
  bottomOpen: boolean
  bottomMaxPercent?: number
}

const props = withDefaults(defineProps<Props>(), {
  bottomMaxPercent: 50,
})

const emit = defineEmits<{
  (e: 'resize'): void
}>()

const STORAGE_KEY = 'sqlkit.sidebar.savedQueries.height'
const MIN_HEIGHT = 60
const containerEl = ref<HTMLElement | null>(null)
const containerHeight = ref(0)
const bottomHeight = ref(280)
const isDragging = ref(false)

function updateContainerHeight() {
  if (containerEl.value) {
    containerHeight.value = containerEl.value.clientHeight
  }
}

function maxHeight() {
  return Math.max(MIN_HEIGHT, Math.floor(containerHeight.value * props.bottomMaxPercent / 100))
}

function clampedBottomHeight() {
  const max = maxHeight()
  return Math.min(Math.max(bottomHeight.value, MIN_HEIGHT), max)
}

function startDrag(e: MouseEvent) {
  e.preventDefault()
  isDragging.value = true
  document.addEventListener('mousemove', handleDrag)
  document.addEventListener('mouseup', stopDrag)
}

function handleDrag(e: MouseEvent) {
  if (!isDragging.value || !containerEl.value)
    return
  const rect = containerEl.value.getBoundingClientRect()
  const newHeight = rect.bottom - e.clientY
  const max = maxHeight()
  bottomHeight.value = Math.min(Math.max(newHeight, MIN_HEIGHT), max)
}

function stopDrag() {
  if (isDragging.value) {
    isDragging.value = false
    localStorage.setItem(STORAGE_KEY, String(bottomHeight.value))
    emit('resize')
  }
  document.removeEventListener('mousemove', handleDrag)
  document.removeEventListener('mouseup', stopDrag)
}

onMounted(() => {
  const saved = localStorage.getItem(STORAGE_KEY)
  if (saved) {
    const h = Number.parseInt(saved, 10)
    if (!Number.isNaN(h) && h >= MIN_HEIGHT) {
      bottomHeight.value = h
    }
  }
  updateContainerHeight()
  window.addEventListener('resize', updateContainerHeight)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateContainerHeight)
  document.removeEventListener('mousemove', handleDrag)
  document.removeEventListener('mouseup', stopDrag)
})

watch(() => props.bottomOpen, (open) => {
  if (open) {
    updateContainerHeight()
  }
})
</script>

<template>
  <div ref="containerEl" class="flex flex-col h-full overflow-hidden">
    <div class="flex-1 min-h-0 overflow-auto">
      <slot name="top" />
    </div>
    <template v-if="bottomOpen">
      <div
        class="shrink-0 h-1 cursor-ns-resize transition-colors hover:bg-primary/20"
        :class="{ 'bg-primary/20': isDragging }"
        @mousedown="startDrag"
      />
      <div
        class="shrink-0 overflow-auto"
        :style="{ height: `${clampedBottomHeight()}px` }"
      >
        <slot name="bottom" />
      </div>
    </template>
  </div>
</template>
