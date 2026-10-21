/** @jest-environment jsdom */
import type { Update } from '@tauri-apps/plugin-updater'
import { check } from '@tauri-apps/plugin-updater'
import { useAppUpdater } from '@/composables/useAppUpdater'

jest.mock('@tauri-apps/plugin-updater', () => ({ check: jest.fn() }))
jest.mock('@tauri-apps/plugin-process', () => ({ relaunch: jest.fn() }))
jest.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
jest.mock('@vueuse/core', () => ({ useLocalStorage: () => jest.requireActual('vue').ref('') }))
jest.mock('@/composables/useNotifications', () => ({ toast: { info: jest.fn(), success: jest.fn(), error: jest.fn() } }))

const updater = useAppUpdater()
const release = { version: '2.0.0', downloadAndInstall: jest.fn() } as unknown as Update

beforeEach(() => {
  jest.clearAllMocks()
  updater.dismissUpdate()
})

it('clears stale release information when a new check finds no update', async () => {
  jest.mocked(check).mockResolvedValueOnce(release).mockResolvedValueOnce(null)
  await updater.checkForUpdates()
  expect(updater.updateAvailable.value).toBe(true)
  await updater.checkForUpdates()
  expect(updater.isUpToDate.value).toBe(true)
  expect(updater.updateAvailable.value).toBe(false)
  expect(updater.updateInfo.value).toBeNull()
})

it('records a silent check failure for inline feedback and allows retry', async () => {
  const errorLog = jest.spyOn(console, 'error').mockImplementation(() => {})
  jest.mocked(check).mockRejectedValueOnce(new Error('Offline')).mockResolvedValueOnce(null)
  await updater.checkForUpdates(false)
  expect(updater.isUpToDate.value).toBe(false)
  expect(updater.updateError.value).toBe('Offline')
  expect(updater.isChecking.value).toBe(false)
  await updater.checkForUpdates(false)
  expect(updater.isUpToDate.value).toBe(true)
  expect(updater.updateError.value).toBe('')
  errorLog.mockRestore()
})

it('keeps skipped releases discoverable in Settings, including an in-flight startup check', async () => {
  jest.mocked(check).mockResolvedValueOnce(release)
  await updater.checkForUpdates()
  updater.skipUpdate()
  jest.mocked(check).mockResolvedValueOnce(release)
  await updater.checkForUpdates()
  expect(updater.updateAvailable.value).toBe(false)

  let finish!: (update: Update) => void
  jest.mocked(check).mockImplementationOnce(() => new Promise(resolve => finish = resolve))
  const startupCheck = updater.checkForUpdates(false)
  await updater.checkForUpdates(false, { includeSkipped: true })
  finish(release)
  await startupCheck
  expect(check).toHaveBeenCalledTimes(3)
  expect(updater.updateAvailable.value).toBe(true)
  expect(updater.updateInfo.value?.version).toBe('2.0.0')
  expect(release.downloadAndInstall).not.toHaveBeenCalled()
})
