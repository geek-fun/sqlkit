import type { CursorPosition, Selection } from '@/common/sqlParser'
import { extractStatementAtCursor } from '@/common/sqlParser'

describe('extractStatementAtCursor', () => {
  describe('with selection', () => {
    it('extracts single-line selection', () => {
      const sql = 'SELECT * FROM users WHERE id = 1'
      const selection: Selection = {
        startLineNumber: 1,
        startColumn: 8,
        endLineNumber: 1,
        endColumn: 14,
      }

      const result = extractStatementAtCursor(sql, undefined, selection)
      expect(result).toBe('* FROM')
    })

    it('extracts multi-line selection', () => {
      const sql = `SELECT *
FROM users
WHERE id = 1`
      const selection: Selection = {
        startLineNumber: 1,
        startColumn: 8,
        endLineNumber: 2,
        endColumn: 11,
      }

      const result = extractStatementAtCursor(sql, undefined, selection)
      expect(result).toBe(`*
FROM users`)
    })

    it('extracts full line when selecting entire line', () => {
      const sql = `SELECT *
FROM users`
      const selection: Selection = {
        startLineNumber: 2,
        startColumn: 1,
        endLineNumber: 2,
        endColumn: 11,
      }

      const result = extractStatementAtCursor(sql, undefined, selection)
      expect(result).toBe('FROM users')
    })

    it('returns full sql when selected lines is empty', () => {
      const sql = 'SELECT * FROM users'
      const selection: Selection = {
        startLineNumber: 5,
        startColumn: 1,
        endLineNumber: 5,
        endColumn: 1,
      }

      const result = extractStatementAtCursor(sql, undefined, selection)
      expect(result).toBe('SELECT * FROM users')
    })
  })

  describe('without selection - statement splitting', () => {
    it('returns trimmed sql when no semicolons', () => {
      const sql = 'SELECT * FROM users'
      const cursor: CursorPosition = { lineNumber: 1, column: 10 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe('SELECT * FROM users')
    })

    it('returns trimmed sql when no cursor position', () => {
      const sql = 'SELECT * FROM users;'

      const result = extractStatementAtCursor(sql)
      expect(result).toBe('SELECT * FROM users;')
    })

    it('extracts statement at cursor - first statement', () => {
      const sql = 'SELECT * FROM users; DELETE FROM logs; UPDATE stats SET count = 1;'
      const cursor: CursorPosition = { lineNumber: 1, column: 15 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe('SELECT * FROM users')
    })

    it('extracts statement at cursor - middle statement', () => {
      const sql = 'SELECT * FROM users; DELETE FROM logs; UPDATE stats SET count = 1;'
      const cursor: CursorPosition = { lineNumber: 1, column: 30 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe('DELETE FROM logs')
    })

    it('extracts statement at cursor - last statement', () => {
      const sql = 'SELECT * FROM users; DELETE FROM logs; UPDATE stats SET count = 1;'
      const cursor: CursorPosition = { lineNumber: 1, column: 60 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe('UPDATE stats SET count = 1')
    })

    it('handles multi-line statements', () => {
      const sql = `SELECT *
FROM users
WHERE id = 1;
DELETE FROM logs;`
      const cursor: CursorPosition = { lineNumber: 2, column: 5 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe(`SELECT *
FROM users
WHERE id = 1`)
    })

    it('handles cursor at end of statement', () => {
      const sql = 'SELECT * FROM users; DELETE FROM logs;'
      const cursor: CursorPosition = { lineNumber: 1, column: 20 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe('SELECT * FROM users')
    })

    it('handles cursor at start of next statement', () => {
      const sql = 'SELECT * FROM users; DELETE FROM logs;'
      const cursor: CursorPosition = { lineNumber: 1, column: 22 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe('DELETE FROM logs')
    })

    it('returns full trimmed sql when statement extraction yields empty', () => {
      const sql = '  ;  '
      const cursor: CursorPosition = { lineNumber: 1, column: 3 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe(';')
    })

    it('handles empty sql', () => {
      const sql = ''
      const cursor: CursorPosition = { lineNumber: 1, column: 1 }

      const result = extractStatementAtCursor(sql, cursor)
      expect(result).toBe('')
    })
  })
})
