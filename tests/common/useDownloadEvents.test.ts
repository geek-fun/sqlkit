/** @jest-environment jsdom */
import { listen } from '@tauri-apps/api/event'
import { flushPromises } from '@vue/test-utils'
import { createApp } from 'vue'
import { useDownloadEvents } from '@/composables/useDownloadEvents'

jest.mock('@tauri-apps/api/event', () => ({ listen: jest.fn() }))
const listenMock = listen as jest.Mock
let receive: (event: { payload: unknown }) => void
const mountDownloads = () => {
  let downloads!: ReturnType<typeof useDownloadEvents>
  const app = createApp({ setup: () => {
    downloads = useDownloadEvents()
    return () => null
  } })
  app.mount(document.createElement('div'))
  return { downloads, unmount: () => app.unmount() }
}

beforeEach(() => {
  listenMock.mockReset()
  listenMock.mockImplementation((_name, handler) => {
    receive = handler
    return Promise.resolve(jest.fn())
  })
})

it('registers before invoking and stays busy until installation resolves', async () => {
  let registered!: (handle: () => void) => void
  listenMock.mockImplementationOnce((_name, handler) => {
    receive = handler
    return new Promise((resolve) => {
      registered = resolve
    })
  })
  const { downloads, unmount } = mountDownloads()
  let finish!: () => void
  const invoke = jest.fn(() => new Promise<void>((resolve) => {
    finish = resolve
  }))
  const done = downloads.startDownload('jre', 'test-runtime', invoke)
  expect(downloads.getState('test-runtime')).toBe('preparing')
  expect(downloads.getProgress('test-runtime')).toBeNull()
  expect(invoke).not.toHaveBeenCalled()
  registered(jest.fn())
  await flushPromises()
  expect(invoke).toHaveBeenCalledTimes(1)
  receive({ payload: { phase: 'progress', id: 'test-runtime', kind: 'jre', downloaded: 25, total: 100 } })
  expect(downloads.getProgress('test-runtime')).toEqual({ downloaded: 25, total: 100 })
  receive({ payload: { phase: 'complete', id: 'test-runtime', kind: 'jre' } })
  expect(downloads.getState('test-runtime')).toBe('installing')
  expect(downloads.isDownloading('test-runtime')).toBe(true)
  finish()
  expect(await done).toBe(true)
  expect(downloads.getState('test-runtime')).toBe('complete')
  unmount()
})

it('registers again after leaving and returning to settings', async () => {
  const first = mountDownloads()
  await flushPromises()
  first.unmount()
  const second = mountDownloads()
  await flushPromises()
  expect(listenMock).toHaveBeenCalledTimes(2)
  receive({ payload: { phase: 'progress', id: 'remount-driver', kind: 'driver', downloaded: 5, total: 0 } })
  expect(second.downloads.getProgress('remount-driver')).toEqual({ downloaded: 5, total: 0 })
  second.unmount()
})

it('cached downloads complete without progress events and failures can retry', async () => {
  const { downloads, unmount } = mountDownloads()
  expect(await downloads.startDownload('bridge', 'cached-bridge', async () => {})).toBe(true)
  expect(downloads.isDownloading('cached-bridge')).toBe(false)
  expect(await downloads.startDownload('driver', 'retry-driver', async () => {
    throw new Error('Offline')
  })).toBe(false)
  expect(downloads.getError('retry-driver')).toBe('Offline')
  expect(await downloads.startDownload('driver', 'retry-driver', async () => {})).toBe(true)
  expect(downloads.getError('retry-driver')).toBeNull()
  unmount()
})

it('does not lose a backend error when the command resolves', async () => {
  const { downloads, unmount } = mountDownloads()
  expect(await downloads.startDownload('driver', 'backend-error', async () => {
    receive({ payload: { phase: 'error', id: 'backend-error', kind: 'driver', error: 'Checksum failed' } })
  })).toBe(false)
  expect(downloads.getState('backend-error')).toBe('error')
  expect(downloads.getError('backend-error')).toBe('Checksum failed')
  unmount()
})
