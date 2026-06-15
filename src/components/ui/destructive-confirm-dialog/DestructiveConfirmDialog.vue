<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { buttonVariants } from '@/components/ui/button'
import { Spinner } from '@/components/ui/spinner'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  open: boolean
  title: string
  message: string
  detail?: string
  confirmLabel?: string
  confirmVariant?: 'destructive' | 'warning'
  loading?: boolean
  class?: HTMLAttributes['class']
}>(), {
  detail: undefined,
  confirmLabel: 'Confirm',
  confirmVariant: 'destructive',
  loading: false,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
  'confirm': []
  'cancel': []
}>()

function handleOpenChange(open: boolean) {
  if (!open)
    emit('cancel')
  emit('update:open', open)
}

function handleConfirm() {
  emit('confirm')
}

const confirmButtonClass = props.confirmVariant === 'warning'
  ? buttonVariants({ variant: 'default' })
  : buttonVariants({ variant: 'destructive' })
</script>

<template>
  <AlertDialog :open="props.open" @update:open="handleOpenChange">
    <AlertDialogContent :class="cn('sm:max-w-md', props.class)">
      <AlertDialogHeader>
        <div class="flex gap-3 items-start">
          <!-- Warning icon -->
          <div
            class="rounded-full flex shrink-0 h-10 w-10 items-center justify-center"
            :class="props.confirmVariant === 'warning'
              ? 'bg-amber-100 text-amber-600 dark:bg-amber-900/30 dark:text-amber-400'
              : 'bg-destructive/10 text-destructive'"
          >
            <span class="i-carbon-warning h-5 w-5" />
          </div>
          <div class="flex-1 space-y-1">
            <AlertDialogTitle>{{ props.title }}</AlertDialogTitle>
            <AlertDialogDescription>
              {{ props.message }}
            </AlertDialogDescription>
            <p
              v-if="props.detail"
              class="text-xs text-muted-foreground/80 pt-1"
            >
              {{ props.detail }}
            </p>
          </div>
        </div>
      </AlertDialogHeader>

      <!-- Slot for extra content (e.g. PK summary, table name confirmation) -->
      <div v-if="$slots.default" class="px-6">
        <slot />
      </div>

      <AlertDialogFooter class="gap-2 sm:justify-end">
        <AlertDialogCancel :disabled="props.loading">
          Cancel
        </AlertDialogCancel>
        <AlertDialogAction
          :class="confirmButtonClass"
          :disabled="props.loading"
          @click.prevent="handleConfirm"
        >
          <Spinner v-if="props.loading" size="sm" class="mr-1.5" />
          {{ props.confirmLabel }}
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
