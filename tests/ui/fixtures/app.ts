import { fileURLToPath } from 'node:url'
import { test as base, expect } from '@playwright/test'
import { build } from 'vite'

type CommandHandler = (args: Record<string, unknown>) => unknown
type TauriMock = { handle: (command: string, handler: CommandHandler) => void }

// Bundle Tauri's mocks for injection before the production app entry runs.
const bundleTauriScript = async (): Promise<string> => {
  const bundles = await build({
    configFile: false,
    logLevel: 'error',
    build: {
      write: false,
      lib: {
        entry: fileURLToPath(new URL('./tauri-browser.ts', import.meta.url)),
        name: 'SqlKitTauriMock',
        formats: ['iife'],
      },
    },
  })
  const outputs = Array.isArray(bundles)
    ? bundles
    : [bundles]
  const entry = outputs
    .flatMap(bundle => 'output' in bundle
      ? bundle.output
      : [])
    .find(chunk => chunk.type === 'chunk' && chunk.isEntry)
  if (entry?.type !== 'chunk')
    throw new Error('Failed to bundle the Tauri test bootstrap')
  return entry.code
}

let tauriScript: Promise<string> | undefined

export const test = base.extend<{ tauri: TauriMock }>({
  tauri: async ({ context }, use) => {
    const storage = new Map<string, unknown>()
    const defaults: Record<string, CommandHandler> = {
      'store_get': ({ key }) => storage.get(String(key)) ?? null,
      'store_set': ({ key, value }) => { storage.set(String(key), value) },
      'set_ui_locale': () => null,
      'consume_pending_auth': () => null,
      'clear_entitlement': () => null,
      'plugin:updater|check': () => null,
      'plugin:window|is_maximized': () => false,
      'plugin:window|is_fullscreen': () => false,
    }
    const handlers = new Map(Object.entries(defaults))
    const unexpected = new Set<string>()
    await context.exposeFunction('invokeSqlKitForTest', (command: string, args: Record<string, unknown> = {}) => {
      const handler = handlers.get(command)
      if (handler)
        return handler(args)
      unexpected.add(command)
      throw new Error(`Unexpected Tauri command: ${command}`)
    })
    tauriScript ??= bundleTauriScript()
    await context.addInitScript({ content: await tauriScript })
    await use({
      handle: (command, handler) => { handlers.set(command, handler) },
    })
    expect([...unexpected], 'Unexpected Tauri commands').toEqual([])
  },
})

export { expect }
