import { useDataGridSort } from '@/composables/useDataGridSort'

describe('useDataGridSort', () => {
  it('cycles a single column ASC → DESC → cleared', () => {
    const sort = useDataGridSort()

    sort.toggleSort('id')
    expect(sort.sortState.value).toEqual([{ column: 'id', direction: 'ASC' }])

    sort.toggleSort('id')
    expect(sort.sortState.value).toEqual([{ column: 'id', direction: 'DESC' }])

    sort.toggleSort('id')
    expect(sort.sortState.value).toEqual([])
  })

  it('replaces the previous column when shift is not held', () => {
    const sort = useDataGridSort()
    sort.toggleSort('id')
    sort.toggleSort('name')
    expect(sort.sortState.value).toEqual([{ column: 'name', direction: 'ASC' }])
  })

  it('appends columns for multi-column sort when shift is held', () => {
    const sort = useDataGridSort()
    sort.toggleSort('id')
    sort.toggleSort('name', true)
    expect(sort.sortState.value).toEqual([
      { column: 'id', direction: 'ASC' },
      { column: 'name', direction: 'ASC' },
    ])
    expect(sort.getSortPriority('name')).toBe(2)
  })

  it('sets an explicit direction without dropping other rules', () => {
    const sort = useDataGridSort()
    sort.toggleSort('id')
    sort.toggleSort('name', true)
    sort.setSort('id', 'DESC')
    expect(sort.sortState.value).toEqual([
      { column: 'id', direction: 'DESC' },
      { column: 'name', direction: 'ASC' },
    ])
  })

  it('prunes rules for columns that are no longer available', () => {
    const sort = useDataGridSort()
    sort.toggleSort('id')
    sort.toggleSort('name', true)
    sort.pruneSort(['name'])
    expect(sort.sortState.value).toEqual([{ column: 'name', direction: 'ASC' }])
    expect(sort.getSortDirection('id')).toBeNull()
  })
})
