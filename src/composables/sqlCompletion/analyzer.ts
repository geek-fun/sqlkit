/**
 * ContextAnalyzer (Layer 1) — lexical analysis of the document at the cursor.
 *
 * Deliberately NOT a SQL parser: it uses regex/scanning to answer the only
 * questions the builder needs:
 *   - what word is being typed
 *   - are we after a qualifier dot (alias./schema./db.schema.)
 *   - which tables are referenced in the current statement (FROM/JOIN) with aliases
 *   - is the cursor inside a comment (→ no suggestions)
 *
 * Pure module — no monaco/tauri imports (jest node-testable).
 */

import type { CompletionContext, TableRef } from './types'

// Identifier: unicode letters/digits/_/$ plus dots for qualified names, and
// double-quoted / backtick-quoted segments (e.g. "my table", `my-table`).
const IDENT_CHAR = /[\p{L}\p{N}_$]/u
const QUOTE_CHARS = ['"', '`']

type StatementSpan = { start: number, end: number }

/**
 * Split the document into statements on ; while tracking quotes and comments,
 * returning the span containing `offset`.
 */
function findContainingStatement(text: string, offset: number): StatementSpan {
  const state = { quote: '', inLineComment: false, inBlockComment: false }
  let stmtStart = 0

  for (let i = 0; i < text.length; i++) {
    const ch = text[i]
    const next = text[i + 1]

    if (state.inLineComment) {
      if (ch === '\n') {
        state.inLineComment = false
      }
    }
    else if (state.inBlockComment) {
      if (ch === '*' && next === '/') {
        state.inBlockComment = false
        i++
      }
    }
    else if (state.quote) {
      if (ch === state.quote) {
        if (next === state.quote) {
          i++ // escaped quote inside identifier
        }
        else {
          state.quote = ''
        }
      }
    }
    else if (ch === '-' && next === '-') {
      state.inLineComment = true
      i++
    }
    else if (ch === '/' && next === '*') {
      state.inBlockComment = true
      i++
    }
    else if (QUOTE_CHARS.includes(ch)) {
      state.quote = ch
    }
    else if (ch === ';') {
      // Statement boundary — but only if the offset is past it.
      if (offset >= i) {
        stmtStart = i + 1
      }
      else {
        return { start: stmtStart, end: i }
      }
    }
  }

  return { start: stmtStart, end: text.length }
}

/** True when `offset` is inside a line/block comment (already statement-scoped). */
function isInsideComment(text: string, start: number, _end: number, offset: number): boolean {
  const prefix = text.slice(start, offset)
  let inLine = false
  let inBlock = false
  let quote = ''

  for (let i = 0; i < prefix.length; i++) {
    const ch = prefix[i]
    const next = prefix[i + 1]
    if (inLine) {
      if (ch === '\n')
        inLine = false
    }
    else if (inBlock) {
      if (ch === '*' && next === '/') {
        inBlock = false
        i++
      }
    }
    else if (quote) {
      if (ch === quote) {
        if (next === quote)
          i++
        else
          quote = ''
      }
    }
    else if (ch === '-' && next === '-') {
      inLine = true
      i++
    }
    else if (ch === '/' && next === '*') {
      inBlock = true
      i++
    }
    else if (QUOTE_CHARS.includes(ch)) {
      quote = ch
    }
  }
  return inLine || inBlock
}

/** Strip quotes from a quoted identifier and collapse doubled quote chars. */
function unquote(name: string): string {
  if (name.length >= 2 && QUOTE_CHARS.includes(name[0]) && name[name.length - 1] === name[0]) {
    const q = name[0]
    const inner = name.slice(1, -1)
    return inner.split(q + q).join(q)
  }
  return name
}

/** Replace comment spans with spaces (same length) so regex scanning skips them. */
function maskComments(text: string, start: number, end: number): string {
  const out = text.split('')
  let inLine = false
  let inBlock = false
  let quote = ''
  for (let i = start; i < end; i++) {
    const ch = text[i]
    const next = text[i + 1]
    if (inLine) {
      out[i] = ' '
      if (ch === '\n')
        inLine = false
    }
    else if (inBlock) {
      out[i] = ' '
      if (ch === '*' && next === '/') {
        out[i + 1] = ' '
        inBlock = false
        i++
      }
    }
    else if (quote) {
      if (ch === quote) {
        if (next === quote) {
          i++
        }
        else {
          quote = ''
        }
      }
    }
    else if (ch === '-' && next === '-') {
      inLine = true
      out[i] = ' '
      out[i + 1] = ' '
      i++
    }
    else if (ch === '/' && next === '*') {
      inBlock = true
      out[i] = ' '
      out[i + 1] = ' '
      i++
    }
    else if (QUOTE_CHARS.includes(ch)) {
      quote = ch
    }
  }
  return out.join('')
}

/**
 * Extract table references from a statement via FROM/JOIN scanning.
 * Comments are masked first so `-- FROM foo` never yields a table ref.
 * Handles: FROM t, FROM t alias, FROM t AS alias, FROM schema.t, quoted names,
 * JOIN variants.
 */
function extractTableRefs(text: string, start: number, end: number): TableRef[] {
  const refs: TableRef[] = []
  const stmt = maskComments(text, start, end).slice(start, end)
  // Match FROM/JOIN followed by a (possibly quoted/qualified) identifier,
  // with an optional AS-alias or bare alias.
  const re = /\b(?:from|join)\s+((?:"[^"]*"|`[^`]*`|[\p{L}\p{N}_$]+)(?:\s*\.\s*(?:"[^"]*"|`[^`]*`|[\p{L}\p{N}_$]+))*)\s*(?:as\s+)?([\p{L}\p{N}_$]+)?/giu
  let m: RegExpExecArray | null
  while (true) {
    m = re.exec(stmt)
    if (m === null)
      break
    const rawName = m[1].trim()
    const rawAlias = m[2]

    // Parse schema.table / table out of the raw name (respecting quotes).
    const parts = rawName.match(/"[^"]*"|`[^`]*`|[\p{L}\p{N}_$]+/gu) ?? []
    const names = parts.map(unquote)
    const table = names.length > 0 ? names[names.length - 1] : ''
    const schema = names.length > 1 ? names[names.length - 2] : undefined
    if (!table)
      continue

    const alias = rawAlias ? unquote(rawAlias) : undefined
    // A bare-alias capture that is actually a keyword (e.g. `FROM a JOIN b`
    // grabbing 'JOIN') means there is no alias — rewind so the next FROM/JOIN
    // in the statement is matched.
    if (alias && /^(?:on|where|group|order|having|limit|offset|union|set|values|select|left|right|inner|outer|join|as|and|or|not|when|then|else|end)$/i.test(alias)) {
      refs.push({ table, schema })
      re.lastIndex = m.index + rawName.length + 1
      continue
    }

    refs.push({ table, schema, alias })
  }
  return refs
}

/**
 * Analyze the document at `offset` and produce the completion context.
 *
 * @param text full document text
 * @param offset character offset of the cursor
 */
export function analyzeCompletionContext(text: string, offset: number): CompletionContext {
  const clampedOffset = Math.max(0, Math.min(offset, text.length))
  const span = findContainingStatement(text, clampedOffset)
  const inComment = isInsideComment(text, span.start, span.end, clampedOffset)

  // Word being typed: scan backwards from cursor over identifier chars.
  let wordStart = clampedOffset
  while (wordStart > span.start && IDENT_CHAR.test(text[wordStart - 1])) {
    wordStart--
  }
  const word = text.slice(wordStart, clampedOffset)

  // Qualifier: scan backwards over `word.` / `schema.` / `db.schema.` segments.
  let qualifier = ''
  let qualifierStart = wordStart
  while (true) {
    // Skip optional whitespace? No — qualifiers are adjacent: `u.` or `db.`.
    const before = text.slice(span.start, qualifierStart)
    const m = before.match(/(?:"[^"]*"|`[^`]*`|[\p{L}\p{N}_$]+)\s*\.\s*$/u)
    if (!m)
      break
    const segStart = qualifierStart - m[0].length
    qualifier = text.slice(segStart, wordStart) // includes trailing dot(s)
    qualifierStart = segStart
  }
  const isAfterDot = qualifier.length > 0

  let tableRefs: TableRef[] = []
  if (!inComment) {
    tableRefs = extractTableRefs(text, span.start, span.end)
  }

  // activeTable: alias/table match on a single-segment qualifier (`u.`, `users.`);
  // otherwise the last FROM/JOIN table as the bare-column context table. A
  // qualifier matching neither an alias nor a table (e.g. `public.us`) must NOT
  // fall back to a FROM table — the builder would return that table's columns
  // instead of the schema/table objects the user is completing.
  let activeTable: TableRef | null = null
  if (isAfterDot && tableRefs.length > 0) {
    const segments = qualifier.split('.').filter(Boolean)
    if (segments.length === 1) {
      const firstQualSeg = unquote(segments[0])
      const byAlias = tableRefs.find(r => r.alias && r.alias.toLowerCase() === firstQualSeg.toLowerCase())
      const byName = tableRefs.find(r => r.table.toLowerCase() === firstQualSeg.toLowerCase())
      activeTable = byAlias ?? byName ?? null
    }
  }
  if (!activeTable && !isAfterDot && tableRefs.length > 0) {
    activeTable = tableRefs[tableRefs.length - 1]
  }

  return {
    word,
    tableRefs,
    activeTable,
    qualifier,
    isAfterDot,
    inComment,
  }
}
