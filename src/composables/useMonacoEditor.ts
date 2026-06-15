import type { Ref } from 'vue'
import type { SqlStatement, StatementToExecute } from './useSqlStatements'
import * as monaco from 'monaco-editor'
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import { onBeforeUnmount } from 'vue'
import {
  getSqlGutterDecorations,
  getStatementAtLine,
  getStatementToExecute,
  MOUSE_TARGET_GUTTER_LINE_DECORATIONS,
  parseSqlStatements,
  SQL_EXECUTE_GUTTER_CLASS,
} from './useSqlStatements'
import { useTheme } from './useTheme'

export type { ExecuteSource, StatementToExecute } from './useSqlStatements'

// Configure Monaco Editor workers for Vite
globalThis.MonacoEnvironment = {
  getWorker(_: unknown, _label: string) {
    return new EditorWorker()
  },
}

export type SQLDialect = 'sql' | 'mysql' | 'pgsql' | 'mssql' | 'plsql' | 'sqlite'

export type MonacoEditorOptions = {
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

type ExecuteGutterCallback = (lineNumber: number) => void
type ExecuteQueryCallback = (result: StatementToExecute) => void
type StatementNotFoundCallback = () => void

export type EditorCallbacks = {
  onExecuteQuery: ExecuteQueryCallback
  onStatementNotFound: StatementNotFoundCallback
  onGutterExecute?: ExecuteGutterCallback
  onGutterContextMenu?: (lineNumber: number, x: number, y: number) => void
  onSave?: (query: string) => void
  onFormat?: () => void
}

export function useMonacoEditor(containerRef: Ref<HTMLElement | null>, initialValue: Ref<string>, options: MonacoEditorOptions = {}) {
  let editor: monaco.editor.IStandaloneCodeEditor | null = null
  let completionProvider: monaco.IDisposable | null = null
  let gutterDecorations: monaco.editor.IEditorDecorationsCollection | null = null
  let parsedStatements: SqlStatement[] = []
  let registeredCallbacks: EditorCallbacks | null = null
  let refreshDebounceTimer: ReturnType<typeof setTimeout> | null = null
  const { isDark } = useTheme()

  const refreshGutterDecorations = () => {
    if (refreshDebounceTimer !== null)
      clearTimeout(refreshDebounceTimer)
    refreshDebounceTimer = setTimeout(() => {
      refreshDebounceTimer = null
      if (!editor)
        return
      const model = editor.getModel()
      if (!model)
        return
      parsedStatements = parseSqlStatements(model.getValue())
      const decorations = getSqlGutterDecorations(parsedStatements)
      if (gutterDecorations) {
        gutterDecorations.set(decorations)
      }
      else {
        gutterDecorations = editor.createDecorationsCollection(decorations)
      }
    }, 150)
  }

  const initEditor = (callbacks: EditorCallbacks) => {
    registeredCallbacks = callbacks
    if (!containerRef.value)
      return

    completionProvider = monaco.languages.registerCompletionItemProvider('sql', {
      provideCompletionItems: (model, position) => {
        const word = model.getWordUntilPosition(position)
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        }

        const noParenFunctions = ['NOW', 'CURRENT_DATE', 'CURRENT_TIME', 'CURRENT_TIMESTAMP']
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
          ...SQL_FUNCTIONS.map(func => ({
            label: func,
            kind: monaco.languages.CompletionItemKind.Function,
            insertText: noParenFunctions.includes(func) ? func : `${func}()`,
            range,
          })),
        ]

        return { suggestions }
      },
    })

    editor = monaco.editor.create(containerRef.value, {
      value: initialValue.value,
      language: options.language || 'sql',
      theme: isDark.value ? 'vs-dark' : 'vs',
      automaticLayout: true,
      readOnly: options.readOnly || false,
      minimap: { enabled: options.minimap !== false },
      fontSize: options.fontSize || 14,
      tabSize: options.tabSize || 2,
      lineNumbers: options.showLineNumbers === false ? 'off' : 'on',
      wordWrap: options.wordWrap === false ? 'off' : 'on',
      roundedSelection: false,
      contextmenu: true,
      formatOnPaste: true,
      formatOnType: true,
      suggest: { showKeywords: true, showSnippets: true },
    })

    refreshGutterDecorations()

    editor.onDidChangeModelContent(() => {
      refreshGutterDecorations()
    })

    editor.onMouseDown(({ event, target }) => {
      if (
        event.leftButton
        && target.type === MOUSE_TARGET_GUTTER_LINE_DECORATIONS
        && target.element?.classList
        && Array.from(target.element.classList).includes(SQL_EXECUTE_GUTTER_CLASS)
        && target.position
      ) {
        const stmt = getStatementAtLine(parsedStatements, target.position.lineNumber)
        if (stmt) {
          callbacks.onExecuteQuery({ sql: stmt.statement, source: 'statement', found: true })
        }
        else {
          callbacks.onStatementNotFound()
        }
      }
    })

    editor.onContextMenu(({ event, target }) => {
      if (
        target.type === MOUSE_TARGET_GUTTER_LINE_DECORATIONS
        && target.element?.classList
        && Array.from(target.element.classList).includes(SQL_EXECUTE_GUTTER_CLASS)
        && target.position
        && callbacks.onGutterContextMenu
      ) {
        event.preventDefault()
        event.stopPropagation()
        callbacks.onGutterContextMenu(target.position.lineNumber, event.posx, event.posy)
      }
    })

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
      if (!editor)
        return
      const result = getStatementToExecute(editor, parsedStatements)
      if (result.found) {
        callbacks.onExecuteQuery(result)
      }
      else {
        callbacks.onStatementNotFound()
      }
    })

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      callbacks.onSave?.(editor?.getValue() ?? '')
    })

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Slash, () => {
      editor?.trigger('keyboard', 'editor.action.commentLine', {})
    })

    if (callbacks.onFormat) {
      editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyF, () => {
        callbacks.onFormat?.()
      })
    }

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Space, () => {
      editor?.trigger('keyboard', 'editor.action.triggerSuggest', {})
    })

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
    if (refreshDebounceTimer !== null)
      clearTimeout(refreshDebounceTimer)
    gutterDecorations?.clear()
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
    executeAtLine: (lineNumber: number) => {
      if (!editor || !registeredCallbacks)
        return
      const stmt = getStatementAtLine(parsedStatements, lineNumber)
      if (stmt) {
        registeredCallbacks.onExecuteQuery({ sql: stmt.statement, source: 'statement', found: true })
      }
      else {
        registeredCallbacks.onStatementNotFound()
      }
    },
    getStatementTextAtLine: (lineNumber: number): string | null => {
      const stmt = getStatementAtLine(parsedStatements, lineNumber)
      return stmt?.statement ?? null
    },
  }
}
