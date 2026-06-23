<script setup lang="ts">
import { useI18n } from 'vue-i18n'
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

type Props = {
  open: boolean
  queryName: string | null
  targetConnection: string | null
  createdAt: string | null
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'confirm'): void
}>()

const { t } = useI18n()
</script>

<template>
  <AlertDialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle class="flex gap-2 items-center">
          <span class="i-carbon-trash-can text-destructive h-4 w-4" />
          {{ t('sidebar.savedQueries.delete.title') }}
        </AlertDialogTitle>
        <AlertDialogDescription>
          {{ t('sidebar.savedQueries.delete.description', { name: queryName }) }}
          <div v-if="targetConnection || createdAt" class="text-xs text-muted-foreground mt-3 space-y-1">
            <div v-if="targetConnection" class="flex gap-1.5 items-center">
              <span class="i-carbon-data-base h-3 w-3" />
              <span>{{ t('sidebar.savedQueries.delete.target') }}: {{ targetConnection }}</span>
            </div>
            <div v-if="createdAt" class="flex gap-1.5 items-center">
              <span class="i-carbon-calendar h-3 w-3" />
              <span>{{ t('sidebar.savedQueries.delete.created') }}: {{ createdAt }}</span>
            </div>
          </div>
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>{{ t('sidebar.savedQueries.delete.cancel') }}</AlertDialogCancel>
        <AlertDialogAction class="text-destructive-foreground bg-destructive hover:bg-destructive/90" @click="emit('confirm')">
          {{ t('sidebar.savedQueries.delete.confirm') }}
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
