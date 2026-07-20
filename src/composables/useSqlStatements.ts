import type * as monaco from 'monaco-editor'

export type SqlStatement = {
  statement: string
  position: {
    startLineNumber: number
    endLineNumber: number
    startColumn: number
    endColumn: number
  }
}

export type ExecuteSource = 'selection' | 'statement' | 'full'

export type StatementToExecute = {
  sql: string
  source: ExecuteSource
  found: boolean
}

// ── Character-based SQL statement splitter ──
// Replaces the old line-based + regex approach that broke on correlated
// subqueries spanning multiple lines (e.g. (SELECT count(*) ...) as alias).
// This is the same algorithm dbx uses in Rust: character-by-character,
// tracking quote/comment context, and splitting only on `;` outside of quotes.

type ScannerState = {
  inSingleQuote: boolean
  inDoubleQuote: boolean
  inBacktick: boolean
  inLineComment: boolean
  inBlockComment: boolean
  prevChar: string | null
  dollarTag: string | null
}

type StatementRange = {
  text: string
  startOffset: number
  endOffset: number
  startLine: number
  endLine: number
}

const DEFAULT_STATE: ScannerState = {
  inSingleQuote: false,
  inDoubleQuote: false,
  inBacktick: false,
  inLineComment: false,
  inBlockComment: false,
  prevChar: null,
  dollarTag: null,
}

function isOutsideString(state: ScannerState): boolean {
  return !state.inSingleQuote && !state.inDoubleQuote && !state.inBacktick
}

function scanStatements(content: string): StatementRange[] {
  const ranges: StatementRange[] = []
  const state: ScannerState = { ...DEFAULT_STATE }
  let bufStart = 0
  let line = 1

  for (let i = 0; i < content.length; i++) {
    const ch = content[i]
    const next = i + 1 < content.length ? content[i + 1] : null

    // Track lines
    if (ch === '\n') {
      line++
    }

    // Handle dollar-quoting (PostgreSQL $$...$$)
    if (state.dollarTag) {
      const tag = state.dollarTag
      if (content.startsWith(tag, i)) {
        state.dollarTag = null
        i += tag.length - 1
        state.prevChar = tag[tag.length - 1]
        continue
      }
      state.prevChar = ch
      continue
    }

    if (state.inLineComment) {
      if (ch === '\n') {
        state.inLineComment = false
      }
      state.prevChar = ch
      continue
    }

    if (state.inBlockComment) {
      if (ch === '/' && state.prevChar === '*') {
        state.inBlockComment = false
      }
      state.prevChar = ch
      continue
    }

    // Start line comment?
    if (isOutsideString(state) && ch === '-' && next === '-') {
      state.inLineComment = true
      state.prevChar = ch
      continue
    }

    // Hash comment (MySQL dialect)?
    if (isOutsideString(state) && ch === '#') {
      state.inLineComment = true
      state.prevChar = ch
      continue
    }

    // Start block comment?
    if (isOutsideString(state) && ch === '/' && next === '*') {
      state.inBlockComment = true
      state.prevChar = ch
      continue
    }

    // Dollar-quote start (PostgreSQL)?
    if (isOutsideString(state) && ch === '$') {
      const tagEnd = content.indexOf('$', i + 1)
      if (tagEnd !== -1) {
        const tag = content.slice(i, tagEnd + 1)
        state.dollarTag = tag
        state.prevChar = ch
        continue
      }
    }

    // Quote tracking
    if (ch === '\'' && !state.inDoubleQuote && !state.inBacktick) {
      if (state.inSingleQuote && next === '\'') {
        i++ // skip escaped quote
        state.prevChar = ch
        continue
      }
      state.inSingleQuote = !state.inSingleQuote
      state.prevChar = ch
      continue
    }

    if (ch === '"' && !state.inSingleQuote && !state.inBacktick) {
      if (state.inDoubleQuote && next === '"') {
        i++ // skip escaped quote
        state.prevChar = ch
        continue
      }
      state.inDoubleQuote = !state.inDoubleQuote
      state.prevChar = ch
      continue
    }

    if (ch === '`' && !state.inSingleQuote && !state.inDoubleQuote) {
      state.inBacktick = !state.inBacktick
      state.prevChar = ch
      continue
    }

    // Statement separator: semicolon outside quotes
    if (ch === ';' && isOutsideString(state) && !state.inLineComment && !state.inBlockComment) {
      const text = content.slice(bufStart, i + 1)
      const textTrimmed = text.trim()
      if (textTrimmed.length > 0) {
        ranges.push(buildRange(content, bufStart, i + 1))
      }
      bufStart = i + 1
      state.prevChar = ch
      continue
    }

    state.prevChar = ch
  }

  // Trailing statement (no trailing semicolon)
  if (bufStart < content.length) {
    const text = content.slice(bufStart)
    if (text.trim().length > 0) {
      ranges.push(buildRange(content, bufStart, content.length))
    }
  }

  return ranges
}

function buildRange(content: string, start: number, end: number): StatementRange {
  return {
    text: content.slice(start, end),
    startOffset: start,
    endOffset: end,
    startLine: lineAtOffset(content, start),
    endLine: lineAtOffset(content, end),
  }
}

function lineAtOffset(content: string, offset: number): number {
  let line = 1
  for (let i = 0; i < offset && i < content.length; i++) {
    if (content[i] === '\n') {
      line++
    }
  }
  return line
}

export function parseSqlStatements(content: string): SqlStatement[] {
  const ranges = scanStatements(content)
  const lines = content.split('\n')

  return ranges
    .filter((r) => {
      // Remove trailing semicolon for the statement text
      const t = r.text.trim().replace(/;\s*$/, '').trim()
      return t.length > 0
    })
    .map((r) => {
      const statement = r.text.trim().replace(/;\s*$/, '').trim()
      return {
        statement,
        position: {
          startLineNumber: r.startLine,
          endLineNumber: r.endLine,
          startColumn: 1,
          endColumn: (lines[r.endLine - 1]?.length ?? 1) + 1,
        },
      }
    })
}

export function getStatementAtLine(statements: SqlStatement[], lineNumber: number): SqlStatement | undefined {
  return statements.find(
    ({ position }) =>
      lineNumber >= position.startLineNumber && lineNumber <= position.endLineNumber,
  )
}

export const SQL_EXECUTE_GUTTER_CLASS = 'sql-execute-decoration'

// Monaco editor MouseTargetType.GUTTER_LINE_DECORATIONS = 4
export const MOUSE_TARGET_GUTTER_LINE_DECORATIONS = 4

export function getSqlGutterDecorations(statements: SqlStatement[]): monaco.editor.IModelDeltaDecoration[] {
  return statements.map(({ position }) => ({
    range: {
      startLineNumber: position.startLineNumber,
      endLineNumber: position.startLineNumber,
      startColumn: 1,
      endColumn: 1,
    },
    options: {
      isWholeLine: true,
      linesDecorationsClassName: SQL_EXECUTE_GUTTER_CLASS,
    },
  }))
}

export function getStatementToExecute(editor: monaco.editor.IStandaloneCodeEditor, statements: SqlStatement[]): StatementToExecute {
  const model = editor.getModel()
  if (!model)
    return { sql: '', source: 'full', found: false }

  const selection = editor.getSelection()
  if (selection && !selection.isEmpty()) {
    const selectedText = model.getValueInRange(selection).trim()
    if (selectedText)
      return { sql: selectedText, source: 'selection', found: true }
  }

  const position = editor.getPosition()
  if (position) {
    const statement = getStatementAtLine(statements, position.lineNumber)
    if (statement)
      return { sql: statement.statement, source: 'statement', found: true }
  }

  return { sql: '', source: 'full', found: false }
}
