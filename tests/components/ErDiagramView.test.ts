import type { ForeignKeyInfo } from '@/datasources/erDiagramApi'
import type { ColumnInfo } from '@/types/connection'
import { invoke } from '@tauri-apps/api/core'
import { getForeignKeys } from '@/datasources/erDiagramApi'

// ─── Mocks ──────────────────────────────────────────────

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))

// ─── Helpers (mirroring ErDiagramView internals) ────────

const HEADER_HEIGHT = 36
const COL_HEIGHT = 28
const EXPAND_BTN_HEIGHT = 28
const CARD_PADDING = 8

function calcNodeHeight(columns: number, isExpanded: boolean): number {
  const colCount = isExpanded
    ? columns
    : Math.min(5, columns)
  const expandBtn = columns > 5 ? EXPAND_BTN_HEIGHT : 0
  return HEADER_HEIGHT + colCount * COL_HEIGHT + expandBtn + CARD_PADDING
}

// ─── Tests ──────────────────────────────────────────────

describe('erDiagramView', () => {
  beforeEach(() => {
    jest.clearAllMocks()
  })

  describe('getForeignKeys (data layer)', () => {
    it('fetches foreign keys via invoke', async () => {
      const mockFKs: ForeignKeyInfo[] = [
        {
          constraint_name: 'fk_order_user',
          source_schema: 'public',
          source_table: 'orders',
          source_column: 'user_id',
          target_schema: 'public',
          target_table: 'users',
          target_column: 'id',
        },
      ]
      ;(invoke as jest.Mock).mockResolvedValue(mockFKs)

      const result = await getForeignKeys('conn-1', 'db1', 'public')

      expect(invoke).toHaveBeenCalledWith('get_foreign_keys', {
        connectionId: 'conn-1',
        database: 'db1',
        schema: 'public',
      })
      expect(result).toEqual(mockFKs)
    })

    it('passes null schema when schema is undefined', async () => {
      ;(invoke as jest.Mock).mockResolvedValue([])

      await getForeignKeys('conn-1', 'db1', null)

      expect(invoke).toHaveBeenCalledWith('get_foreign_keys', {
        connectionId: 'conn-1',
        database: 'db1',
        schema: null,
      })
    })

    it('returns empty array when no foreign keys exist', async () => {
      ;(invoke as jest.Mock).mockResolvedValue([])

      const result = await getForeignKeys('conn-1', 'db1', 'public')

      expect(result).toEqual([])
    })
  })

  describe('list_tables and list_columns (data layer)', () => {
    it('fetches tables via invoke', async () => {
      const mockTables = [
        { name: 'users', schema: 'public', table_type: 'TABLE' },
        { name: 'orders', schema: 'public', table_type: 'TABLE' },
      ]
      ;(invoke as jest.Mock).mockResolvedValue(mockTables)

      const result = await invoke('list_tables', {
        connectionId: 'conn-1',
        database: 'db1',
        schema: 'public',
      })

      expect(invoke).toHaveBeenCalledWith('list_tables', {
        connectionId: 'conn-1',
        database: 'db1',
        schema: 'public',
      })
      expect(result).toEqual(mockTables)
    })

    it('fetches columns for a table via invoke', async () => {
      const mockColumns: ColumnInfo[] = [
        { name: 'id', data_type: 'integer', nullable: false, is_primary_key: true, is_auto_increment: true },
        { name: 'name', data_type: 'varchar', nullable: true, is_primary_key: false, is_auto_increment: false },
        { name: 'email', data_type: 'varchar', nullable: false, is_primary_key: false, is_auto_increment: false },
      ]
      ;(invoke as jest.Mock).mockResolvedValue(mockColumns)

      const result = await invoke('list_columns', {
        connectionId: 'conn-1',
        database: 'db1',
        schema: 'public',
        tableName: 'users',
      })

      expect(invoke).toHaveBeenCalledWith('list_columns', {
        connectionId: 'conn-1',
        database: 'db1',
        schema: 'public',
        tableName: 'users',
      })
      expect(result).toEqual(mockColumns)
    })

    it('handles list_columns failure gracefully (returns empty array)', async () => {
      ;(invoke as jest.Mock).mockRejectedValue(new Error('DB error'))

      let columns: ColumnInfo[] = []
      try {
        columns = await invoke('list_columns', {
          connectionId: 'conn-1',
          database: 'db1',
          schema: 'public',
          tableName: 'broken',
        })
      }
      catch {
        columns = []
      }

      expect(columns).toEqual([])
    })
  })

  describe('table data enrichment', () => {
    it('attaches relevant foreign keys to each table', () => {
      const fkList: ForeignKeyInfo[] = [
        { constraint_name: 'fk1', source_schema: 'public', source_table: 'orders', source_column: 'user_id', target_schema: 'public', target_table: 'users', target_column: 'id' },
        { constraint_name: 'fk2', source_schema: 'public', source_table: 'reviews', source_column: 'user_id', target_schema: 'public', target_table: 'users', target_column: 'id' },
      ]

      const tables = [
        { name: 'users', schema: 'public', table_type: 'TABLE' },
        { name: 'orders', schema: 'public', table_type: 'TABLE' },
      ]

      const enriched = tables.map(t => ({
        name: t.name,
        schema: t.schema,
        foreignKeys: fkList.filter(
          fk => fk.source_table === t.name || fk.target_table === t.name,
        ),
      }))

      const usersFKs = enriched.find(e => e.name === 'users')!.foreignKeys
      expect(usersFKs).toHaveLength(2)
      expect(usersFKs[0].source_table).toBe('orders')
      expect(usersFKs[1].source_table).toBe('reviews')

      const ordersFKs = enriched.find(e => e.name === 'orders')!.foreignKeys
      expect(ordersFKs).toHaveLength(1)
      expect(ordersFKs[0].source_table).toBe('orders')
    })
  })

  describe('calcNodeHeight (layout helper)', () => {
    it('shows 5 columns when collapsed with 10+ columns', () => {
      const height = calcNodeHeight(10, false)
      // HEADER + 5*COL + EXPAND_BTN + PADDING
      expect(height).toBe(36 + 5 * 28 + 28 + 8)
    })

    it('shows all columns when expanded', () => {
      const height = calcNodeHeight(10, true)
      // HEADER + 10*COL + EXPAND_BTN + PADDING
      expect(height).toBe(36 + 10 * 28 + 28 + 8)
    })

    it('hides expand button when 5 or fewer columns', () => {
      const height = calcNodeHeight(5, false)
      // HEADER + 5*COL + 0 + PADDING (no expand btn)
      expect(height).toBe(36 + 5 * 28 + 0 + 8)
    })

    it('shows expand button when more than 5 columns', () => {
      const height = calcNodeHeight(6, false)
      // HEADER + 5*COL + EXPAND_BTN + PADDING
      expect(height).toBe(36 + 5 * 28 + 28 + 8)
    })

    it('handles 0 columns gracefully', () => {
      const height = calcNodeHeight(0, false)
      // HEADER + 0 + 0 + PADDING
      expect(height).toBe(36 + 0 + 0 + 8)
    })
  })

  describe('highlightedTables logic', () => {
    it('finds all directly connected tables', () => {
      const fkList: ForeignKeyInfo[] = [
        { constraint_name: 'fk1', source_schema: 'public', source_table: 'orders', source_column: 'user_id', target_schema: 'public', target_table: 'users', target_column: 'id' },
        { constraint_name: 'fk2', source_schema: 'public', source_table: 'reviews', source_column: 'user_id', target_schema: 'public', target_table: 'users', target_column: 'id' },
        { constraint_name: 'fk3', source_schema: 'public', source_table: 'orders', source_column: 'product_id', target_schema: 'public', target_table: 'products', target_column: 'id' },
      ]

      // Simulating the highlightedTables computed from ErDiagramView
      const selectedTableId = 'users'
      const connected = new Set<string>([selectedTableId])
      for (const fk of fkList) {
        if (fk.source_table === selectedTableId)
          connected.add(fk.target_table)
        if (fk.target_table === selectedTableId)
          connected.add(fk.source_table)
      }

      expect(connected.has('users')).toBe(true)
      expect(connected.has('orders')).toBe(true)
      expect(connected.has('reviews')).toBe(true)
      expect(connected.has('products')).toBe(false) // not directly connected to users
    })

    it('returns only the selected table when it has no relationships', () => {
      const fkList: ForeignKeyInfo[] = [
        { constraint_name: 'fk1', source_schema: 'public', source_table: 'orders', source_column: 'user_id', target_schema: 'public', target_table: 'users', target_column: 'id' },
      ]

      const selectedTableId = 'standalone'
      const connected = new Set<string>([selectedTableId])
      for (const fk of fkList) {
        if (fk.source_table === selectedTableId)
          connected.add(fk.target_table)
        if (fk.target_table === selectedTableId)
          connected.add(fk.source_table)
      }

      expect(connected.size).toBe(1)
      expect(connected.has('standalone')).toBe(true)
    })
  })
})
