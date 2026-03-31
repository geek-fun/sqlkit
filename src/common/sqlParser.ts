export interface CursorPosition {
  lineNumber: number
  column: number
}

export interface Selection {
  startLineNumber: number
  startColumn: number
  endLineNumber: number
  endColumn: number
}

export function extractStatementAtCursor(sql: string, cursorPosition?: CursorPosition, selection?: Selection): string {
  if (selection) {
    const lines = sql.split('\n')
    const selectedLines = lines.slice(
      selection.startLineNumber - 1,
      selection.endLineNumber,
    )

    if (selectedLines.length === 0)
      return sql

    if (selectedLines.length === 1) {
      return selectedLines[0].substring(
        selection.startColumn - 1,
        selection.endColumn - 1,
      )
    }

    const firstLine = selectedLines[0].substring(selection.startColumn - 1)
    const lastLine = selectedLines[selectedLines.length - 1].substring(
      0,
      selection.endColumn - 1,
    )
    const middleLines = selectedLines.slice(1, -1)

    return [firstLine, ...middleLines, lastLine].join('\n')
  }

  if (!cursorPosition || !sql.includes(';')) {
    return sql.trim()
  }

  const lines = sql.split('\n')
  const offset = lines
    .slice(0, cursorPosition.lineNumber - 1)
    .reduce((acc, line) => acc + line.length + 1, cursorPosition.column - 1)

  const beforeCursor = sql.substring(0, offset)
  const afterCursor = sql.substring(offset)

  const lastSemicolon = beforeCursor.lastIndexOf(';')
  const nextSemicolon = afterCursor.indexOf(';')

  const start = lastSemicolon === -1 ? 0 : lastSemicolon + 1
  const end = nextSemicolon === -1 ? sql.length : offset + nextSemicolon

  const statement = sql.substring(start, end).trim()
  return statement || sql.trim()
}
