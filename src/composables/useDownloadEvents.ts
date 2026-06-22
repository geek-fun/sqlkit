import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { onUnmounted, reactive, ref } from 'vue'

export type DownloadKind = 'jre' | 'bridge' | 'driver'

export type DownloadProgress = {
  downloaded: number
  total: number
}

export type DownloadState = 'idle' | 'downloading' | 'complete' | 'error'

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
let listenPromise: Promise<void> | null = null
let unlisten: UnlistenFn | null = null

function ensureListener() {
  listenerCount++
  if (listenPromise)
    return listenPromise
  listenPromise = (async () => {
    unlisten = await listen<DownloadEvent>(EVENT_NAME, (event) => {
      const { phase, id, kind } = event.payload
      if (kind)
        kinds[id] = kind
      if (phase === 'progress') {
        progress[id] = { downloaded: event.payload.downloaded, total: event.payload.total }
        states[id] = 'downloading'
        delete errors[id]
      }
      else if (phase === 'complete') {
        delete progress[id]
        states[id] = 'complete'
      }
      else if (phase === 'error') {
        delete progress[id]
        states[id] = 'error'
        errors[id] = event.payload.error
      }
    })
    // If all components unmounted while we were registering, clean up immediately
    if (listenerCount === 0 && unlisten) {
      unlisten()
      unlisten = null
    }
  })()
  return listenPromise
}

function releaseListener() {
  if (listenerCount === 0)
    return
  listenerCount--
  if (listenerCount === 0 && unlisten) {
    unlisten()
    unlisten = null
  }
}

export function useDownloadEvents() {
  const ready = ref(false)

  ensureListener().then(() => {
    ready.value = true
  })

  onUnmounted(() => {
    releaseListener()
  })

  const getProgress = (id: string): DownloadProgress | null => progress[id] ?? null

  const getState = (id: string): DownloadState => states[id] ?? 'idle'

  const getError = (id: string): string | null => errors[id] ?? null

  const getKind = (id: string): DownloadKind | null => kinds[id] ?? null

  const isDownloading = (id: string): boolean => getState(id) === 'downloading'

  const startDownload = async (kind: DownloadKind, id: string, invokeFn: () => Promise<void>): Promise<boolean> => {
    kinds[id] = kind
    states[id] = 'downloading'
    progress[id] = { downloaded: 0, total: 1 }
    delete errors[id]
    try {
      await invokeFn()
      return true
    }
    catch (error) {
      errors[id] = error instanceof Error ? error.message : String(error)
      states[id] = 'error'
      delete progress[id]
      return false
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
