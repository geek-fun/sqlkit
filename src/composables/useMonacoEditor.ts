import type { Ref } from 'vue'
import * as monaco from 'monaco-editor'
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import { onBeforeUnmount } from 'vue'
import { useTheme } from './useTheme'

// Configure Monaco Editor workers for Vite
globalThis.MonacoEnvironment = {
  getWorker(_: unknown, _label: string) {
    return new EditorWorker()
  },
}

export type SQLDialect = 'sql' | 'mysql' | 'pgsql' | 'mssql' | 'plsql' | 'sqlite'

export interface MonacoEditorOptions {
  language?: SQLDialect
  readOnly?: boolean
  minimap?: boolean
  fontSize?: number
  tabSize?: number
  showLineNumbers?: boolean
  wordWrap?: boolean
}

// SQL keywords for auto-completion
const SQL_KEYWORDS = [
  'SELECT',
  'FROM',
  'WHERE',
  'INSERT',
  'UPDATE',
  'DELETE',
  'CREATE',
  'ALTER',
  'DROP',
  'TABLE',
  'INDEX',
  'VIEW',
  'DATABASE',
  'SCHEMA',
  'JOIN',
  'INNER',
  'LEFT',
  'RIGHT',
  'OUTER',
  'ON',
  'AS',
  'AND',
  'OR',
  'NOT',
  'NULL',
  'IS',
  'IN',
  'BETWEEN',
  'LIKE',
  'ORDER',
  'BY',
  'GROUP',
  'HAVING',
  'LIMIT',
  'OFFSET',
  'UNION',
  'DISTINCT',
  'COUNT',
  'SUM',
  'AVG',
  'MAX',
  'MIN',
  'CAST',
  'CASE',
  'WHEN',
  'THEN',
  'ELSE',
  'END',
  'PRIMARY',
  'KEY',
  'FOREIGN',
  'REFERENCES',
  'CONSTRAINT',
  'UNIQUE',
  'CHECK',
  'DEFAULT',
  'AUTO_INCREMENT',
  'CASCADE',
  'SET',
  'VALUES',
  'INTO',
  'BEGIN',
  'COMMIT',
  'ROLLBACK',
  'TRANSACTION',
  'SAVEPOINT',
  'TRUNCATE',
  'GRANT',
  'REVOKE',
  'WITH',
  'RECURSIVE',
  'WINDOW',
  'PARTITION',
  'OVER',
  'ROW_NUMBER',
  'RANK',
  'DENSE_RANK',
]

// SQL data types
const SQL_TYPES = [
  'INT',
  'INTEGER',
  'BIGINT',
  'SMALLINT',
  'TINYINT',
  'DECIMAL',
  'NUMERIC',
  'FLOAT',
  'REAL',
  'DOUBLE',
  'CHAR',
  'VARCHAR',
  'TEXT',
  'NCHAR',
  'NVARCHAR',
  'NTEXT',
  'DATE',
  'TIME',
  'DATETIME',
  'TIMESTAMP',
  'YEAR',
  'BOOLEAN',
  'BOOL',
  'BINARY',
  'VARBINARY',
  'BLOB',
  'CLOB',
  'JSON',
  'UUID',
  'SERIAL',
  'BIGSERIAL',
]

// SQL functions
const SQL_FUNCTIONS = [
  'CONCAT',
  'SUBSTRING',
  'UPPER',
  'LOWER',
  'TRIM',
  'LTRIM',
  'RTRIM',
  'LENGTH',
  'REPLACE',
  'COALESCE',
  'NULLIF',
  'IFNULL',
  'NOW',
  'CURRENT_DATE',
  'CURRENT_TIME',
  'CURRENT_TIMESTAMP',
  'DATE_ADD',
  'DATE_SUB',
  'DATEDIFF',
  'EXTRACT',
  'TO_CHAR',
  'TO_DATE',
  'TO_NUMBER',
  'ROUND',
  'CEIL',
  'FLOOR',
  'ABS',
  'SIGN',
  'MOD',
  'POWER',
  'SQRT',
  'EXP',
  'LN',
  'LOG',
]

export function useMonacoEditor(
  containerRef: Ref<HTMLElement | null>,
  initialValue: Ref<string>,
  options: MonacoEditorOptions = {},
) {
  let editor: monaco.editor.IStandaloneCodeEditor | null = null
  let completionProvider: monaco.IDisposable | null = null
  const { isDark } = useTheme()

  const initEditor = () => {
    if (!containerRef.value)
      return

    // Set up auto-completion provider
    completionProvider = monaco.languages.registerCompletionItemProvider('sql', {
      provideCompletionItems: (model, position) => {
        const word = model.getWordUntilPosition(position)
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        }

        const suggestions: monaco.languages.CompletionItem[] = [
          ...SQL_KEYWORDS.map(keyword => ({
            label: keyword,
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: keyword,
            range,
          })),
          ...SQL_TYPES.map(type => ({
            label: type,
            kind: monaco.languages.CompletionItemKind.TypeParameter,
            insertText: type,
            range,
          })),
          ...SQL_FUNCTIONS.map((func) => {
            // Some functions don't require parentheses in all SQL dialects
            const noParenFunctions = ['NOW', 'CURRENT_DATE', 'CURRENT_TIME', 'CURRENT_TIMESTAMP']
            const insertText = noParenFunctions.includes(func) ? func : `${func}()`
            return {
              label: func,
              kind: monaco.languages.CompletionItemKind.Function,
              insertText,
              range,
            }
          }),
        ]

        return { suggestions }
      },
    })

    // Create editor instance
    editor = monaco.editor.create(containerRef.value, {
      value: initialValue.value,
      language: options.language || 'sql',
      theme: isDark.value ? 'vs-dark' : 'vs',
      automaticLayout: true,
      readOnly: options.readOnly || false,
      minimap: {
        enabled: options.minimap !== false,
      },
      fontSize: options.fontSize || 14,
      tabSize: options.tabSize || 2,
      lineNumbers: options.showLineNumbers === false ? 'off' : 'on',
      wordWrap: options.wordWrap === false ? 'off' : 'on',
      roundedSelection: false,
      contextmenu: true,
      formatOnPaste: true,
      formatOnType: true,
      suggest: {
        showKeywords: true,
        showSnippets: true,
      },
    })

    // Add keyboard shortcuts
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
      () => {
        // Get cursor position
        const position = editor?.getPosition()
        const selection = editor?.getSelection()

        // This will be handled by parent component
        const event = new CustomEvent('execute-query', {
          detail: {
            query: editor?.getValue(),
            cursorPosition: position,
            selection: selection
              ? {
                  startLineNumber: selection.startLineNumber,
                  startColumn: selection.startColumn,
                  endLineNumber: selection.endLineNumber,
                  endColumn: selection.endColumn,
                }
              : undefined,
          },
        })
        containerRef.value?.dispatchEvent(event)
      },
    )

    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS,
      () => {
        // This will be handled by parent component
        const event = new CustomEvent('save-query', {
          detail: { query: editor?.getValue() },
        })
        containerRef.value?.dispatchEvent(event)
      },
    )

    return editor
  }

  const getValue = (): string => {
    return editor?.getValue() || ''
  }

  const setValue = (value: string) => {
    editor?.setValue(value)
  }

  const updateOptions = (newOpts: Partial<MonacoEditorOptions>) => {
    if (!editor)
      return
    editor.updateOptions({
      ...(newOpts.showLineNumbers !== undefined && { lineNumbers: newOpts.showLineNumbers ? 'on' : 'off' }),
      ...(newOpts.fontSize !== undefined && { fontSize: newOpts.fontSize }),
      ...(newOpts.tabSize !== undefined && { tabSize: newOpts.tabSize }),
      ...(newOpts.minimap !== undefined && { minimap: { enabled: newOpts.minimap } }),
      ...(newOpts.wordWrap !== undefined && { wordWrap: newOpts.wordWrap ? 'on' : 'off' }),
    })
  }

  const updateTheme = (dark: boolean) => {
    monaco.editor.setTheme(dark ? 'vs-dark' : 'vs')
  }

  const dispose = () => {
    completionProvider?.dispose()
    editor?.dispose()
  }

  onBeforeUnmount(() => {
    dispose()
  })

  return {
    initEditor,
    getValue,
    setValue,
    updateTheme,
    updateOptions,
    dispose,
  }
}
