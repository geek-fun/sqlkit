import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { onUnmounted, reactive, ref } from 'vue'

export type DownloadKind = 'jre' | 'bridge' | 'driver'

export type DownloadProgress = {
  downloaded: number
  total: number
}

export type DownloadState = 'idle' | 'preparing' | 'downloading' | 'installing' | 'complete' | 'error'

type DownloadEvent
  = | { phase: 'progress', id: string, kind: DownloadKind, downloaded: number, total: number }
    | { phase: 'complete', id: string, kind: DownloadKind }
    | { phase: 'error', id: string, kind: DownloadKind, error: string }

const EVENT_NAME = 'download-progress'

const progress = reactive<Record<string, DownloadProgress>>({})
const states = reactive<Record<string, DownloadState>>({})
const errors = reactive<Record<string, string>>({})
const kinds = reactive<Record<string, DownloadKind>>({})

let listenerCount = 0
let generation = 0
let listenPromise: Promise<void> | null = null
let unlisten: UnlistenFn | null = null
const pending = reactive<Record<string, boolean>>({})

const receiveEvent = (event: { payload: DownloadEvent }) => {
  const { phase, id, kind } = event.payload
  kinds[id] = kind
  if (phase === 'progress') {
    progress[id] = { downloaded: event.payload.downloaded, total: event.payload.total }
    states[id] = 'downloading'
    delete errors[id]
  }
  else if (phase === 'complete') {
    delete progress[id]
    states[id] = pending[id] ? 'installing' : 'complete'
  }
  else {
    delete progress[id]
    states[id] = 'error'
    errors[id] = event.payload.error
  }
}

const registerListener = () => {
  if (!listenPromise) {
    const token = ++generation
    listenPromise = listen<DownloadEvent>(EVENT_NAME, receiveEvent).then((handle) => {
      if (token !== generation || (listenerCount === 0 && !Object.values(pending).some(Boolean)))
        handle()
      else unlisten = handle
    }).catch((error) => {
      if (token === generation)
        listenPromise = null
      throw error
    })
  }
  return listenPromise
}

const ensureListener = () => {
  listenerCount++
  return registerListener()
}

const releaseListener = () => {
  listenerCount = Math.max(0, listenerCount - 1)
  if (listenerCount === 0 && !Object.values(pending).some(Boolean)) {
    generation++
    unlisten?.()
    unlisten = null
    listenPromise = null
  }
}

export const useDownloadEvents = () => {
  const ready = ref(false)

  const registration = ensureListener()
  registration.then(() => {
    ready.value = true
  }).catch(() => {
    ready.value = false
  })

  onUnmounted(() => {
    releaseListener()
  })

  const getProgress = (id: string): DownloadProgress | null => progress[id] ?? null

  const getState = (id: string): DownloadState => states[id] ?? 'idle'

  const getError = (id: string): string | null => errors[id] ?? null

  const getKind = (id: string): DownloadKind | null => kinds[id] ?? null

  const isDownloading = (id: string): boolean => ['preparing', 'downloading', 'installing'].includes(getState(id))

  const startDownload = async (kind: DownloadKind, id: string, invokeFn: () => Promise<void>): Promise<boolean> => {
    if (isDownloading(id))
      return false
    kinds[id] = kind
    pending[id] = true
    states[id] = 'preparing'
    delete progress[id]
    delete errors[id]
    try {
      await (listenPromise ?? registerListener())
      ready.value = true
      await invokeFn()
      if (getState(id) === 'error')
        throw new Error(errors[id])
      states[id] = 'complete'
      delete progress[id]
      return true
    }
    catch (error) {
      errors[id] = error instanceof Error ? error.message : String(error)
      states[id] = 'error'
      delete progress[id]
      return false
    }
    finally {
      delete pending[id]
      if (listenerCount === 0)
        releaseListener()
    }
  }

  const reset = (id: string): void => {
    delete progress[id]
    delete states[id]
    delete errors[id]
    delete kinds[id]
  }

  return {
    ready,
    progress,
    states,
    errors,
    kinds,
    getProgress,
    getState,
    getError,
    getKind,
    isDownloading,
    startDownload,
    reset,
  }
}
