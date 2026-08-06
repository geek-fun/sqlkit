/**
 * Domain model for SQLKit's schema-aware autocompletion.
 *
 * This module is deliberately framework-free (no monaco, no tauri, no vue):
 * it is imported by every other layer and must stay testable in a plain
 * jest node environment.
 */

/** Monaco language ids SQLKit registers completion for. */
export type SQLDialect = 'sql' | 'mysql' | 'pgsql' | 'mssql' | 'plsql' | 'sqlite'

/** SQL dialects that ship a monaco grammar contribution (have highlighting). */
export const GRAMMAR_DIALECTS: readonly SQLDialect[] = ['sql', 'mysql', 'pgsql']

/** A table (or alias) referenced by the current statement. */
export type TableRef = {
  /** Table name, unquoted and normalized (case preserved). */
  table: string
  /** User-provided alias if any (`FROM users u` → 'u'). */
  alias?: string
  /** Schema qualifier if present (`FROM public.users` → 'public'). */
  schema?: string
}

/** What the user is typing right now and where. */
export type CompletionContext = {
  /** The raw word being typed (already typed characters of the current identifier). */
  word: string
  /** The current statement's table references in order of appearance. */
  tableRefs: TableRef[]
  /** Table the cursor is scoped to: alias/table match on a single-segment qualifier, else the last FROM/JOIN table in bare-column context; null for unmatched schema/db qualifiers (`public.`). */
  activeTable: TableRef | null
  /** Qualifier prefix before the word, e.g. 'u.' or 'public.' or 'db.public.'. */
  qualifier: string
  /** True when typing after a dot (only object suggestions should apply). */
  isAfterDot: boolean
  /** True when cursor is inside a comment (line or block) → no suggestions. */
  inComment: boolean
}

/** Column metadata as surfaced for completion. */
export type ColumnSuggestion = {
  name: string
  dataType?: string
  isPrimaryKey?: boolean
}

/** A database. */
export type DatabaseRef = {
  name: string
  isSystem: boolean
}

/** Schema-level view of a connection's objects (built from databaseStore + column cache). */
export type SchemaSnapshot = {
  databases: DatabaseRef[]
  /** databases → schemas. */
  schemasByDb: Record<string, string[]>
  /** `db` or `db.schema` → tables. */
  tablesByKey: Record<string, string[]>
  /** `connId|db|schema|table` → columns. */
  columnsByTable: Record<string, ColumnSuggestion[]>
  /** Derived (document) tables: alias or CTE name → real table name. */
  derivedTables: Record<string, string>
  /** Whether the snapshot has any database-level data (false → keyword-only). */
  hasSchemaData: boolean
}

export type SuggestionKind
  = | 'keyword'
    | 'function'
    | 'type'
    | 'table'
    | 'column'
    | 'schema'
    | 'database'

/** A single completion suggestion (layer-agnostic; mapped to monaco at the edge). */
export type Suggestion = {
  label: string
  kind: SuggestionKind
  insertText: string
  detail?: string
  documentation?: string
  /** Higher = ranked earlier when labels tie. */
  sortPrefix?: number
}

/** Per-dialect data used to converge grammar differences without code branches. */
export type DialectProfile = {
  id: SQLDialect
  /** Character used to quote identifiers (`"` for pgsql/sql, backtick for mysql). */
  quoteChar: string
  /** Whether schema-qualified names (`schema.table`) are typical for this dialect. */
  supportsSchemaQualification: boolean
  keywords: string[]
  functions: string[]
  types: string[]
  /** Functions inserted WITHOUT trailing parens (e.g. NOW, CURRENT_DATE). */
  noParenFunctions: string[]
}

/** Completion context required from the host page (per editor/tab). */
export type CompletionContextInput = {
  connectionId?: string
  database?: string | null
  schema?: string | null
  dialectId?: SQLDialect
}
