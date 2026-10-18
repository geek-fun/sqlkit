import { getAlignmentClass, getColumnAlignment } from '@/utils/gridAlignment'

describe('getColumnAlignment', () => {
  it('right-aligns numeric types', () => {
    const numeric = [
      'INT',
      'BIGINT',
      'INT4',
      'INT8',
      'SMALLINT',
      'TINYINT',
      'SERIAL',
      'BIGSERIAL',
      'FLOAT',
      'DOUBLE',
      'REAL',
      'DECIMAL',
      'NUMERIC',
      'MONEY',
    ]
    numeric.forEach(type => expect(getColumnAlignment(type)).toBe('end'))
  })

  it('matches numeric types case-insensitively', () => {
    expect(getColumnAlignment('numeric')).toBe('end')
    expect(getColumnAlignment('Int8')).toBe('end')
    expect(getColumnAlignment('BigInt')).toBe('end')
  })

  it('centers boolean types', () => {
    expect(getColumnAlignment('BOOL')).toBe('center')
    expect(getColumnAlignment('BOOLEAN')).toBe('center')
    expect(getColumnAlignment('BIT')).toBe('center')
  })

  it('left-aligns text, date, json and blob types', () => {
    ;['TEXT', 'VARCHAR', 'DATE', 'TIMESTAMP', 'JSONB', 'JSON', 'BYTEA', 'BLOB'].forEach(type =>
      expect(getColumnAlignment(type)).toBe('start'),
    )
  })

  it('left-aligns when the type is unknown', () => {
    expect(getColumnAlignment(undefined)).toBe('start')
    expect(getColumnAlignment('')).toBe('start')
  })
})

describe('getAlignmentClass', () => {
  // Header cells and data cells must resolve the same alignment per column,
  // otherwise a numeric column shows its header on the left and its values on
  // the right (which read as misaligned columns).
  it('returns the header/cell justification matching the column type', () => {
    expect(getAlignmentClass('NUMERIC')).toBe('justify-end')
    expect(getAlignmentClass('BOOL')).toBe('justify-center')
    expect(getAlignmentClass('TEXT')).toBe('')
    expect(getAlignmentClass(undefined)).toBe('')
  })
})
