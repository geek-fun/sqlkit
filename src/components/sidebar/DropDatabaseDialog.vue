<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'

type Props = {
  open: boolean
  databaseName: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'confirm'): void
}>()

const { t } = useI18n()
const typedName = ref('')

watch(() => props.open, (open) => {
  if (open)
    typedName.value = ''
})

const isMatch = () => typedName.value.trim() === (props.databaseName || '')
</script>

<template>
  <Dialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <DialogContent class="sm:max-w-[400px]">
      <DialogHeader>
        <DialogTitle>{{ t('sidebar.databases.actions.dropDatabase.title') }}</DialogTitle>
        <DialogDescription>
          {{ t('sidebar.databases.actions.dropDatabase.message', { name: databaseName }) }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-2">
        <p class="text-xs text-destructive font-medium">
          {{ t('sidebar.databases.actions.dropDatabase.typeToConfirm') }}
        </p>
        <Input
          v-model="typedName"
          :placeholder="databaseName || ''"
          class="text-xs border-destructive/50"
        />
      </div>

      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">
          {{ t('sidebar.databases.actions.dropDatabase.cancel') }}
        </Button>
        <Button
          variant="destructive"
          size="sm"
          :disabled="!isMatch()"
          @click="emit('confirm')"
        >
          {{ t('sidebar.databases.actions.dropDatabase.confirm') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
