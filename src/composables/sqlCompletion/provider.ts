/**
 * CompletionProvider (Layer 4) — monaco-facing wiring.
 *
 * Singleton that registers a completion provider for every SQLDialect id.
 * provideCompletionItems is fully SYNC (no per-keystroke backend call); only
 * resolveCompletionItem is async (augments documentation, currently from
 * in-memory snapshot — async signature reserved for a future LSP escape).
 *
 * The factory createProvider(monacoLike, deps) lets unit tests inject a fake
 * monaco; the singleton uses the real monaco-editor ESM API.
 */

import type * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
import type { SchemaMetadataService } from './metadata'
import type { CompletionContextInput, SchemaSnapshot, SQLDialect, Suggestion } from './types'
import { analyzeCompletionContext } from './analyzer'
import { buildSuggestions } from './builder'
import { getDialectProfile } from './dialects'
import { getMetadataService } from './metadata'

export const SQL_DIALECT_IDS: readonly SQLDialect[] = ['sql', 'mysql', 'pgsql', 'mssql', 'plsql', 'sqlite']

/** Per-editor identity + snapshot, keyed by the backing ITextModel. */
export type ModelContext = {
  connId: string
  database: string | null
  schema: string | null
  dialectId: SQLDialect
  snapshot: SchemaSnapshot
}

export type ProviderDeps = {
  metadataService: SchemaMetadataService
  analyze: typeof analyzeCompletionContext
  build: typeof buildSuggestions
  profiles: typeof getDialectProfile
}

export function emptySnapshot(): SchemaSnapshot {
  return {
    databases: [],
    schemasByDb: {},
    tablesByKey: {},
    columnsByTable: {},
    derivedTables: {},
    hasSchemaData: false,
  }
}

export function createProvider(monacoLike: typeof monaco, deps: ProviderDeps) {
  const KIND_MAP: Record<Suggestion['kind'], monaco.languages.CompletionItemKind> = {
    keyword: monacoLike.languages.CompletionItemKind.Keyword,
    function: monacoLike.languages.CompletionItemKind.Function,
    type: monacoLike.languages.CompletionItemKind.TypeParameter,
    table: monacoLike.languages.CompletionItemKind.Class,
    column: monacoLike.languages.CompletionItemKind.Field,
    schema: monacoLike.languages.CompletionItemKind.Module,
    database: monacoLike.languages.CompletionItemKind.Folder,
  }
  const contexts = new WeakMap<monaco.editor.ITextModel, ModelContext>()

  const makeSnapshot = (input: CompletionContextInput, metadata: SchemaMetadataService): SchemaSnapshot => {
    const connId = input.connectionId ?? ''
    const db = input.database ?? null
    const schema = input.schema ?? null

    const databases = metadata.getDatabases(connId)
    const schemasByDb: Record<string, string[]> = {}
    for (const d of databases) {
      const schemas = metadata.getSchemas(connId, d.name)
      if (schemas.length > 0) {
        schemasByDb[d.name] = schemas
      }
    }

    const tablesByKey: Record<string, string[]> = {}
    const addTables = (key: string, tables: string[]) => {
      if (tables.length > 0) {
        tablesByKey[key] = tables
      }
    }
    if (db) {
      if (schema) {
        addTables(`${db}.${schema}`, metadata.getTables(connId, db, schema))
      }
      else {
        const unqualified = metadata.getTables(connId, db)
        const qualified = metadata.getSchemas(connId, db).map(s => metadata.getTables(connId, db, s)).flat()
        addTables(db, [...new Set([...unqualified, ...qualified])])
      }
    }

    return {
      databases,
      schemasByDb,
      tablesByKey,
      columnsByTable: metadata.getCachedColumns(connId),
      derivedTables: {},
      hasSchemaData: databases.length > 0,
    }
  }

  const provideCompletionItems = (
    model: monaco.editor.ITextModel,
    position: monaco.Position,
  ): monaco.languages.CompletionList => {
    if (!model || !position) {
      return { suggestions: [] }
    }
    const modelCtx = contexts.get(model)
    if (!modelCtx) {
      return { suggestions: [] }
    }

    const offset = model.getOffsetAt(position)
    const analysis = deps.analyze(model.getValue(), offset)
    const dialect = deps.profiles(modelCtx.dialectId)
    const suggestions = deps.build(analysis, modelCtx.snapshot, dialect, {
      connectionId: modelCtx.connId,
      currentDb: modelCtx.database,
      currentSchema: modelCtx.schema,
    })

    const word = model.getWordUntilPosition(position)
    const range = {
      startLineNumber: position.lineNumber,
      endLineNumber: position.lineNumber,
      startColumn: word.startColumn,
      endColumn: word.endColumn,
    }

    return {
      suggestions: suggestions.map(s => ({
        label: s.label,
        kind: KIND_MAP[s.kind],
        insertText: s.insertText,
        detail: s.detail,
        documentation: s.documentation,
        range,
      })),
    }
  }

  const resolveCompletionItem = async (
    item: monaco.languages.CompletionItem,
  ): Promise<monaco.languages.CompletionItem> => {
    return item
  }

  const register = (): monaco.IDisposable[] => {
    return SQL_DIALECT_IDS.map(id =>
      monacoLike.languages.registerCompletionItemProvider(id, {
        provideCompletionItems,
        resolveCompletionItem,
      }),
    )
  }

  const setContext = (model: monaco.editor.ITextModel | null, input: CompletionContextInput) => {
    if (!model) {
      return
    }
    const metadata = deps.metadataService
    const connId = input.connectionId ?? ''
    contexts.set(model, {
      connId,
      database: input.database ?? null,
      schema: input.schema ?? null,
      dialectId: input.dialectId ?? 'sql',
      snapshot: makeSnapshot(input, metadata),
    })
  }

  const clearModel = (model: monaco.editor.ITextModel | null) => {
    if (model) {
      contexts.delete(model)
    }
  }

  const dispose = () => {
    // contexts is a WeakMap; nothing to release eagerly.
  }

  return {
    register,
    setContext,
    clearModel,
    provideCompletionItems,
    resolveCompletionItem,
    dispose,
    contexts,
  }
}

export type CompletionProvider = ReturnType<typeof createProvider>

let singleton: CompletionProvider | null = null
let registrationDisposables: monaco.IDisposable[] | null = null

/** Memoized singleton. The real monaco must be passed on first call (from useMonacoEditor) to register providers. */
export function getCompletionProvider(monacoLike: typeof monaco): CompletionProvider {
  if (!singleton) {
    const provider = createProvider(monacoLike, {
      metadataService: getMetadataService(),
      analyze: analyzeCompletionContext,
      build: buildSuggestions,
      profiles: getDialectProfile,
    })
    registrationDisposables = provider.register()
    singleton = {
      ...provider,
      register: () => registrationDisposables ?? [],
      dispose: () => {
        registrationDisposables?.forEach(d => d.dispose())
        registrationDisposables = null
        singleton = null
      },
    }
  }
  return singleton
}

export function disposeCompletionProvider() {
  singleton?.dispose()
  singleton = null
}
