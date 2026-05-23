import type { UnlistenFn } from '@tauri-apps/api/event'
import type {
  BackgroundTask,
  ExportFormat,
  ExportRequest,
  ExportTaskConfig,
  ImportRequest,
  ImportTaskConfig,
  JobProgress,
  ObjectSelection,
  TaskConfig,
  TaskKind,
  TaskRuntime,
  TaskStatus,
  TransferJob,
  TransferProfile,
  TransferProgress,
  TransferResult,
} from '@/types/transfer'
import { listen } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

import {
  backupServer,
  listTransferProfiles,
  migrateServer,
  runTransferProfile,
  saveTransferProfile,
} from '@/datasources/transferApi'

export const useTransferStore = defineStore('transfer', () => {
  const activeTab = ref<'export' | 'import' | 'migration' | 'structure'>('export')

  const isRunning = ref(false)
  const progress = ref<TransferProgress | null>(null)
  const lastResult = ref<TransferResult | null>(null)

  const exportStep = ref(0)
  const exportRequest = ref<Partial<ExportRequest>>({})

  const importStep = ref(0)
  const importRequest = ref<Partial<ImportRequest>>({})

  const runningTasks = ref<BackgroundTask[]>([])
  const activeExportTaskId = ref<string | null>(null)
  const activeImportTaskId = ref<string | null>(null)

  const jobs = ref<TransferJob[]>([])
  const savedProfiles = ref<TransferProfile[]>([])
  const unlistenMap = new Map<string, UnlistenFn>()

  const progressPercent = computed(() => progress.value?.percent ?? 0)

  const taskCount = computed(() => runningTasks.value.length)

  const hasRunningTasks = computed(() =>
    runningTasks.value.some(t => t.status === 'running'),
  )

  const activeTaskId = computed(() => {
    switch (activeTab.value) {
      case 'export': return activeExportTaskId.value
      case 'import': return activeImportTaskId.value
      default: return null
    }
  })

  const setActiveTab = (tab: typeof activeTab.value) => {
    activeTab.value = tab
  }

  const updateProgress = (p: TransferProgress) => {
    progress.value = p
  }

  const startOperation = () => {
    isRunning.value = true
    progress.value = null
    lastResult.value = null
  }

  const completeOperation = (result: TransferResult) => {
    isRunning.value = false
    lastResult.value = result
    progress.value = null
  }

  const resetExport = () => {
    exportStep.value = 0
    exportRequest.value = {}
    lastResult.value = null
  }

  const resetImport = () => {
    importStep.value = 0
    importRequest.value = {}
    lastResult.value = null
  }

  const addRunningTask = (task: BackgroundTask) => {
    runningTasks.value = [...runningTasks.value, task]
  }

  const updateTaskRuntime = (taskId: string, runtime: Partial<TaskRuntime>) => {
    runningTasks.value = runningTasks.value.map(t =>
      t.id === taskId
        ? {
            ...t,
            runtime: { ...t.runtime, ...runtime },
            progress: {
              complete: runtime.complete ?? t.progress.complete,
              total: runtime.total ?? t.progress.total,
            },
          }
        : t,
    )
  }

  const updateTaskStatus = (taskId: string, status: TaskStatus, error?: string) => {
    runningTasks.value = runningTasks.value.map(t =>
      t.id === taskId
        ? {
            ...t,
            status,
            endTime: status === 'completed' || status === 'failed' ? new Date() : undefined,
            error,
          }
        : t,
    )
  }

  const removeTask = (taskId: string) => {
    runningTasks.value = runningTasks.value.filter(t => t.id !== taskId)
  }

  const clearCompletedTasks = () => {
    runningTasks.value = runningTasks.value.filter(t =>
      t.status === 'running' || t.status === 'pending',
    )
  }

  const syncProgressToTask = (taskId: string, p: TransferProgress) => {
    updateTaskRuntime(taskId, {
      complete: p.processedRows,
      total: p.totalRows ?? 0,
      skipped: p.skippedRows,
      errorCount: p.errorCount,
    })
  }

  const detachActiveTask = (kind: TaskKind) => {
    switch (kind) {
      case 'export':
        activeExportTaskId.value = null
        break
      case 'import':
        activeImportTaskId.value = null
        break
    }
  }

  const openTask = (taskId: string) => {
    const task = runningTasks.value.find(t => t.id === taskId)
    if (!task)
      return

    const tabMap: Record<TaskKind, typeof activeTab.value> = {
      export: 'export',
      import: 'import',
      sqlFile: 'structure',
      migration: 'migration',
    }
    activeTab.value = tabMap[task.kind]

    switch (task.kind) {
      case 'export':
        activeExportTaskId.value = taskId
        break
      case 'import':
        activeImportTaskId.value = taskId
        break
    }
  }

  const generateTaskLabel = (kind: TaskKind, config: TaskConfig): string => {
    switch (kind) {
      case 'export': {
        const cfg = config as ExportTaskConfig
        return `Export ${cfg.table} → ${cfg.format.toUpperCase()}`
      }
      case 'import': {
        const cfg = config as ImportTaskConfig
        const fileName = cfg.filePath.split('/').pop() || cfg.filePath
        return `Import ${fileName}`
      }
      default:
        return 'Transfer task'
    }
  }

  const createTask = (kind: TaskKind, config: TaskConfig, total: number): BackgroundTask => {
    const id = crypto.randomUUID()
    const label = generateTaskLabel(kind, config)
    return {
      id,
      kind,
      status: 'running',
      progress: { complete: 0, total },
      config,
      runtime: { complete: 0, total, skipped: 0, errorCount: 0 },
      label,
      startTime: new Date(),
    }
  }

  const subscribeToJob = async (jobId: string) => {
    const unlisten = await listen<{ status: string, progress: JobProgress, error?: string }>(
      `transfer://progress/${jobId}`,
      (event) => {
        const payload = event.payload
        jobs.value = jobs.value.map(job =>
          job.id === jobId
            ? {
                ...job,
                status: payload.status as any,
                progress: payload.progress,
                error: payload.error,
                finishedAt: ['completed', 'failed', 'cancelled'].includes(payload.status) ? Date.now() : job.finishedAt,
              }
            : job,
        )

        if (['completed', 'failed', 'cancelled'].includes(payload.status)) {
          const fn = unlistenMap.get(jobId)
          if (fn) {
            fn()
            unlistenMap.delete(jobId)
          }
        }
      },
    )
    unlistenMap.set(jobId, unlisten)
    return unlisten
  }

  const startBackupServer = async (args: {
    connectionId: string
    name: string
    selection: ObjectSelection
    format: ExportFormat
    destination: string
    options: Record<string, unknown>
  }) => {
    const requestedJobId = crypto.randomUUID()
    await subscribeToJob(requestedJobId)

    const newJob: TransferJob = {
      id: requestedJobId,
      name: args.name,
      kind: 'backup',
      scope: 'server',
      connectionId: args.connectionId,
      status: 'queued',
      progress: { stage: 'Initializing...', current: 0, total: 1 },
      startedAt: Date.now(),
    }
    jobs.value = [...jobs.value, newJob]

    const jobId = await backupServer(
      args.connectionId,
      args.selection,
      args.format,
      args.destination,
      args.options,
      requestedJobId,
    )

    return jobId
  }

  const startMigrateServer = async (args: {
    sourceConnectionId: string
    targetConnectionId: string
    name: string
    selection: ObjectSelection
    options: Record<string, unknown>
  }) => {
    const requestedJobId = crypto.randomUUID()
    await subscribeToJob(requestedJobId)

    const newJob: TransferJob = {
      id: requestedJobId,
      name: args.name,
      kind: 'migrate',
      scope: 'server',
      connectionId: args.sourceConnectionId,
      status: 'queued',
      progress: { stage: 'Initializing...', current: 0, total: 1 },
      startedAt: Date.now(),
    }
    jobs.value = [...jobs.value, newJob]

    const jobId = await migrateServer(
      args.sourceConnectionId,
      args.targetConnectionId,
      args.selection,
      args.options,
      requestedJobId,
    )

    return jobId
  }

  const saveProfile = async (profile: TransferProfile) => {
    const profileId = await saveTransferProfile(profile)
    const newProfile = { ...profile, id: profileId }
    savedProfiles.value = [...savedProfiles.value, newProfile]
    return profileId
  }

  const loadProfiles = async () => {
    const profiles = await listTransferProfiles()
    savedProfiles.value = profiles
  }

  const runProfile = async (profileId: string) => {
    const profile = savedProfiles.value.find(p => p.id === profileId)
    if (!profile)
      throw new Error('Profile not found')

    const jobId = await runTransferProfile(profileId)
    const newJob: TransferJob = {
      id: jobId,
      name: `Run ${profile.name}`,
      kind: profile.kind,
      scope: profile.scope,
      connectionId: profile.connectionId,
      status: 'queued',
      progress: { stage: 'Initializing...', current: 0, total: 1 },
      startedAt: Date.now(),
    }
    jobs.value = [...jobs.value, newJob]
    await subscribeToJob(jobId)
    return jobId
  }

  const cancelJob = (jobId: string) => {
    jobs.value = jobs.value.map(job =>
      job.id === jobId
        ? { ...job, status: 'cancelled', finishedAt: Date.now() }
        : job,
    )

    const fn = unlistenMap.get(jobId)
    if (fn) {
      fn()
      unlistenMap.delete(jobId)
    }
  }

  const dismissJob = (jobId: string) => {
    jobs.value = jobs.value.filter(job => job.id !== jobId)
    const fn = unlistenMap.get(jobId)
    if (fn) {
      fn()
      unlistenMap.delete(jobId)
    }
  }

  return {
    activeTab,
    setActiveTab,
    isRunning,
    progress,
    lastResult,
    progressPercent,
    updateProgress,
    startOperation,
    completeOperation,
    exportStep,
    exportRequest,
    importStep,
    importRequest,
    resetExport,
    resetImport,
    runningTasks,
    activeExportTaskId,
    activeImportTaskId,
    taskCount,
    hasRunningTasks,
    activeTaskId,
    addRunningTask,
    updateTaskRuntime,
    updateTaskStatus,
    removeTask,
    clearCompletedTasks,
    syncProgressToTask,
    detachActiveTask,
    openTask,
    createTask,
    jobs,
    savedProfiles,
    subscribeToJob,
    startBackupServer,
    startMigrateServer,
    saveProfile,
    loadProfiles,
    runProfile,
    cancelJob,
    dismissJob,
  }
})
