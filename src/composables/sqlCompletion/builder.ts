/**
 * SuggestionBuilder (Layer 3) — pure assembly of completion suggestions.
 *
 * Input: CompletionContext + SchemaSnapshot + DialectProfile + host context.
 * Output: Suggestion[] — no I/O, no monaco, no document access.
 */

import type { ColumnSuggestion, CompletionContext, DialectProfile, SchemaSnapshot, Suggestion, TableRef } from './types'

const MAX_SUGGESTIONS = 100

export type BuilderOptions = {
  connectionId: string
  currentDb: string | null
  currentSchema: string | null
}

const ORDER: Record<Suggestion['kind'], number> = {
  keyword: 0,
  function: 1,
  type: 2,
  table: 3,
  column: 4,
  schema: 5,
  database: 6,
}

function matchesPrefix(label: string, word: string): boolean {
  return label.toLowerCase().startsWith(word.toLowerCase())
}

/** Rank: exact prefix match (word at start) ranks higher than contains; sortPrefix breaks ties. */
function compareSuggestions(a: Suggestion, b: Suggestion): number {
  const orderDiff = ORDER[a.kind] - ORDER[b.kind]
  if (orderDiff !== 0)
    return orderDiff
  const sortDiff = (a.sortPrefix ?? 0) - (b.sortPrefix ?? 0)
  if (sortDiff !== 0)
    return sortDiff
  return a.label.localeCompare(b.label)
}

function keywordSuggestions(word: string, dialect: DialectProfile): Suggestion[] {
  const lower = word.toLowerCase()
  return dialect.keywords
    .filter(k => k.toLowerCase().startsWith(lower))
    .map(k => ({ label: k, kind: 'keyword' as const, insertText: k }))
}

function functionSuggestions(word: string, dialect: DialectProfile): Suggestion[] {
  const lower = word.toLowerCase()
  return dialect.functions
    .filter(f => f.toLowerCase().startsWith(lower))
    .map(f => ({
      label: f,
      kind: 'function' as const,
      insertText: dialect.noParenFunctions.includes(f) ? f : `${f}()`,
    }))
}

function typeSuggestions(word: string, dialect: DialectProfile): Suggestion[] {
  const lower = word.toLowerCase()
  return dialect.types
    .filter(t => t.toLowerCase().startsWith(lower))
    .map(t => ({ label: t, kind: 'type' as const, insertText: t }))
}

/**
 * Build completion suggestions for the given context.
 *
 * - After a dot → only object suggestions (columns of the active table when
 *   the qualifier is an alias/table; tables of a schema/db; databases when
 *   no qualifier match). No keywords after a dot.
 * - Otherwise → keywords + functions + types + tables + schemas + databases.
 */
export function buildSuggestions(
  ctx: CompletionContext,
  snapshot: SchemaSnapshot,
  dialect: DialectProfile,
  opts: BuilderOptions,
): Suggestion[] {
  if (ctx.inComment)
    return []

  if (ctx.isAfterDot) {
    return buildObjectSuggestions(ctx, snapshot, dialect, opts).slice(0, MAX_SUGGESTIONS)
  }

  // With an empty word, keyword/function/type suggestions are noise (they would
  // all match); only object names are useful, e.g. tables right after FROM.
  const lexical: Suggestion[] = ctx.word === ''
    ? []
    : [
        ...keywordSuggestions(ctx.word, dialect),
        ...functionSuggestions(ctx.word, dialect),
        ...typeSuggestions(ctx.word, dialect),
      ]
  const suggestions: Suggestion[] = [
    ...lexical,
    ...buildTableSuggestions(ctx, snapshot, opts),
    ...buildSchemaSuggestions(ctx, snapshot, opts),
    ...buildDatabaseSuggestions(ctx, snapshot),
  ]
  return suggestions.sort(compareSuggestions).slice(0, MAX_SUGGESTIONS)
}

function buildObjectSuggestions(
  ctx: CompletionContext,
  snapshot: SchemaSnapshot,
  dialect: DialectProfile,
  opts: BuilderOptions,
): Suggestion[] {
  const segments = ctx.qualifier.split('.').filter(Boolean)
  const first = segments[0] ?? ''

  // `alias.` or `table.` → columns of that table.
  const active = ctx.activeTable
  if (active) {
    const realName = snapshot.derivedTables[active.table] ?? active.table
    const columns = collectColumns(snapshot, opts.connectionId, opts.currentDb, opts.currentSchema, { ...active, table: realName })
    if (columns.length > 0) {
      return columns
        .filter(c => matchesPrefix(c.name, ctx.word))
        .map(c => ({
          label: c.name,
          kind: 'column' as const,
          insertText: c.name,
          detail: c.dataType,
          documentation: c.isPrimaryKey ? 'Primary key' : undefined,
        }))
        .sort(compareSuggestions)
    }
  }

  // `schema.` → tables of that schema (or db when no schema support).
  if (first && snapshot.hasSchemaData) {
    const db = opts.currentDb ?? snapshot.databases[0]?.name ?? ''
    if (db) {
      const schemaKey = dialect.supportsSchemaQualification && opts.currentSchema
        ? `${db}.${first}`
        : db
      const tables = snapshot.tablesByKey[schemaKey] ?? snapshot.tablesByKey[db] ?? []
      const filtered = tables.filter(t => matchesPrefix(t, ctx.word))
      if (filtered.length > 0 || segments.length >= 2) {
        return filtered
          .map(t => ({ label: t, kind: 'table' as const, insertText: t }))
          .sort(compareSuggestions)
      }
    }
  }

  // `db.` → schemas.
  if (segments.length === 1) {
    const schemas = snapshot.schemasByDb[first] ?? []
    return schemas
      .filter(s => matchesPrefix(s, ctx.word))
      .map(s => ({ label: s, kind: 'schema' as const, insertText: s }))
      .sort(compareSuggestions)
  }

  return []
}

function collectColumns(
  snapshot: SchemaSnapshot,
  connectionId: string,
  currentDb: string | null,
  currentSchema: string | null,
  active: TableRef,
): ColumnSuggestion[] {
  const db = currentDb ?? snapshot.databases[0]?.name ?? ''
  if (!db)
    return []
  const schema = active.schema ?? currentSchema ?? ''
  const key = `${connectionId}|${db}|${schema}|${active.table}`
  return snapshot.columnsByTable[key] ?? []
}

function buildTableSuggestions(ctx: CompletionContext, snapshot: SchemaSnapshot, opts: BuilderOptions): Suggestion[] {
  if (!snapshot.hasSchemaData)
    return []
  const db = opts.currentDb ?? snapshot.databases[0]?.name ?? ''
  if (!db)
    return []
  const key = opts.currentSchema && snapshot.tablesByKey[`${db}.${opts.currentSchema}`]
    ? `${db}.${opts.currentSchema}`
    : db
  const tables = snapshot.tablesByKey[key] ?? snapshot.tablesByKey[db] ?? []
  return tables
    .filter(t => matchesPrefix(t, ctx.word))
    .map(t => ({ label: t, kind: 'table' as const, insertText: t }))
}

function buildSchemaSuggestions(ctx: CompletionContext, snapshot: SchemaSnapshot, opts: BuilderOptions): Suggestion[] {
  const db = opts.currentDb ?? snapshot.databases[0]?.name ?? ''
  if (!db)
    return []
  return (snapshot.schemasByDb[db] ?? [])
    .filter(s => matchesPrefix(s, ctx.word))
    .map(s => ({ label: s, kind: 'schema' as const, insertText: s }))
}

function buildDatabaseSuggestions(ctx: CompletionContext, snapshot: SchemaSnapshot): Suggestion[] {
  return snapshot.databases
    .filter(d => matchesPrefix(d.name, ctx.word))
    .map(d => ({ label: d.name, kind: 'database' as const, insertText: d.name }))
}
