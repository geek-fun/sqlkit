import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'

const browser = window as Window & {
  invokeSqlKitForTest?: (command: string, args: unknown) => Promise<unknown>
}
const invoke = browser.invokeSqlKitForTest
if (!invoke)
  throw new Error('Playwright did not provide the Tauri command handler')

const platform = navigator.platform.startsWith('Mac')
  ? 'macos'
  : navigator.platform.startsWith('Win')
    ? 'windows'
    : 'linux'
Object.assign(window, {
  __TAURI_OS_PLUGIN_INTERNALS__: { platform },
})

mockWindows('main')
mockIPC(invoke, { shouldMockEvents: true })
