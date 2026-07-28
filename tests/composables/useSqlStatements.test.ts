import { parseSqlStatements } from '@/composables/useSqlStatements'

describe('parseSqlStatements', () => {
  describe('simple statements', () => {
    it('parses a single SELECT without semicolon', () => {
      const sql = 'SELECT * FROM users'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toBe('SELECT * FROM users')
    })

    it('parses a single SELECT with semicolon', () => {
      const sql = 'SELECT * FROM users;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toBe('SELECT * FROM users')
    })

    it('parses two statements separated by semicolon', () => {
      const sql = 'SELECT * FROM users;\nDELETE FROM logs;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      expect(result[0].statement).toBe('SELECT * FROM users')
      expect(result[1].statement).toBe('DELETE FROM logs')
    })

    it('parses multi-line statement', () => {
      const sql = 'SELECT *\nFROM users\nWHERE id = 1;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toBe('SELECT *\nFROM users\nWHERE id = 1')
    })

    it('ignores blank lines and comment lines', () => {
      const sql = '\n-- comment\nSELECT * FROM users;'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('SELECT * FROM users')
    })
  })

  describe('cte (common table expression) statements', () => {
    it('treats a single CTE as one statement', () => {
      const sql = [
        'WITH ActiveUsers AS (',
        '  SELECT Id, FirstName FROM Users WHERE IsActive = 1',
        ')',
        'SELECT * FROM ActiveUsers;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('WITH ActiveUsers AS')
      expect(result[0].statement).toContain('SELECT * FROM ActiveUsers')
    })

    it('treats a CTE with INNER JOIN as one statement', () => {
      const sql = [
        'WITH ActiveUsers AS (',
        '  SELECT Id, FirstName, LastName FROM Users WHERE IsActive = 1',
        ')',
        'SELECT au.FirstName, au.LastName, o.ProductName',
        'FROM ActiveUsers au',
        'INNER JOIN Orders o ON au.Id = o.UserId;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('WITH ActiveUsers AS')
      expect(result[0].statement).toContain('INNER JOIN Orders')
    })

    it('treats multiple CTEs (comma-separated) as one statement', () => {
      const sql = [
        'WITH',
        '  cte1 AS (SELECT 1 AS n),',
        '  cte2 AS (SELECT 2 AS n)',
        'SELECT * FROM cte1 UNION ALL SELECT * FROM cte2;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('WITH')
      expect(result[0].statement).toContain('cte1')
      expect(result[0].statement).toContain('cte2')
    })

    it('correctly separates a CTE from a following plain SELECT', () => {
      const sql = [
        'WITH ActiveUsers AS (',
        '  SELECT Id FROM Users WHERE IsActive = 1',
        ')',
        'SELECT * FROM ActiveUsers;',
        'SELECT * FROM Orders;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      expect(result[0].statement).toContain('WITH ActiveUsers AS')
      expect(result[1].statement).toBe('SELECT * FROM Orders')
    })

    it('records correct line positions for a CTE statement', () => {
      const sql = [
        'WITH cte AS (',
        '  SELECT 1',
        ')',
        'SELECT * FROM cte;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].position.startLineNumber).toBe(1)
      expect(result[0].position.endLineNumber).toBe(4)
    })
  })

  describe('statement positions (gutter icon alignment)', () => {
    it('records correct startLineNumber for second statement when blank lines separate them', () => {
      const sql = [
        'SELECT 1;', // line 1
        '', // line 2 (blank)
        '', // line 3 (blank)
        'SELECT 2;', // line 4
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      // First statement starts at line 1
      expect(result[0].position.startLineNumber).toBe(1)
      // Second statement should start at line 4 (first non-blank line),
      // NOT at the line after the ; (line 1)
      expect(result[1].position.startLineNumber).toBe(4)
    })

    it('records correct startLineNumber when preceded by blank lines and comment', () => {
      const sql = [
        'SELECT 1;', // line 1
        '', // line 2 (blank)
        '', // line 3 (blank)
        '-- comment', // line 4
        'SELECT 2;', // line 5
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      // Icon should be at line 5 (first SQL line), not at the -- comment (line 4)
      // or the blank after ; (line 1)
      expect(result[1].position.startLineNumber).toBe(5)
    })

    it('keeps startLineNumber unchanged when there is no leading blank', () => {
      const sql = [
        'SELECT 1;',
        'SELECT 2;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      expect(result[0].position.startLineNumber).toBe(1)
      expect(result[1].position.startLineNumber).toBe(2)
    })
  })

  describe('soft split at blank lines (missing semicolon)', () => {
    it('splits two statements separated by 2 blank lines', () => {
      const sql = [
        'SELECT 1', // line 1
        '', // line 2 (blank)
        '', // line 3 (blank)
        'SELECT 2', // line 4
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      expect(result[0].statement).toBe('SELECT 1')
      expect(result[1].statement).toBe('SELECT 2')
    })

    it('does not split on only 1 blank line', () => {
      const sql = 'SELECT 1\n\nSELECT 2'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('SELECT 1')
      expect(result[0].statement).toContain('SELECT 2')
    })

    it('splits 3 statements separated by 2 blank lines each', () => {
      const sql = [
        'SELECT 1', // line 1
        '', // line 2
        '', // line 3
        'SELECT 2', // line 4
        '', // line 5
        '', // line 6
        'SELECT 3', // line 7
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(3)
      expect(result[0].statement).toBe('SELECT 1')
      expect(result[1].statement).toBe('SELECT 2')
      expect(result[2].statement).toBe('SELECT 3')
    })

    it('does not split when next line after 2 blanks is not a SQL keyword', () => {
      const sql = [
        'SELECT 1', // line 1
        '', // line 2
        '', // line 3
        '  some_column', // line 4 (not a keyword)
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
    })

    it('sets correct positions for soft-split statements', () => {
      const sql = 'SELECT 1\n\n\nSELECT 2'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(2)
      // First statement on line 1
      expect(result[0].position.startLineNumber).toBe(1)
      expect(result[0].position.endLineNumber).toBe(1)
      // Second statement on line 4 (after 2 blank lines on lines 2-3)
      expect(result[1].position.startLineNumber).toBe(4)
      expect(result[1].position.endLineNumber).toBe(4)
    })

    it('handles mixed ; and blank-line splits', () => {
      const sql = 'SELECT 1;\nSELECT 2\n\n\nSELECT 3'
      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(3)
      expect(result[0].statement).toBe('SELECT 1')
      expect(result[1].statement).toBe('SELECT 2')
      expect(result[2].statement).toBe('SELECT 3')
    })
  })

  describe('nested parens (subqueries)', () => {
    it('does not split on SELECT inside a subquery', () => {
      const sql = [
        'SELECT * FROM (',
        '  SELECT id FROM users WHERE active = 1',
        ') sub;',
      ].join('\n')

      const result = parseSqlStatements(sql)
      expect(result).toHaveLength(1)
      expect(result[0].statement).toContain('SELECT * FROM')
      expect(result[0].statement).toContain('SELECT id FROM users')
    })
  })
})
