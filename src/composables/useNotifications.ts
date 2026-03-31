import { ref } from 'vue'

type ToastType = 'success' | 'error' | 'info'

interface Toast {
  id: number
  type: ToastType
  title: string
  description?: string
}

const toasts = ref<Toast[]>([])
let nextId = 0

const DURATION_MS = 4000

function add(type: ToastType, title: string, description?: string) {
  const id = ++nextId
  toasts.value = [...toasts.value, { id, type, title, description }]
  setTimeout(() => dismiss(id), DURATION_MS)
}

function dismiss(id: number) {
  toasts.value = toasts.value.filter(t => t.id !== id)
}

export const toast = {
  success: (title: string, opts?: { description?: string }) => add('success', title, opts?.description),
  error: (title: string, opts?: { description?: string }) => add('error', title, opts?.description),
  info: (title: string, opts?: { description?: string }) => add('info', title, opts?.description),
}

export const useNotifications = () => ({ toasts, dismiss })
