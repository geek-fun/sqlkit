import {
  formatTableValue,
  isTableNullValue,
  rowsToCsv,
} from '@/components/database-browser/dataTableHelpers'

describe('dataTableHelpers', () => {
  describe('formatTableValue', () => {
    it('formats null as NULL', () => {
      expect(formatTableValue(null)).toBe('NULL')
    })

    it('formats undefined as NULL', () => {
      expect(formatTableValue(undefined)).toBe('NULL')
    })

    it('formats a string as-is', () => {
      expect(formatTableValue('hello')).toBe('hello')
    })

    it('formats a number as a string', () => {
      expect(formatTableValue(42)).toBe('42')
      expect(formatTableValue(3.14)).toBe('3.14')
    })

    it('formats a boolean as a string', () => {
      expect(formatTableValue(true)).toBe('true')
      expect(formatTableValue(false)).toBe('false')
    })

    it('formats an object as JSON', () => {
      expect(formatTableValue({ key: 'value' })).toBe('{"key":"value"}')
    })

    it('formats an array as JSON', () => {
      expect(formatTableValue([1, 2, 3])).toBe('[1,2,3]')
    })
  })

  describe('isTableNullValue', () => {
    it('returns true for null', () => {
      expect(isTableNullValue(null)).toBe(true)
    })

    it('returns true for undefined', () => {
      expect(isTableNullValue(undefined)).toBe(true)
    })

    it('returns false for non-null values', () => {
      expect(isTableNullValue('')).toBe(false)
      expect(isTableNullValue(0)).toBe(false)
      expect(isTableNullValue(false)).toBe(false)
      expect(isTableNullValue('text')).toBe(false)
    })
  })

  describe('rowsToCsv', () => {
    const columns = ['id', 'name', 'email']
    const rows = [
      { id: 1, name: 'Alice', email: 'alice@example.com' },
      { id: 2, name: 'Bob', email: null },
      { id: 3, name: 'Charlie "Chuck"', email: 'chuck@example.com' },
    ]

    it('generates correct CSV header', () => {
      const csv = rowsToCsv(rows, columns)
      const header = csv.split('\n')[0]
      expect(header).toBe('"id","name","email"')
    })

    it('generates correct CSV rows', () => {
      const csv = rowsToCsv(rows, columns)
      const lines = csv.split('\n')
      expect(lines[1]).toBe('"1","Alice","alice@example.com"')
    })

    it('serializes null as empty string', () => {
      const csv = rowsToCsv(rows, columns)
      const lines = csv.split('\n')
      expect(lines[2]).toBe('"2","Bob",')
    })

    it('escapes double quotes in values', () => {
      const csv = rowsToCsv(rows, columns)
      const lines = csv.split('\n')
      expect(lines[3]).toBe('"3","Charlie ""Chuck""","chuck@example.com"')
    })

    it('returns only header for empty rows', () => {
      const csv = rowsToCsv([], columns)
      expect(csv).toBe('"id","name","email"')
    })

    it('handles object values by JSON-stringifying them', () => {
      const objRows = [{ id: 1, data: { key: 'val' } }]
      const csv = rowsToCsv(objRows, ['id', 'data'])
      const lines = csv.split('\n')
      expect(lines[1]).toBe('"1","{""key"":""val""}"')
    })
  })
})
