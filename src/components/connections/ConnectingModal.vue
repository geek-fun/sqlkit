<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Spinner } from '@/components/ui/spinner'

const { t } = useI18n()

const showModal = ref(false)
const connectionName = ref('')
const errorMessage = ref('')
const cancelCallback = ref<(() => void) | null>(null)
const retryCallback = ref<(() => void) | null>(null)

function show(name: string, onCancel: () => void, onRetry: () => void) {
  connectionName.value = name
  cancelCallback.value = onCancel
  retryCallback.value = onRetry
  errorMessage.value = ''
  showModal.value = true
}

function hide() {
  showModal.value = false
  connectionName.value = ''
  errorMessage.value = ''
  cancelCallback.value = null
  retryCallback.value = null
}

function showError(error: string) {
  errorMessage.value = error
}

function handleDialogOpenChange(open: boolean) {
  if (!open) {
    handleCancel()
  }
}

function handleCancel() {
  if (cancelCallback.value && !errorMessage.value) {
    cancelCallback.value()
  }
  errorMessage.value = ''
  hide()
}

function handleRetry() {
  errorMessage.value = ''
  if (retryCallback.value) {
    retryCallback.value()
  }
}

defineExpose({
  show,
  hide,
  showError,
})
</script>

<template>
  <Dialog :open="showModal" @update:open="handleDialogOpenChange">
    <DialogContent
      class="sm:max-w-[500px]"
      @interact-outside="(e) => e.preventDefault()"
      @escape-key-down="(e) => e.preventDefault()"
    >
      <DialogTitle class="sr-only">
        {{ t('components.connectingModal.connecting', { name: connectionName }) }}
      </DialogTitle>

      <div class="py-4 flex flex-col gap-4 items-center">
        <div
          v-if="errorMessage"
          class="text-sm text-destructive p-4 rounded-md bg-destructive/10 w-full"
        >
          <div class="flex gap-3 items-start">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="mt-0.5 flex-shrink-0 h-5 w-5"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="12" x2="12" y1="8" y2="12" />
              <line x1="12" x2="12.01" y1="16" y2="16" />
            </svg>
            <div class="flex-1">
              <p class="font-medium">
                {{ t('components.connectingModal.connectionError') }}
              </p>
              <p class="mt-1 opacity-90">
                {{ errorMessage }}
              </p>
            </div>
            <button
              class="opacity-70 transition-opacity hover:opacity-100"
              @click="errorMessage = ''"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-4 w-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M18 6 6 18" />
                <path d="m6 6 12 12" />
              </svg>
            </button>
          </div>
        </div>

        <div v-if="!errorMessage" class="py-4">
          <Spinner size="lg" />
        </div>

        <p class="text-lg font-medium text-center">
          {{ t('components.connectingModal.connecting', { name: connectionName }) }}
        </p>

        <p v-if="!errorMessage" class="text-sm text-muted-foreground text-center">
          {{ t('components.connectingModal.connectingSubtext') }}
        </p>
      </div>

      <div class="pt-4 border-t flex gap-3 justify-end">
        <Button v-if="errorMessage" @click="handleRetry">
          {{ t('common.buttons.retry') }}
        </Button>
        <Button :variant="errorMessage ? 'outline' : 'secondary'" @click="handleCancel">
          {{ t('common.buttons.cancel') }}
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>
