<script setup lang="ts">
import type { MonacoEditorOptions, SQLDialect } from '@/composables/useMonacoEditor'
import type { StatementToExecute } from '@/composables/useSqlStatements'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { ProgressBar } from '@/components/ui/progress'
import { useMonacoEditor } from '@/composables/useMonacoEditor'
import { usePlatform } from '@/composables/usePlatform'
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
  placeholder: '',
  isExecuting: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'execute': [details: StatementToExecute]
  'statementNotFound': []
  'save': [query: string]
}>()

const editorContainer = ref<HTMLElement | null>(null)
const { isDark } = useTheme()
const { modifierKey } = usePlatform()

const cmdKey = modifierKey

const defaultPlaceholder = computed(() =>
  `-- Enter your SQL query here\n-- Press ${modifierKey.value}Enter to execute`,
)

const editorValue = ref(props.modelValue || props.placeholder || defaultPlaceholder.value)

const contextMenuVisible = ref(false)
const contextMenuPosition = ref({ x: 0, y: 0 })
const contextMenuLine = ref<number | null>(null)

const isSettingValueFromProp = ref(false)

const editorOptions: MonacoEditorOptions = {
  language: props.dialect,
  readOnly: props.readOnly,
  minimap: props.minimap,
  fontSize: props.fontSize,
  tabSize: props.tabSize,
  showLineNumbers: props.showLineNumbers,
  wordWrap: props.wordWrap,
}

const { initEditor, getValue, setValue, updateTheme, updateOptions, executeAtLine, getStatementTextAtLine } = useMonacoEditor(
  editorContainer,
  editorValue,
  editorOptions,
)

let editorInstance: ReturnType<typeof initEditor> | null = null

function hideContextMenu() {
  contextMenuVisible.value = false
  contextMenuLine.value = null
}

function handleDocumentClick(event: MouseEvent) {
  if (!contextMenuVisible.value)
    return
  const target = event.target as HTMLElement
  if (!target.closest('.sql-context-menu'))
    hideContextMenu()
}

function handleDocumentKeydown(event: KeyboardEvent) {
  if (contextMenuVisible.value && event.key === 'Escape')
    hideContextMenu()
}

const CONTEXT_MENU_WIDTH = 180
const CONTEXT_MENU_HEIGHT = 100

function clampMenuPosition(x: number, y: number): { x: number, y: number } {
  const vw = window.innerWidth
  const vh = window.innerHeight
  return {
    x: Math.min(x, vw - CONTEXT_MENU_WIDTH),
    y: Math.min(y, vh - CONTEXT_MENU_HEIGHT),
  }
}

onMounted(() => {
  editorInstance = initEditor({
    onExecuteQuery: (result: StatementToExecute) => {
      emit('execute', result)
    },
    onStatementNotFound: () => {
      emit('statementNotFound')
    },
    onSave: (query: string) => {
      emit('save', query)
    },
    onGutterContextMenu: (lineNumber: number, x: number, y: number) => {
      const clamped = clampMenuPosition(x, y)
      contextMenuPosition.value = clamped
      contextMenuLine.value = lineNumber
      contextMenuVisible.value = true
    },
  })

  if (editorInstance) {
    editorInstance.onDidChangeModelContent(() => {
      if (isSettingValueFromProp.value)
        return
      emit('update:modelValue', getValue())
    })
  }

  document.addEventListener('click', handleDocumentClick)
  document.addEventListener('keydown', handleDocumentKeydown)
})

onUnmounted(() => {
  document.removeEventListener('click', handleDocumentClick)
  document.removeEventListener('keydown', handleDocumentKeydown)
})

watch(() => props.modelValue, (newValue) => {
  if (newValue !== getValue()) {
    isSettingValueFromProp.value = true
    setValue(newValue || '')
    isSettingValueFromProp.value = false
  }
})

watch(isDark, dark => updateTheme(dark))

watch(() => props.showLineNumbers, val => updateOptions({ showLineNumbers: val }))
watch(() => props.wordWrap, val => updateOptions({ wordWrap: val }))
watch(() => props.fontSize, val => updateOptions({ fontSize: val }))
watch(() => props.tabSize, val => updateOptions({ tabSize: val }))
watch(() => props.minimap, val => updateOptions({ minimap: val }))

function handleContextMenuExecute() {
  if (contextMenuLine.value === null)
    return
  const line = contextMenuLine.value
  hideContextMenu()
  executeAtLine(line)
}

function handleContextMenuFormat() {
  hideContextMenu()
  editorInstance?.trigger('contextmenu', 'editor.action.formatDocument', {})
}

async function handleContextMenuCopy() {
  if (contextMenuLine.value === null)
    return
  const line = contextMenuLine.value
  hideContextMenu()
  const statementText = getStatementTextAtLine(line)
  if (statementText && navigator.clipboard)
    await navigator.clipboard.writeText(statementText)
}

defineExpose({ getValue, setValue })
</script>

<template>
  <div class="sql-editor-wrapper" :style="{ height: props.height }">
    <ProgressBar :visible="props.isExecuting" />
    <div
      ref="editorContainer"
      class="sql-editor-container"
      :style="{ height: '100%', width: '100%' }"
    />
    <div
      v-if="contextMenuVisible"
      class="sql-context-menu"
      :style="{ top: `${contextMenuPosition.y}px`, left: `${contextMenuPosition.x}px` }"
      @click.stop
    >
      <ul>
        <li @click="handleContextMenuExecute">
          <span>Execute</span>
          <span class="shortcut">{{ cmdKey }}Enter</span>
        </li>
        <li @click="handleContextMenuFormat">
          <span>Format</span>
          <span class="shortcut">{{ cmdKey }}I</span>
        </li>
        <li @click="handleContextMenuCopy">
          <span>Copy</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.sql-editor-wrapper {
  position: relative;
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

.sql-context-menu {
  position: fixed;
  background-color: hsl(var(--background));
  border: 1px solid hsl(var(--border));
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  z-index: 10000;
  border-radius: 4px;
  overflow: hidden;
}

.sql-context-menu ul {
  list-style: none;
  margin: 0;
  padding: 4px 0;
  min-width: 160px;
}

.sql-context-menu li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 13px;
}

.sql-context-menu li .shortcut {
  font-size: 11px;
  opacity: 0.5;
  white-space: nowrap;
}

.sql-context-menu li:hover {
  background: hsl(var(--accent));
}

:deep(.sql-execute-decoration) {
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

:deep(.sql-execute-decoration::before) {
  content: '';
  display: block;
  width: 0;
  height: 0;
  border-style: solid;
  border-width: 6px 0 6px 10px;
  border-color: transparent transparent transparent hsl(var(--primary));
}

:deep(.sql-execute-decoration:hover::before) {
  border-color: transparent transparent transparent hsl(var(--primary) / 0.7);
}
</style>
