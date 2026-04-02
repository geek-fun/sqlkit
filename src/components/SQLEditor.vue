<script setup lang="ts">
import type * as monaco from 'monaco-editor'
import type { MonacoEditorOptions, SQLDialect } from '@/composables/useMonacoEditor'
import { onMounted, ref, watch } from 'vue'
import { ProgressBar } from '@/components/ui/progress'
import { useMonacoEditor } from '@/composables/useMonacoEditor'
import { useTheme } from '@/composables/useTheme'

type Props = {
  modelValue?: string
  dialect?: SQLDialect
  readOnly?: boolean
  minimap?: boolean
  fontSize?: number
  tabSize?: number
  showLineNumbers?: boolean
  wordWrap?: boolean
  height?: string
  placeholder?: string
  isExecuting?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  dialect: 'sql',
  readOnly: false,
  minimap: true,
  fontSize: 14,
  tabSize: 2,
  showLineNumbers: true,
  wordWrap: true,
  height: '400px',
  placeholder: '-- Enter your SQL query here\n-- Press Ctrl+Enter to execute',
  isExecuting: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'execute': [details: { query: string, cursorPosition?: { lineNumber: number, column: number }, selection?: { startLineNumber: number, startColumn: number, endLineNumber: number, endColumn: number } }]
  'save': [query: string]
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
  showLineNumbers: props.showLineNumbers,
  wordWrap: props.wordWrap,
}

const { initEditor, getValue, setValue, updateTheme, updateOptions } = useMonacoEditor(
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
      emit('execute', event.detail)
    }) as EventListener)

    // Listen for save command
    editorContainer.value?.addEventListener('save-query', ((event: CustomEvent) => {
      emit('save', event.detail.query)
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

// Watch for editor option changes
watch(() => props.showLineNumbers, val => updateOptions({ showLineNumbers: val }))
watch(() => props.wordWrap, val => updateOptions({ wordWrap: val }))
watch(() => props.fontSize, val => updateOptions({ fontSize: val }))
watch(() => props.tabSize, val => updateOptions({ tabSize: val }))
watch(() => props.minimap, val => updateOptions({ minimap: val }))

// Expose methods for parent component
defineExpose({
  getValue,
  setValue,
})
</script>

<template>
  <div class="sql-editor-wrapper" :style="{ height: props.height }">
    <ProgressBar :visible="props.isExecuting" />
    <div
      ref="editorContainer"
      class="sql-editor-container"
      :style="{ height: '100%', width: '100%' }"
    />
  </div>
</template>

<style scoped>
.sql-editor-wrapper {
  position: relative;
  overflow: hidden;
}
</style>

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
