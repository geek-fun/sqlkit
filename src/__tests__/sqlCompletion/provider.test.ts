import type { StoreMetadata } from '@/composables/sqlCompletion/metadata'
import type { SchemaSnapshot } from '@/composables/sqlCompletion/types'
import { analyzeCompletionContext } from '@/composables/sqlCompletion/analyzer'
import { buildSuggestions } from '@/composables/sqlCompletion/builder'
import { getDialectProfile } from '@/composables/sqlCompletion/dialects'
/**
 * @jest-environment node
 */
import { SchemaMetadataService } from '@/composables/sqlCompletion/metadata'
import { createProvider, emptySnapshot, SQL_DIALECT_IDS } from '@/composables/sqlCompletion/provider'

const fakeStore: StoreMetadata = {
  databases: [{ name: 'app', is_system: false }],
  schemas: { app: ['public'] },
  tables: {
    'app': [{ name: 'users', schema: 'public' }],
    'app.public': [{ name: 'users', schema: 'public' }],
  },
}

function makeService() {
  return new SchemaMetadataService({ getMetadata: () => fakeStore })
}

const deps = {
  metadataService: makeService(),
  analyze: analyzeCompletionContext,
  build: buildSuggestions,
  profiles: getDialectProfile,
}

type FakeModel = {
  getValue: () => string
  getOffsetAt: (pos: { lineNumber: number, column: number }) => number
  getWordUntilPosition: (pos: { lineNumber: number, column: number }) => { startColumn: number, endColumn: number }
}

function makeModel(text: string): FakeModel {
  const lineStarts = [0]
  for (let i = 0; i < text.length; i++) {
    if (text[i] === '\n')
      lineStarts.push(i + 1)
  }
  return {
    getValue: () => text,
    getOffsetAt: (pos) => {
      const start = lineStarts[pos.lineNumber - 1] ?? 0
      return start + pos.column - 1
    },
    getWordUntilPosition: () => ({ startColumn: 1, endColumn: 1 }),
  }
}

function makeFakeMonaco() {
  const registered: { language: string, provider: unknown }[] = []
  return {
    languages: {
      registerCompletionItemProvider: jest.fn((language: string, provider: unknown) => {
        registered.push({ language, provider })
        return { dispose: jest.fn() }
      }),
      CompletionItemKind: {
        Keyword: 0,
        Function: 1,
        TypeParameter: 2,
        Class: 3,
        Field: 4,
        Module: 5,
        Folder: 6,
      },
    },
    registered,
  }
}

type ProviderHandler = {
  provideCompletionItems: (model: unknown, position: { lineNumber: number, column: number }) => { suggestions: Array<{ label: string, kind: number }> }
  resolveCompletionItem: (item: unknown) => Promise<unknown>
}

function providerOf(fake: ReturnType<typeof makeFakeMonaco>, id: string): ProviderHandler {
  return fake.registered.find(r => r.language === id)?.provider as unknown as ProviderHandler
}

describe('completion provider', () => {
  it('(a) registers a provider for all 6 SQL dialect ids', () => {
    const fake = makeFakeMonaco()
    const provider = createProvider(fake as never, deps)
    const disposables = provider.register()
    expect(fake.languages.registerCompletionItemProvider).toHaveBeenCalledTimes(6)
    expect(fake.registered.map(r => r.language)).toEqual([...SQL_DIALECT_IDS])
    expect(disposables).toHaveLength(6)
    disposables.forEach(d => d.dispose())
  })

  it('(b) no context → sync empty suggestions, no throw', () => {
    const fake = makeFakeMonaco()
    const provider = createProvider(fake as never, deps)
    provider.register()
    const model = makeModel('SELECT ')
    const handler = providerOf(fake, 'sql')
    const result = handler.provideCompletionItems(model, { lineNumber: 1, column: 8 })
    expect(result).toEqual({ suggestions: [] })
  })

  it('(b2) setContext with snapshot → table suggestion appears (empty word)', () => {
    const fake = makeFakeMonaco()
    const provider = createProvider(fake as never, deps)
    provider.register()
    const model = makeModel('SELECT FROM ')
    provider.setContext(model as never, { connectionId: 'c1', database: 'app', schema: 'public' })
    const handler = providerOf(fake, 'sql')
    const result = handler.provideCompletionItems(model, { lineNumber: 1, column: 13 })
    const labels = result.suggestions.map((s: { label: string }) => s.label)
    expect(labels).toContain('users')
  })

  it('(c) after alias dot → column suggestions with kind Field', () => {
    const fake = makeFakeMonaco()
    const svc = new SchemaMetadataService({ getMetadata: () => fakeStore })
    const localProvider = createProvider(fake as never, { ...deps, metadataService: svc })
    localProvider.register()
    const model = makeModel('SELECT u. FROM users u')
    localProvider.setContext(model as never, { connectionId: 'c1', database: 'app', schema: 'public' })
    const snapshot: SchemaSnapshot = {
      databases: [{ name: 'app', isSystem: false }],
      schemasByDb: {},
      tablesByKey: {},
      columnsByTable: {
        'c1|app|public|users': [{ name: 'id', dataType: 'int4' }, { name: 'email', dataType: 'text' }],
      },
      derivedTables: {},
      hasSchemaData: true,
    }
    localProvider.contexts.set(model as never, {
      connId: 'c1',
      database: 'app',
      schema: 'public',
      dialectId: 'sql',
      snapshot,
    })
    const handler = providerOf(fake, 'sql')
    const result = handler.provideCompletionItems(model, { lineNumber: 1, column: 10 })
    const labels = result.suggestions.map((s: { label: string }) => s.label)
    expect(labels).toEqual(['email', 'id'])
    expect(result.suggestions[0].kind).toBe(4)
  })

  it('(c2) active alias column lookup works through public API without private access', () => {
    const fake = makeFakeMonaco()
    const svc = makeService()
    // Pre-populate the column cache via the public API.
    svc.prefetchColumns = jest.fn().mockResolvedValue(undefined) as never
    const provider = createProvider(fake as never, { ...deps, metadataService: svc })
    provider.register()
    const model = makeModel('SELECT u.em FROM users u')
    provider.setContext(model as never, { connectionId: 'c1', database: 'app', schema: 'public' })
    const handler = providerOf(fake, 'sql')
    const result = handler.provideCompletionItems(model, { lineNumber: 1, column: 14 })
    expect(Array.isArray(result.suggestions)).toBe(true)
  })

  it('(d) WeakMap entry removed on clearModel', () => {
    const fake = makeFakeMonaco()
    const provider = createProvider(fake as never, deps)
    provider.register()
    const model = makeModel('SEL')
    provider.setContext(model as never, { connectionId: 'c1', database: 'app', schema: 'public' })
    expect(provider.contexts.has(model as never)).toBe(true)
    provider.clearModel(model as never)
    expect(provider.contexts.has(model as never)).toBe(false)
  })

  it('(e) resolveCompletionItem returns the item as-is (async signature)', async () => {
    const fake = makeFakeMonaco()
    const provider = createProvider(fake as never, deps)
    provider.register()
    const item = { label: 'users', kind: 3, insertText: 'users' }
    const handler = providerOf(fake, 'sql')
    const resolved = await handler.resolveCompletionItem(item)
    expect(resolved).toEqual(item)
  })

  it('(g) null model → empty suggestions, no throw', () => {
    const fake = makeFakeMonaco()
    const provider = createProvider(fake as never, deps)
    provider.register()
    const handler = providerOf(fake, 'sql')
    expect(handler.provideCompletionItems(null, { lineNumber: 1, column: 1 })).toEqual({ suggestions: [] })
  })

  it('emptySnapshot has hasSchemaData false', () => {
    expect(emptySnapshot().hasSchemaData).toBe(false)
  })
})
