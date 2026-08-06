import type { CompletionContext, SchemaSnapshot } from '@/composables/sqlCompletion/types'
import { buildSuggestions } from '@/composables/sqlCompletion/builder'
/**
 * @jest-environment node
 */
import { getDialectProfile } from '@/composables/sqlCompletion/dialects'

const profile = getDialectProfile('sql')

const snapshot: SchemaSnapshot = {
  databases: [{ name: 'app', isSystem: false }],
  schemasByDb: { app: ['public', 'analytics'] },
  tablesByKey: {
    'app': ['users', 'orders'],
    'app.public': ['users', 'orders'],
    'app.analytics': ['events'],
  },
  columnsByTable: {
    'c1|app|public|users': [
      { name: 'id', dataType: 'int4', isPrimaryKey: true },
      { name: 'email', dataType: 'text' },
      { name: 'created_at', dataType: 'timestamptz' },
    ],
  },
  derivedTables: {},
  hasSchemaData: true,
}

const opts = { connectionId: 'c1', currentDb: 'app', currentSchema: 'public' }

function ctx(partial: Partial<CompletionContext>): CompletionContext {
  return {
    word: '',
    tableRefs: [],
    activeTable: null,
    qualifier: '',
    isAfterDot: false,
    inComment: false,
    ...partial,
  }
}

const labels = (items: { label: string }[]) => items.map(i => i.label)

describe('buildSuggestions', () => {
  it('(a) word SEL → SELECT keyword suggested', () => {
    const result = buildSuggestions(ctx({ word: 'SEL' }), snapshot, profile, opts)
    expect(labels(result)).toContain('SELECT')
  })

  it('(b) empty word after FROM → table names (current db first)', () => {
    const result = buildSuggestions(
      ctx({ word: '', tableRefs: [{ table: 'users' }], activeTable: { table: 'users' } }),
      snapshot,
      profile,
      opts,
    )
    expect(labels(result)).toContain('users')
    expect(labels(result)).toContain('orders')
  })

  it('(c) after alias dot → columns only, kind column, detail data_type', () => {
    const result = buildSuggestions(
      ctx({ word: '', isAfterDot: true, qualifier: 'u.', activeTable: { table: 'users', alias: 'u' } }),
      snapshot,
      profile,
      opts,
    )
    expect(labels(result)).toEqual(['created_at', 'email', 'id'])
    const col = result.find(r => r.label === 'email')
    expect(col?.kind).toBe('column')
    expect(col?.detail).toBe('text')
  })

  it('(c2) after alias dot with word prefix filters columns', () => {
    const result = buildSuggestions(
      ctx({ word: 'cre', isAfterDot: true, qualifier: 'u.', activeTable: { table: 'users', alias: 'u' } }),
      snapshot,
      profile,
      opts,
    )
    expect(labels(result)).toEqual(['created_at'])
  })

  it('(d) after schema dot → tables of that schema', () => {
    const result = buildSuggestions(
      ctx({ word: '', isAfterDot: true, qualifier: 'analytics.' }),
      snapshot,
      profile,
      { ...opts, currentSchema: 'public' },
    )
    expect(labels(result)).toContain('events')
  })

  it('(d2) schema-qualified word → schema tables, never the FROM-table columns', () => {
    // Regression: `public.us` must offer tables in `public`, not columns of
    // the last FROM table (`users`) — activeTable is null for unmatched
    // qualifiers, so the builder falls through to the schema branch.
    const snap: SchemaSnapshot = {
      ...snapshot,
      tablesByKey: { ...snapshot.tablesByKey, 'app.public': ['users', 'orders', 'user_events'] },
      columnsByTable: {
        ...snapshot.columnsByTable,
        'c1|app|public|users': [
          { name: 'id', dataType: 'int4' },
          { name: 'user_id', dataType: 'int4' },
        ],
      },
    }
    const result = buildSuggestions(
      ctx({
        word: 'user',
        isAfterDot: true,
        qualifier: 'public.',
        tableRefs: [{ table: 'users', alias: 'u' }],
        activeTable: null,
      }),
      snap,
      profile,
      opts,
    )
    expect(labels(result)).toEqual(['user_events', 'users'])
    expect(result.every(r => r.kind !== 'column')).toBe(true)
  })

  it('(e) noParenFunctions preserved: NOW without parens, CONCAT with', () => {
    const result = buildSuggestions(ctx({ word: 'N' }), snapshot, profile, opts)
    const now = result.find(r => r.label === 'NOW')
    expect(now?.insertText).toBe('NOW')
    const concat = buildSuggestions(ctx({ word: 'CONC' }), snapshot, profile, opts).find(r => r.label === 'CONCAT')
    expect(concat?.insertText).toBe('CONCAT()')
  })

  it('(f) prefix filters tables', () => {
    const result = buildSuggestions(
      ctx({ word: 'us', tableRefs: [{ table: 'users' }], activeTable: { table: 'users' } }),
      snapshot,
      profile,
      opts,
    )
    expect(labels(result)).toContain('users')
    expect(labels(result)).not.toContain('orders')
  })

  it('(g) empty schema data → keywords still present (graceful degradation)', () => {
    const empty: SchemaSnapshot = { ...snapshot, hasSchemaData: false, databases: [], schemasByDb: {}, tablesByKey: {}, columnsByTable: {} }
    const result = buildSuggestions(ctx({ word: 'SEL' }), empty, profile, opts)
    expect(labels(result)).toContain('SELECT')
    expect(labels(result)).not.toContain('users')
  })

  it('(g2) after dot with empty snapshot → empty, no throw', () => {
    const empty: SchemaSnapshot = { ...snapshot, hasSchemaData: false, databases: [], schemasByDb: {}, tablesByKey: {}, columnsByTable: {} }
    const result = buildSuggestions(ctx({ word: '', isAfterDot: true, qualifier: 'x.' }), empty, profile, opts)
    expect(result).toEqual([])
  })

  it('(h) >100 candidates → exactly 100', () => {
    const bigSnapshot: SchemaSnapshot = {
      ...snapshot,
      tablesByKey: { app: Array.from({ length: 200 }, (_, i) => `table_${i}`) },
    }
    const result = buildSuggestions(ctx({ word: 'table_' }), bigSnapshot, profile, opts)
    expect(result.length).toBeLessThanOrEqual(100)
    expect(result.length).toBe(100)
  })

  it('in comment → no suggestions', () => {
    const result = buildSuggestions(ctx({ word: 'SEL', inComment: true }), snapshot, profile, opts)
    expect(result).toEqual([])
  })

  it('no keywords after a dot (only objects)', () => {
    const result = buildSuggestions(
      ctx({ word: '', isAfterDot: true, qualifier: 'u.', activeTable: { table: 'users', alias: 'u' } }),
      snapshot,
      profile,
      opts,
    )
    expect(result.every(r => r.kind !== 'keyword')).toBe(true)
  })
})
