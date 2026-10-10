import type { SortState } from '@/types/grid'
import {
  collectSortableColumns,
  compareSortValues,
  formatSortState,
  isPrimitiveValue,
  isSortableColumn,
  sortRowsByState,
} from '@/utils/gridSort'

type Row = Record<string, unknown>

describe('isPrimitiveValue', () => {
  it('accepts primitives and null', () => {
    expect(isPrimitiveValue('a')).toBe(true)
    expect(isPrimitiveValue(1)).toBe(true)
    expect(isPrimitiveValue(true)).toBe(true)
    expect(isPrimitiveValue(null)).toBe(true)
    expect(isPrimitiveValue(undefined)).toBe(true)
  })

  it('rejects objects and arrays', () => {
    expect(isPrimitiveValue({ a: 1 })).toBe(false)
    expect(isPrimitiveValue([1, 2])).toBe(false)
  })
})

describe('isSortableColumn', () => {
  it('allows primitive columns', () => {
    const rows: Row[] = [{ id: 1 }, { id: 2 }, { id: null }]
    expect(isSortableColumn('id', rows, { id: 'integer' })).toBe(true)
  })

  it('rejects declared JSON/BLOB column types', () => {
    const rows: Row[] = [{ payload: '{"a":1}' }, { payload: '"x"' }]
    expect(isSortableColumn('payload', rows, { payload: 'jsonb' })).toBe(false)
    expect(isSortableColumn('payload', rows, { payload: 'BYTEA' })).toBe(false)
    expect(isSortableColumn('payload', rows, { payload: 'blob' })).toBe(false)
  })

  it('rejects columns holding object or array cells', () => {
    const rows: Row[] = [{ payload: 'ok' }, { payload: { a: 1 } }]
    expect(isSortableColumn('payload', rows, { payload: 'text' })).toBe(false)
  })

  it('treats an empty result set as sortable', () => {
    expect(isSortableColumn('id', [], { id: 'integer' })).toBe(true)
  })

  it('collects only sortable columns', () => {
    const rows: Row[] = [{ id: 1, meta: { a: 1 }, label: 'x' }]
    const sortable = collectSortableColumns(['id', 'meta', 'label'], rows, { meta: 'jsonb' })
    expect([...sortable]).toEqual(['id', 'label'])
  })
})

describe('mixed-type columns', () => {
  it('compares an untyped number+string column by one fixed kind (numbers first)', () => {
    const rows = [
      { v: 'abc' },
      { v: 10 },
      { v: 2 },
      { v: null },
    ]
    const sorted = sortRowsByState(rows, [{ column: 'v', direction: 'ASC' }])
    expect(sorted.map(r => r.v)).toEqual([2, 10, 'abc', null])
  })

  it('keeps the comparator symmetric on mixed columns', () => {
    // per-cell kind resolution made compare(a,b) !== -compare(b,a) here
    expect(compareSortValues('10', 9, 'number')).toBeGreaterThan(0)
    expect(compareSortValues(9, '10', 'number')).toBeLessThan(0)
  })

  it('declared type wins over value kinds', () => {
    const rows = [
      { v: '9' },
      { v: '10' },
    ]
    const sorted = sortRowsByState(rows, [{ column: 'v', direction: 'ASC' }], { v: 'integer' })
    expect(sorted.map(r => r.v)).toEqual(['9', '10'])
  })

  it('compares numeric strings with a numeric column type', () => {
    expect(compareSortValues('9', '10', 'bigint')).toBeLessThan(0)
  })

  it('orders booleans false before true', () => {
    expect(compareSortValues(false, true, 'boolean')).toBeLessThan(0)
    expect(compareSortValues('t', 'f', 'bool')).toBeGreaterThan(0)
  })

  it('orders dates chronologically', () => {
    expect(compareSortValues('2024-01-01', '2024-06-01', 'date')).toBeLessThan(0)
    expect(compareSortValues('2024-06-01T00:00:00Z', '2024-06-01T00:00:00Z', 'timestamp')).toBe(0)
  })

  it('sorts nulls after real values in ascending order', () => {
    expect(compareSortValues(null, 1)).toBeGreaterThan(0)
    expect(compareSortValues(1, null)).toBeLessThan(0)
    expect(compareSortValues(null, null)).toBe(0)
  })

  it('uses numeric-aware, case-insensitive string comparison', () => {
    expect(compareSortValues('item2', 'item10')).toBeLessThan(0)
    expect(compareSortValues('Apple', 'apple')).toBe(0)
  })
})

describe('sortRowsByState', () => {
  const rows: Row[] = [
    { id: 3, name: 'carol' },
    { id: 1, name: 'alice' },
    { id: 2, name: 'bob' },
  ]

  it('returns a copy when no sort rules are active', () => {
    const result = sortRowsByState(rows, [])
    expect(result).toEqual(rows)
    expect(result).not.toBe(rows)
  })

  it('sorts ascending without mutating the input', () => {
    const result = sortRowsByState(rows, [{ column: 'id', direction: 'ASC' }])
    expect(result.map(r => r.id)).toEqual([1, 2, 3])
    expect(rows.map(r => r.id)).toEqual([3, 1, 2])
  })

  it('sorts descending', () => {
    const result = sortRowsByState(rows, [{ column: 'id', direction: 'DESC' }])
    expect(result.map(r => r.id)).toEqual([3, 2, 1])
  })

  it('applies multiple rules in priority order', () => {
    const data: Row[] = [
      { dept: 'b', age: 30 },
      { dept: 'a', age: 40 },
      { dept: 'a', age: 20 },
    ]
    const state: SortState = [
      { column: 'dept', direction: 'ASC' },
      { column: 'age', direction: 'DESC' },
    ]
    const result = sortRowsByState(data, state)
    expect(result.map(r => `${r.dept}${r.age}`)).toEqual(['a40', 'a20', 'b30'])
  })

  it('keeps the original order for equal keys', () => {
    const data: Row[] = [
      { group: 'x', seq: 1 },
      { group: 'x', seq: 2 },
      { group: 'x', seq: 3 },
    ]
    const result = sortRowsByState(data, [{ column: 'group', direction: 'ASC' }])
    expect(result.map(r => r.seq)).toEqual([1, 2, 3])
  })

  it('puts nulls last ascending and first descending', () => {
    const data: Row[] = [{ n: 2 }, { n: null }, { n: 1 }]
    expect(sortRowsByState(data, [{ column: 'n', direction: 'ASC' }]).map(r => r.n)).toEqual([1, 2, null])
    expect(sortRowsByState(data, [{ column: 'n', direction: 'DESC' }]).map(r => r.n)).toEqual([null, 2, 1])
  })

  it('uses the declared column type for ordering', () => {
    const data: Row[] = [{ v: '10' }, { v: '9' }]
    expect(sortRowsByState(data, [{ column: 'v', direction: 'ASC' }], { v: 'bigint' }).map(r => r.v))
      .toEqual(['9', '10'])
  })
})

describe('formatSortState', () => {
  it('renders column and direction', () => {
    expect(formatSortState([{ column: 'id', direction: 'ASC' }, { column: 'name', direction: 'DESC' }]))
      .toBe('id ↑, name ↓')
  })

  it('renders an empty string without rules', () => {
    expect(formatSortState([])).toBe('')
  })
})
