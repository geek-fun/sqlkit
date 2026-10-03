import type { DatabaseFileProbe } from '@/utils/databaseFiles'
import {
  assessDatabaseFile,
  buildMemoryDatabaseHost,
  createMemoryDatabaseLabel,
  formatFileSize,
  isMemoryDatabaseHost,
  isSameDatabaseFile,
  memoryDatabaseLabel,
  mergeRecentDatabase,
  normalizeComparablePath,
  recentDatabasesKey,
  storageVersionMinRelease,
  suggestDatabaseFileName,
  withDatabaseExtension,
} from '@/utils/databaseFiles'
import { classifyDuckDbFailure } from '@/utils/duckdbErrors'

function probe(overrides: Partial<DatabaseFileProbe> = {}): DatabaseFileProbe {
  return {
    path: '/tmp/app.duckdb',
    exists: false,
    isDirectory: false,
    sizeBytes: null,
    parentExists: true,
    parentWritable: true,
    writable: true,
    format: null,
    storageVersion: null,
    hasWal: false,
    ...overrides,
  }
}

describe('database file paths', () => {
  it('appends the engine extension only when the file name has none', () => {
    expect(withDatabaseExtension('/tmp/analytics', 'duckdb')).toBe('/tmp/analytics.duckdb')
    expect(withDatabaseExtension('/tmp/analytics.duckdb', 'duckdb')).toBe('/tmp/analytics.duckdb')
    expect(withDatabaseExtension('/tmp/analytics.db', 'duckdb')).toBe('/tmp/analytics.db')
    expect(withDatabaseExtension('/tmp/my.folder/analytics', 'duckdb')).toBe('/tmp/my.folder/analytics.duckdb')
    expect(withDatabaseExtension('C:\\data\\analytics', 'duckdb')).toBe('C:\\data\\analytics.duckdb')
    expect(withDatabaseExtension('   ', 'duckdb')).toBe('')
  })

  it('normalizes paths for duplicate detection', () => {
    expect(normalizeComparablePath('/tmp/App.duckdb/', 'linux')).toBe('/tmp/App.duckdb')
    expect(normalizeComparablePath('/tmp/App.duckdb', 'macos')).toBe('/tmp/app.duckdb')
    expect(normalizeComparablePath('C:\\Data\\App.duckdb', 'windows')).toBe('c:\\data\\app.duckdb')
  })

  it('compares database files case-insensitively on macOS and Windows', () => {
    expect(isSameDatabaseFile('/tmp/App.duckdb', '/tmp/app.duckdb', 'macos')).toBe(true)
    expect(isSameDatabaseFile('/tmp/App.duckdb', '/tmp/app.duckdb', 'linux')).toBe(false)
    expect(isSameDatabaseFile('', '/tmp/app.duckdb', 'linux')).toBe(false)
  })

  it('suggests a safe file name for the save dialog', () => {
    expect(suggestDatabaseFileName('My Analytics')).toBe('My-Analytics.duckdb')
    expect(suggestDatabaseFileName('sales/2026')).toBe('sales-2026.duckdb')
    expect(suggestDatabaseFileName()).toBe('database.duckdb')
  })
})

describe('duckdb memory instances', () => {
  it('detects both sqlite and duckdb memory sentinels', () => {
    expect(isMemoryDatabaseHost(':memory:')).toBe(true)
    expect(isMemoryDatabaseHost('memory:sqlkit_ab12')).toBe(true)
    expect(isMemoryDatabaseHost('/tmp/app.duckdb')).toBe(false)
  })

  it('round-trips a named in-memory instance', () => {
    const label = createMemoryDatabaseLabel()
    const host = buildMemoryDatabaseHost(label)
    expect(host.startsWith('memory:sqlkit_')).toBe(true)
    expect(memoryDatabaseLabel(host)).toBe(label)
    expect(memoryDatabaseLabel('/tmp/app.duckdb')).toBe('')
  })
})

describe('file probe assessment', () => {
  it('treats memory hosts as memory without probing', () => {
    expect(assessDatabaseFile(null, 'duckdb', 'memory:sqlkit_x').status).toBe('memory')
    expect(assessDatabaseFile(null, 'duckdb', ':memory:').status).toBe('memory')
  })

  it('reports a missing path before a probe arrives', () => {
    expect(assessDatabaseFile(null, 'duckdb', '').status).toBe('unknown')
  })

  it('explains that a missing file will be created by the engine', () => {
    expect(assessDatabaseFile(probe(), 'duckdb', '/tmp/app.duckdb').status).toBe('will-create')
  })

  it('flags a missing parent directory and an unwritable parent', () => {
    expect(assessDatabaseFile(probe({ parentExists: false }), 'duckdb', '/tmp/app.duckdb').status)
      .toBe('missing-parent')
    expect(assessDatabaseFile(probe({ parentWritable: false, writable: false }), 'duckdb', '/tmp/app.duckdb').status)
      .toBe('unwritable')
  })

  it('flags directories and read-only files', () => {
    expect(assessDatabaseFile(probe({ exists: true, isDirectory: true }), 'duckdb', '/tmp/app.duckdb').status)
      .toBe('directory')
    expect(assessDatabaseFile(probe({ exists: true, sizeBytes: 10, writable: false, format: 'duckdb' }), 'duckdb', '/tmp/app.duckdb').status)
      .toBe('unwritable')
  })

  it('recognises empty and ready database files', () => {
    expect(assessDatabaseFile(probe({ exists: true, sizeBytes: 0, format: 'unknown' }), 'duckdb', '/tmp/app.duckdb').status)
      .toBe('empty')
    expect(assessDatabaseFile(probe({ exists: true, sizeBytes: 8192, format: 'duckdb' }), 'duckdb', '/tmp/app.duckdb').status)
      .toBe('ready')
    expect(assessDatabaseFile(probe({ exists: true, sizeBytes: 8192, format: null }), 'duckdb', '/tmp/app.duckdb').status)
      .toBe('ready')
  })

  it('rejects DuckDB paths the JDBC driver cannot parse', () => {
    expect(assessDatabaseFile(null, 'duckdb', '/tmp/semi;colon.duckdb').status).toBe('unsupported-path')
    // SQLite passes paths straight to the native engine, so ';' is fine there.
    expect(assessDatabaseFile(probe({ exists: true, sizeBytes: 4096, format: 'sqlite' }), 'sqlite', '/tmp/semi;colon.db').status)
      .toBe('ready')
  })

  it('flags files written by another engine', () => {
    const assessment = assessDatabaseFile(
      probe({ exists: true, sizeBytes: 4096, format: 'sqlite' }),
      'duckdb',
      '/tmp/legacy.db',
    )
    expect(assessment.status).toBe('wrong-format')
    expect(assessment.format).toBe('sqlite')
  })
})

describe('recent database files', () => {
  it('keeps per-engine storage keys', () => {
    expect(recentDatabasesKey('duckdb')).toBe('duckdb_recent_databases')
    expect(recentDatabasesKey('sqlite')).toBe('sqlite_recent_databases')
  })

  it('moves a re-used path to the front and caps the list', () => {
    const entries = [
      { path: '/tmp/a.duckdb', timestamp: 2 },
      { path: '/tmp/b.duckdb', timestamp: 1 },
    ]
    expect(mergeRecentDatabase(entries, '/tmp/b.duckdb', 3)).toEqual([
      { path: '/tmp/b.duckdb', timestamp: 3 },
      { path: '/tmp/a.duckdb', timestamp: 2 },
    ])

    const capped = mergeRecentDatabase(
      Array.from({ length: 10 }, (_, index) => ({ path: `/tmp/${index}.duckdb`, timestamp: index })),
      '/tmp/new.duckdb',
      99,
    )
    expect(capped).toHaveLength(10)
    expect(capped[0].path).toBe('/tmp/new.duckdb')
  })

  it('formats file sizes', () => {
    expect(formatFileSize(0)).toBe('0 B')
    expect(formatFileSize(512)).toBe('512 B')
    expect(formatFileSize(2048)).toBe('2.0 KB')
    expect(formatFileSize(15 * 1024 * 1024)).toBe('15 MB')
    expect(formatFileSize(null)).toBe('')
  })
})

describe('duckdb failure classification', () => {
  it('recognises the file-lock failure and offers read-only', () => {
    const hint = classifyDuckDbFailure('IO Error: Could not set lock on file "/tmp/app.duckdb": Conflicting lock is held')
    expect(hint?.kind).toBe('lock-conflict')
    expect(hint?.action).toBe('use-read-only')
    expect(hint?.messageKey).toBe('components.serverForm.duckdbFailures.lock-conflict')
  })

  it('recognises the instance-configuration conflict', () => {
    const hint = classifyDuckDbFailure('Connection Error: Can\'t open a connection to same database file with a different configuration than existing connections')
    expect(hint?.kind).toBe('config-conflict')
    expect(hint?.action).toBe('use-read-only')
  })

  it('explains read-only connections rejected by an older bridge', () => {
    const hint = classifyDuckDbFailure('Failed to initialize pool: Can\'t change read-only status on connection level.')
    expect(hint?.kind).toBe('read-only-unsupported')
    expect(hint?.action).toBe('use-read-write')
  })

  it('recognises storage-format incompatibility', () => {
    expect(classifyDuckDbFailure('Serialization Error: Failed to deserialize: ...')?.kind).toBe('storage-version')
    expect(classifyDuckDbFailure('unable to open database: incompatible storage version')?.kind).toBe('storage-version')
  })

  it('recognises our own unsupported-path rejection', () => {
    const ours = 'Invalid DuckDB database path: DuckDB\'s JDBC driver splits the connection URL on \';\', so a database path containing \';\' cannot be opened. Rename the file and try again.'
    expect(classifyDuckDbFailure(ours)?.kind).toBe('unsupported-url')
    // The driver's own wording, as surfaced by the JDBC bridge.
    expect(classifyDuckDbFailure('Failed to verify connection: Invalid URL entry: colon.duckdb')?.kind)
      .toBe('unsupported-url')
  })

  it('only applies generic signatures to DuckDB messages', () => {
    expect(classifyDuckDbFailure('no such file or directory')?.kind).toBeUndefined()
    expect(classifyDuckDbFailure('DuckDB: IO Error: No such file or directory')?.kind).toBe('missing-file')
    expect(classifyDuckDbFailure('duckdb: IO Error: No such file or directory')?.kind).toBe('missing-file')
    // A PostgreSQL error must not be explained with DuckDB advice.
    expect(classifyDuckDbFailure('database "reports" does not exist')).toBeNull()
  })

  it('returns null for unrelated errors', () => {
    expect(classifyDuckDbFailure('password authentication failed for user "postgres"')).toBeNull()
    expect(classifyDuckDbFailure('')).toBeNull()
    expect(classifyDuckDbFailure(null)).toBeNull()
  })
})

describe('storage versions', () => {
  it('maps DuckDB storage versions to the oldest compatible release', () => {
    // 64 is what duckdb_jdbc 1.5.6 writes; DuckDB itself reports v1.0.0+.
    expect(storageVersionMinRelease(64)).toBe('v1.0.0')
    expect(storageVersionMinRelease(65)).toBe('v1.2.0')
    expect(storageVersionMinRelease(66)).toBe('v1.3.0')
    expect(storageVersionMinRelease(99)).toBeNull()
    expect(storageVersionMinRelease(null)).toBeNull()
  })
})
