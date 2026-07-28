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
  let _line = 1

  for (let i = 0; i < content.length; i++) {
    const ch = content[i]
    const next = i + 1 < content.length ? content[i + 1] : null

    // Track lines
    if (ch === '\n') {
      _line++
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

/**
 * Count leading lines in `text` that are whitespace-only or comment-only
 * (`--` line comments, `/ * ... * /` block comments). Used to position the
 * gutter execute icon on the first line with actual SQL content.
 */
function skipLeadingComments(text: string): number {
  const lines = text.split('\n')
  let inBlockComment = false
  let skipped = 0

  for (const line of lines) {
    if (inBlockComment) {
      const endIdx = line.indexOf('*/')
      if (endIdx !== -1) {
        inBlockComment = false
        if (line.slice(endIdx + 2).trim().length > 0) {
          // SQL after block comment on same line — stop
          break
        }
      }
      skipped++
      continue
    }

    const trimmed = line.trim()

    if (trimmed.length === 0) {
      skipped++
      continue
    }

    if (trimmed.startsWith('--')) {
      skipped++
      continue
    }

    if (trimmed.startsWith('/*')) {
      const endIdx = trimmed.indexOf('*/')
      if (endIdx === -1) {
        inBlockComment = true
        skipped++
        continue
      }
      if (trimmed.slice(endIdx + 2).trim().length > 0) {
        // SQL after inline block comment on same line
        break
      }
      skipped++
      continue
    }

    // First line with actual SQL content
    break
  }

  return skipped
}

// Keywords that can signal a new statement start after 2+ blank lines
// (when a `;` was forgotten). Covers DML, DDL, TCL, and utility commands.
const SOFT_STATEMENT_KEYWORDS = new Set([
  'SELECT',
  'CREATE',
  'ALTER',
  'DROP',
  'INSERT',
  'UPDATE',
  'DELETE',
  'TRUNCATE',
  'GRANT',
  'REVOKE',
  'EXPLAIN',
  'SHOW',
  'DESCRIBE',
  'USE',
  'SET',
  'CALL',
  'EXEC',
  'EXECUTE',
  'BEGIN',
  'COMMIT',
  'ROLLBACK',
  'DECLARE',
  'ANALYZE',
  'VACUUM',
  'PRAGMA',
  'REFRESH',
  'COPY',
  'WITH',
  'MERGE',
  'REPLACE',
])

/**
 * After splitting by `;`, scan each range for 2+ consecutive blank lines
 * followed by a SQL keyword. When found, split the range there — this
 * recovers from missing `;` between statements separated by blank lines.
 */
function splitRangesAtBlankLines(ranges: StatementRange[], content: string): StatementRange[] {
  const result: StatementRange[] = []

  for (const range of ranges) {
    const text = content.slice(range.startOffset, range.endOffset)
    const lines = text.split('\n')

    // Pre-compute each line's byte offset within `text`
    const lineOffsets: number[] = []
    let off = 0
    for (const line of lines) {
      lineOffsets.push(off)
      off += line.length + 1 // +1 for \n
    }

    let blankRun = 0
    let seenContent = false
    let inBlockComment = false
    // Track the line index of the last actual SQL content (for split boundaries)
    let lastContentLine = -1
    // When the segment starts with `WITH`, suppress soft-split for the next DML
    // keyword — it's the main query of a CTE, not a new statement.
    let segmentFirstKeyword: string | null = null
    const FIRST_DML_KEYWORDS = new Set(['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'MERGE', 'REPLACE'])
    // Each split: { sqlLine: the SQL keyword line, prevContentLine: last content before blank run }
    const splits: { sqlLine: number, prevContentLine: number }[] = []

    for (let i = 0; i < lines.length; i++) {
      const trimmed = lines[i].trim()

      // Lines inside a multi-line block comment — skip without resetting blankRun
      if (inBlockComment) {
        if (trimmed.endsWith('*/'))
          inBlockComment = false
        continue
      }

      if (trimmed.length === 0) {
        if (seenContent)
          blankRun++
        continue
      }

      // Comment-only lines should not reset the blank-run counter,
      // so that `SELECT ...\n\n\n-- comment\nSELECT ...` still splits.
      if (trimmed.startsWith('--') || trimmed.startsWith('#')) {
        continue
      }
      if (trimmed.startsWith('/*')) {
        if (!trimmed.endsWith('*/'))
          inBlockComment = true
        continue
      }

      // Actual SQL content
      const firstWord = trimmed.split(/\s+/)[0]?.toUpperCase()
      if (!seenContent) {
        seenContent = true
        segmentFirstKeyword = firstWord ?? null
      }
      else if (blankRun >= 2 && firstWord && SOFT_STATEMENT_KEYWORDS.has(firstWord)) {
        // When the segment starts with WITH, the next DML keyword is the main
        // query of the CTE (e.g. `WITH cte AS (...) \n\n\n SELECT ...`).
        // Splitting there would orphan the CTE definition.
        const isCteMainQuery = segmentFirstKeyword === 'WITH' && firstWord && FIRST_DML_KEYWORDS.has(firstWord)
        if (!isCteMainQuery)
          splits.push({ sqlLine: i, prevContentLine: lastContentLine })
      }
      blankRun = 0
      lastContentLine = i
    }

    if (splits.length === 0) {
      result.push(range)
      continue
    }

    // Build sub-ranges. Each preceding segment ends at its last content line
    // (excluding trailing blank/comment lines). Each new segment starts at the
    // SQL keyword line in `splits[].sqlLine`.
    let segStart = range.startOffset + lineOffsets[0]

    for (const { sqlLine, prevContentLine } of splits) {
      // End the preceding segment right after `prevContentLine`
      const segEnd = range.startOffset + lineOffsets[prevContentLine] + lines[prevContentLine].length
      result.push(buildRange(content, segStart, segEnd))
      segStart = range.startOffset + lineOffsets[sqlLine]
    }

    // Final segment (trim leading whitespace)
    const rawFinal = content.slice(segStart, range.endOffset)
    const finalTrimmed = rawFinal.trimStart()
    if (finalTrimmed.length > 0) {
      const skipped = rawFinal.length - finalTrimmed.length
      result.push(buildRange(content, segStart + skipped, range.endOffset))
    }
  }

  return result.filter(r => r.text.trim().length > 0)
}

export function parseSqlStatements(content: string): SqlStatement[] {
  const rawRanges = scanStatements(content)
  const ranges = splitRangesAtBlankLines(rawRanges, content)
  const lines = content.split('\n')

  return ranges
    .filter((r) => {
      const t = r.text.trim().replace(/;\s*$/, '').trim()
      return t.length > 0
    })
    .map((r) => {
      const statement = r.text.trim().replace(/;\s*$/, '').trim()
      return {
        statement,
        position: {
          startLineNumber: r.startLine + skipLeadingComments(r.text),
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
