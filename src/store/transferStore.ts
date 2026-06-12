import type {
  BackgroundTask,
  ExportRequest,
  ExportTaskConfig,
  ImportRequest,
  ImportTaskConfig,
  TaskConfig,
  TaskKind,
  TaskRuntime,
  TaskStatus,
  TransferProgress,
  TransferResult,
} from '@/types/transfer'
import { defineStore } from 'pinia'

import { computed, ref } from 'vue'

export const useTransferStore = defineStore('transfer', () => {
  const activeTab = ref<'export' | 'import'>('export')

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
        const tableName = cfg.sources?.[0]?.table || 'unknown'
        return `Export ${tableName} → ${cfg.format.toUpperCase()}`
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
  }
})
