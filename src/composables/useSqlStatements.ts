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

// Matches DML/DDL keywords that begin a SQL statement at line start (case-insensitive)
// Also matches parentheses wrapping keywords (for subqueries like (SELECT...))
const SQL_STATEMENT_START_REGEX
  = /^\s*(?:\(\s*)?(?:SELECT|INSERT|UPDATE|DELETE|CREATE|ALTER|DROP|TRUNCATE|WITH|EXPLAIN|CALL|EXEC|MERGE|REPLACE|SHOW|DESCRIBE|DESC|USE|SET|BEGIN|COMMIT|ROLLBACK|SAVEPOINT|PRAGMA|VACUUM|GRANT|REVOKE|ATTACH|DETACH|ANALYZE|REINDEX|LOAD|UNLOAD|COPY|LOCK|UNLOCK)\b/i

const isStatementStart = (line: string): boolean => SQL_STATEMENT_START_REGEX.test(line)

function findFirstUnquotedSemicolon(line: string): number {
  let singleQuote = false
  let doubleQuote = false
  for (let i = 0; i < line.length; i++) {
    const ch = line[i]
    if (ch === '\'' && !doubleQuote) {
      if (singleQuote && line[i + 1] === '\'') {
        i++
        continue
      }
      singleQuote = !singleQuote
    }
    else if (ch === '"' && !singleQuote) {
      doubleQuote = !doubleQuote
    }
    else if (ch === '-' && line[i + 1] === '-' && !singleQuote && !doubleQuote) {
      // Line comment starts -- everything after is ignored
      return -1
    }
    else if (ch === ';' && !singleQuote && !doubleQuote) {
      return i
    }
  }
  return -1
}

function findStatementEnd(lines: string[], startLine: number): number {
  for (let i = startLine; i < lines.length; i++) {
    const semiIdx = findFirstUnquotedSemicolon(lines[i])
    if (semiIdx !== -1)
      return i

    if (i > startLine && isStatementStart(lines[i].trim()))
      return i - 1
  }
  return lines.length - 1
}

export function parseSqlStatements(content: string): SqlStatement[] {
  const lines = content.split('\n')
  const statements: SqlStatement[] = []
  let i = 0

  while (i < lines.length) {
    const trimmed = lines[i].trim()
    if (trimmed === '' || trimmed.startsWith('--') || trimmed.startsWith('//')) {
      i++
      continue
    }
    if (!isStatementStart(trimmed)) {
      i++
      continue
    }

    const startLine = i
    const endLine = findStatementEnd(lines, startLine)
    const statement = lines.slice(startLine, endLine + 1).join('\n').trim().replace(/;\s*$/, '')

    if (statement.length > 0) {
      statements.push({
        statement,
        position: {
          startLineNumber: startLine + 1,
          endLineNumber: endLine + 1,
          startColumn: 1,
          endColumn: lines[endLine].length + 1,
        },
      })
    }

    i = endLine + 1
  }

  return statements
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
