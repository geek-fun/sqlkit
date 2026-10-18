export type CursorPosition = {
  lineNumber: number
  column: number
}

export type Selection = {
  startLineNumber: number
  startColumn: number
  endLineNumber: number
  endColumn: number
}

export const extractStatementAtCursor = (sql: string, cursorPosition?: CursorPosition, selection?: Selection): string => {
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

  // eslint-disable-next-line regexp/use-ignore-case -- PostgreSQL dollar-quote tags are case-sensitive.
  const tokens = /--[^\r\n]*|\/\*[\s\S]*?\*\/|'(?:''|\\[\s\S]|[^'\\])*'|"(?:""|[^"])*"|`(?:``|[^`])*`|\[(?:\]\]|[^\]])*\]|(?<dollar>\$(?:[A-Za-z_]\w*)?\$)[\s\S]*?\k<dollar>|;/g
  const separators = Array.from(sql.matchAll(tokens))
    .filter(match => match[0] === ';')
    .map(match => match.index!)
  const start = separators.reduce((start, position) => position < offset ? position + 1 : start, 0)
  const end = separators.find(position => position >= offset) ?? sql.length

  const statement = sql.substring(start, end).trim()
  return statement || sql.trim()
}
