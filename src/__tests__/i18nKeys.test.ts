import * as fs from 'node:fs'
import * as path from 'node:path'
import { enUS } from '@/lang/enUS'
import { zhCN } from '@/lang/zhCN'

/**
 * Guards against the class of bug where a literal `t('some.key')` does not
 * resolve and the raw key ends up rendered in the UI (e.g. a menu item showing
 * `pages.connections.newMenu.duckdbFile`).
 *
 * Only literal single-quoted keys are inspected; computed/template keys are
 * intentionally skipped because they cannot be resolved statically.
 */

const SRC_ROOT = path.resolve(__dirname, '..')
const SKIPPED_DIRECTORIES = new Set(['lang', 'node_modules'])
const KEY_PATTERN = /\$?\bt\(\s*'([\w.-]+)'/g

function collectSourceFiles(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory())
      return SKIPPED_DIRECTORIES.has(entry.name) ? [] : collectSourceFiles(fullPath)
    // This guard file mentions key patterns in its own documentation.
    if (entry.name === 'i18nKeys.test.ts')
      return []
    return /\.(?:vue|ts)$/.test(entry.name) ? [fullPath] : []
  })
}

/** Key patterns inside comments are documentation, not usage. */
function stripComments(source: string): string {
  return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(?<!:)\/\/.*$/gm, '')
}

function lookup(messages: Record<string, unknown>, key: string): unknown {
  return key
    .split('.')
    .reduce<unknown>(
      (acc, part) => (acc && typeof acc === 'object' ? (acc as Record<string, unknown>)[part] : undefined),
      messages,
    )
}

function collectLiteralKeys(files: string[]): Map<string, string> {
  const keys = new Map<string, string>()
  for (const file of files) {
    const content = stripComments(fs.readFileSync(file, 'utf8'))
    for (const match of content.matchAll(KEY_PATTERN)) {
      const key = match[1]
      if (key.includes('.') && !keys.has(key))
        keys.set(key, path.relative(SRC_ROOT, file))
    }
  }
  return keys
}

describe('i18n message keys', () => {
  const files = collectSourceFiles(SRC_ROOT)
  const keys = collectLiteralKeys(files)

  it('finds the translation keys used in source files', () => {
    expect(files.length).toBeGreaterThan(100)
    expect(keys.size).toBeGreaterThan(100)
  })

  it('resolves every literal key in enUS', () => {
    const missing = [...keys.entries()]
      .filter(([key]) => lookup(enUS as unknown as Record<string, unknown>, key) === undefined)
      .map(([key, file]) => `${key} (${file})`)

    expect(missing).toEqual([])
  })

  it('resolves every literal key in zhCN', () => {
    const missing = [...keys.entries()]
      .filter(([key]) => lookup(zhCN as unknown as Record<string, unknown>, key) === undefined)
      .map(([key, file]) => `${key} (${file})`)

    expect(missing).toEqual([])
  })
})
