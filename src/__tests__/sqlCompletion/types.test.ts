import type {
  ColumnSuggestion,
  CompletionContext,
  CompletionContextInput,
  DatabaseRef,
  DialectProfile,
  SchemaSnapshot,
  Suggestion,
  TableRef,
} from '@/composables/sqlCompletion/types'
/**
 * @jest-environment node
 */
import { GRAMMAR_DIALECTS } from '@/composables/sqlCompletion/types'

describe('sqlCompletion domain types', () => {
  it('satisfies the CompletionContext shape', () => {
    const ctx: CompletionContext = {
      word: 'us',
      tableRefs: [{ table: 'users', alias: 'u' }],
      activeTable: { table: 'users', alias: 'u' },
      qualifier: 'u.',
      isAfterDot: true,
      inComment: false,
    }
    expect(ctx.word).toBe('us')
    expect(ctx.isAfterDot).toBe(true)
  })

  it('satisfies the SchemaSnapshot shape with column cache entries', () => {
    const snapshot: SchemaSnapshot = {
      databases: [{ name: 'app', isSystem: false }],
      schemasByDb: { app: ['public'] },
      tablesByKey: { 'app': ['users'], 'app.public': ['orders'] },
      columnsByTable: {
        'c1|app|public|orders': [{ name: 'id', dataType: 'int4', isPrimaryKey: true }],
      },
      derivedTables: { x: 'users' },
      hasSchemaData: true,
    }
    expect(snapshot.tablesByKey['app.public']).toContain('orders')
  })

  it('satisfies the DialectProfile shape with noParenFunctions', () => {
    const profile: DialectProfile = {
      id: 'pgsql',
      quoteChar: '"',
      supportsSchemaQualification: true,
      keywords: ['SELECT'],
      functions: ['NOW'],
      types: ['INT'],
      noParenFunctions: ['NOW', 'CURRENT_DATE'],
    }
    expect(profile.noParenFunctions).toContain('NOW')
  })

  it('satisfies Suggestion / TableRef / DatabaseRef / ColumnSuggestion / CompletionContextInput shapes', () => {
    const s: Suggestion = { label: 'users', kind: 'table', insertText: 'users', sortPrefix: 1 }
    const t: TableRef = { table: 'users', alias: 'u', schema: 'public' }
    const d: DatabaseRef = { name: 'app', isSystem: false }
    const c: ColumnSuggestion = { name: 'id', dataType: 'int4', isPrimaryKey: true }
    const input: CompletionContextInput = { connectionId: 'c1', database: 'app', schema: 'public' }
    expect([s.kind, t.table, d.name, c.name, input.connectionId]).toEqual(['table', 'users', 'app', 'id', 'c1'])
  })

  it('gRAMMAR_DIALECTS contains exactly the dialects with monaco grammar contributions', () => {
    expect(GRAMMAR_DIALECTS).toEqual(['sql', 'mysql', 'pgsql'])
  })
})
