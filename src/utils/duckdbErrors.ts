/**
 * Maps raw DuckDB connection failures onto a short explanation (and, when it is
 * actionable, the action that fixes them).
 *
 * DuckDB has a small set of very characteristic failures — exclusive file locks,
 * instance-scoped configuration, one-way storage-format compatibility — and users
 * hit them exactly when they create and share files locally. Signatures were
 * verified against `duckdb_jdbc` 1.5.6 (see scripts/verify-duckdb-jdbc-urls.sh).
 */

export type DuckDbFailureKind
  = | 'read-only-unsupported'
    | 'storage-version'
    | 'config-conflict'
    | 'lock-conflict'
    | 'unsupported-url'
    | 'extension'
    | 'read-only-file'
    | 'missing-file'

export type DuckDbFailureHint = {
  kind: DuckDbFailureKind
  /** i18n key of the user-facing explanation. */
  messageKey: string
  /** Action the UI can offer inline, when one exists. */
  action?: 'use-read-only' | 'use-read-write'
}

type Signature = {
  kind: DuckDbFailureKind
  patterns: string[]
  action?: 'use-read-only' | 'use-read-write'
  /**
   * Generic patterns (e.g. "no such file or directory") also appear in other
   * engines' errors, so they only count when the message mentions DuckDB.
   */
  generic?: boolean
}

const DUCKDB_CONTEXT = /duckdb/i

const SIGNATURES: Signature[] = [
  {
    // DuckDB refuses setReadOnly() at connection level, so a JDBC bridge that
    // does not align HikariCP's read-only flag with the URL fails on a read-only
    // connection. Fixed in the bridge; reported clearly until it ships.
    kind: 'read-only-unsupported',
    patterns: [
      'can\'t change read-only status',
      'cannot change read-only status',
    ],
    action: 'use-read-write',
  },
  {
    kind: 'storage-version',
    patterns: [
      'serialization error',
      'failed to deserialize',
      'storage version',
      'incompatible storage',
      'created with a newer version',
      'written by a newer version',
    ],
  },
  {
    kind: 'config-conflict',
    patterns: [
      'different configuration than existing connections',
      'same database file with a different configuration',
    ],
    action: 'use-read-only',
  },
  {
    kind: 'lock-conflict',
    patterns: [
      'could not set lock',
      'conflicting lock is held',
      'file is locked',
      'being used by another process',
      'already open in another',
    ],
    action: 'use-read-only',
  },
  {
    kind: 'unsupported-url',
    patterns: ['invalid url entry', 'splits the connection url', 'database path containing'],
  },
  {
    kind: 'extension',
    patterns: [
      'failed to download extension',
      'unable to load extension',
      'extension not found',
      'extension "',
    ],
    generic: true,
  },
  {
    kind: 'read-only-file',
    patterns: [
      'read-only file system',
      'attempting to write to a read-only',
      'cannot write to a read-only',
      'read-only database',
    ],
    generic: true,
  },
  {
    kind: 'missing-file',
    patterns: [
      'no such file or directory',
      'cannot open file',
      'does not exist',
      'no such database',
    ],
    generic: true,
  },
]

/** Explain a DuckDB connection failure, or return `null` when unrecognised. */
export function classifyDuckDbFailure(message: string | null | undefined): DuckDbFailureHint | null {
  const text = (message ?? '').toLowerCase()
  if (!text)
    return null

  const mentionsDuckDb = DUCKDB_CONTEXT.test(text)

  const match = SIGNATURES.find(signature =>
    (mentionsDuckDb || !signature.generic)
    && signature.patterns.some(pattern => text.includes(pattern)),
  )
  if (!match)
    return null

  return {
    kind: match.kind,
    messageKey: `components.serverForm.duckdbFailures.${match.kind}`,
    action: match.action,
  }
}
