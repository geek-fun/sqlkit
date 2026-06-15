<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TriggerInfo } from '@/datasources/browseApi'
import { browseApi } from '@/datasources/browseApi'
import { Button } from '@/components/ui/button'
import { Spinner } from '@/components/ui/spinner'
import DdlModal from './DdlModal.vue'

const props = defineProps<{
  connectionId: string
  database: string
  schema: string | null
  tableName: string
}>()

const { t } = useI18n()

const triggers = ref<TriggerInfo[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

// DDL modal
const ddlModalOpen = ref(false)
const ddlTitle = ref('')
const ddlContent = ref('')

async function fetchTriggers() {
  loading.value = true
  error.value = null
  try {
    triggers.value = await browseApi.listTriggers(
      props.connectionId,
      props.database,
      props.schema,
      props.tableName,
    )
  }
  catch (err) {
    error.value = String(err)
  }
  finally {
    loading.value = false
  }
}

async function openDdlModal(trigger: TriggerInfo) {
  ddlTitle.value = `${trigger.name} — Trigger`
  if (trigger.ddl) {
    ddlContent.value = trigger.ddl
  }
  else {
    ddlContent.value = ''
    try {
      ddlContent.value = await browseApi.getObjectDdl(
        props.connectionId,
        props.database,
        props.schema,
        trigger.name,
        'TRIGGER',
      )
    }
    catch (err) {
      ddlContent.value = `-- Failed to load DDL:\n-- ${String(err)}`
    }
  }
  ddlModalOpen.value = true
}

onMounted(fetchTriggers)
</script>

<template>
  <div class="p-3 flex-1 overflow-auto">
    <div v-if="loading" class="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
      <Spinner size="sm" />
      {{ t('common.loading') }}
    </div>
    <div v-else-if="error" class="py-8 text-sm text-destructive text-center">
      {{ error }}
    </div>
    <div v-else-if="triggers.length === 0" class="py-8 text-sm text-muted-foreground text-center">
      {{ t('components.triggersTab.empty') }}
    </div>
    <div v-else class="rounded-md border">
      <table class="w-full text-xs">
        <thead>
          <tr class="text-left bg-muted/50">
            <th class="px-3 py-2 font-medium">
              {{ t('components.triggersTab.name') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.triggersTab.timing') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.triggersTab.event') }}
            </th>
            <th class="px-3 py-2 font-medium">
              {{ t('components.triggersTab.ddl') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="trigger in triggers"
            :key="trigger.name"
            class="border-t border-border/50 hover:bg-accent/30"
          >
            <td class="px-3 py-1.5 font-medium">
              {{ trigger.name }}
            </td>
            <td class="px-3 py-1.5 font-mono text-muted-foreground">
              {{ trigger.action_timing }}
            </td>
            <td class="px-3 py-1.5 font-mono text-muted-foreground">
              {{ trigger.event }}
            </td>
            <td class="px-3 py-1.5">
              <Button variant="ghost" size="sm" class="text-xs h-6" @click="openDdlModal(trigger)">
                {{ t('components.triggersTab.viewDdl') }}
              </Button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <DdlModal
      :open="ddlModalOpen"
      :title="ddlTitle"
      :ddl="ddlContent"
      @update:open="ddlModalOpen = $event"
    />
  </div>
</template>
