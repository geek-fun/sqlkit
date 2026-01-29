/**
 * SQL parsing utilities for extracting statements at cursor position
 */

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

/**
 * Extract the SQL statement at the cursor position
 * Statements are separated by semicolons
 */
export function extractStatementAtCursor(
  sql: string,
  cursorPosition?: CursorPosition,
  selection?: Selection,
): string {
  // If there's a selection, return the selected text
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

    // Multi-line selection
    const firstLine = selectedLines[0].substring(selection.startColumn - 1)
    const lastLine = selectedLines[selectedLines.length - 1].substring(
      0,
      selection.endColumn - 1,
    )
    const middleLines = selectedLines.slice(1, -1)

    return [firstLine, ...middleLines, lastLine].join('\n')
  }

  // If no cursor position or no semicolons, return entire SQL
  if (!cursorPosition || !sql.includes(';')) {
    return sql.trim()
  }

  // Find the offset in the full text based on cursor position
  const lines = sql.split('\n')
  let offset = 0
  for (let i = 0; i < cursorPosition.lineNumber - 1; i++) {
    offset += lines[i].length + 1 // +1 for newline
  }
  offset += cursorPosition.column - 1

  // Find statement boundaries
  let start = 0
  let end = sql.length

  // Find the previous semicolon
  for (let i = offset - 1; i >= 0; i--) {
    if (sql[i] === ';') {
      start = i + 1
      break
    }
  }

  // Find the next semicolon
  for (let i = offset; i < sql.length; i++) {
    if (sql[i] === ';') {
      end = i
      break
    }
  }

  const statement = sql.substring(start, end).trim()
  return statement || sql.trim()
}
