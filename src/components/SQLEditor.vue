<script setup lang="ts">
import type * as monaco from 'monaco-editor'
import type { MonacoEditorOptions, SQLDialect } from '@/composables/useMonacoEditor'
import { defineEmits, onMounted, ref, watch } from 'vue'
import { useMonacoEditor } from '@/composables/useMonacoEditor'
import { useTheme } from '@/composables/useTheme'

interface Props {
  modelValue?: string
  dialect?: SQLDialect
  readOnly?: boolean
  minimap?: boolean
  fontSize?: number
  tabSize?: number
  height?: string
  placeholder?: string
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  dialect: 'sql',
  readOnly: false,
  minimap: true,
  fontSize: 14,
  tabSize: 2,
  height: '400px',
  placeholder: '-- Enter your SQL query here\n-- Press Ctrl+Enter to execute',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'execute': [query: string]
}>()

const editorContainer = ref<HTMLElement | null>(null)
const editorValue = ref(props.modelValue || props.placeholder)
const { isDark } = useTheme()

const editorOptions: MonacoEditorOptions = {
  language: props.dialect,
  readOnly: props.readOnly,
  minimap: props.minimap,
  fontSize: props.fontSize,
  tabSize: props.tabSize,
}

const { initEditor, getValue, setValue, updateTheme } = useMonacoEditor(
  editorContainer,
  editorValue,
  editorOptions,
)

let editor: monaco.editor.IStandaloneCodeEditor | null = null

onMounted(() => {
  editor = initEditor() ?? null

  if (editor) {
    // Listen for content changes
    editor.onDidChangeModelContent(() => {
      const value = getValue()
      emit('update:modelValue', value)
    })

    // Listen for execute command
    editorContainer.value?.addEventListener('execute-query', ((event: CustomEvent) => {
      emit('execute', event.detail.query)
    }) as EventListener)
  }
})

// Watch for external value changes
watch(() => props.modelValue, (newValue) => {
  if (newValue !== getValue()) {
    setValue(newValue || '')
  }
})

// Watch for theme changes
watch(isDark, (dark) => {
  updateTheme(dark)
})

// Expose methods for parent component
defineExpose({
  getValue,
  setValue,
})
</script>

<template>
  <div class="sql-editor-wrapper" :style="{ height: props.height }">
    <div
      ref="editorContainer"
      class="sql-editor-container"
      :style="{ height: '100%', width: '100%' }"
    />
  </div>
</template>

<style scoped>
.sql-editor-wrapper {
  border: 1px solid var(--border-color, #e5e7eb);
  border-radius: 0.375rem;
  overflow: hidden;
  background-color: var(--background-color, #ffffff);
}

.dark .sql-editor-wrapper {
  --border-color: #374151;
  --background-color: #1e1e1e;
}

.sql-editor-container {
  position: relative;
}
</style>
