<script setup lang="ts">
import type { AccessDenial, AiPermissions } from '@/common'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { saveAiPermissions } from '@/common'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { PaidGate } from '@/components/upgrade'
import { agentApi } from '@/datasources/agentApi'
import { useDataStudioStore } from '@/store/dataStudioStore'
import ModifySourceModal from '@/views/data-studio/components/modify-source-modal.vue'
import ConnectionPermissionEditor from './ConnectionPermissionEditor.vue'
import DefaultPermissionEditor from './DefaultPermissionEditor.vue'

const props = defineProps<{ open: boolean, denial: AccessDenial, toolName: string, sessionId: string }>()
const emit = defineEmits<{ 'update:open': [value: boolean] }>()
const { t } = useI18n()
const store = useDataStudioStore()
const rules = ref<AiPermissions>({})
const editor = ref<InstanceType<typeof ConnectionPermissionEditor> | null>(null)
const saving = ref(false)
const error = ref('')
const ruleRemoved = ref(false)
const sourceIdx = computed(() => store.activeSession?.id === props.sessionId
  ? store.activeSession.sources.filter(source => !source.detached).findIndex(source => source.sourceId === props.denial.sourceId)
  : -1)
const reviewConversation = computed(() => props.denial.source === 'conversation' && sourceIdx.value >= 0)
watch(() => [props.open, props.denial], () => {
  error.value = ''
  ruleRemoved.value = false
})
const saveConnection = async () => {
  if (!props.denial.connectionId || !editor.value?.ready || saving.value)
    return
  saving.value = true
  error.value = ''
  try {
    await saveAiPermissions(props.denial.connectionId, rules.value)
    emit('update:open', false)
  }
  catch (cause) {
    error.value = String(cause)
  }
  finally {
    saving.value = false
  }
}
const removeRule = async () => {
  saving.value = true
  error.value = ''
  try {
    const rules = await agentApi.loadConfirmationRules(props.sessionId)
    const rule = rules.find(rule => rule.tool_name === props.toolName && rule.action === 'deny')
    if (rule)
      await store.removeConfirmationRule(rule.id)
    ruleRemoved.value = true
  }
  catch (cause) {
    error.value = String(cause)
  }
  finally {
    saving.value = false
  }
}
</script>

<template>
  <ModifySourceModal v-if="reviewConversation" :open="open" :source-idx="sourceIdx" @update:open="emit('update:open', $event)" />
  <Dialog v-else :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="max-h-[85vh] max-w-[calc(100vw-2rem)] overflow-y-auto sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>{{ t('dataStudio.agent.message.reviewPermissions') }}</DialogTitle>
        <DialogDescription>{{ t('dataStudio.agent.message.reviewHelp') }}</DialogDescription>
      </DialogHeader>
      <p v-if="error" role="alert" class="text-sm text-destructive-text break-words">
        {{ error }}
      </p>
      <template v-if="denial.source === 'rule'">
        <p class="text-sm leading-relaxed">
          {{ t('dataStudio.agent.message.denialSource.rule') }}
        </p>
        <p v-if="ruleRemoved" role="status" class="text-sm">
          {{ t('dataStudio.agent.message.ruleRemoved') }}
        </p>
        <Button v-else type="button" variant="outline" :disabled="saving" @click="removeRule">
          {{ t('dataStudio.agent.message.removeDenyRule') }}
        </Button>
      </template>
      <p v-else-if="denial.source === 'conversation'" class="text-sm leading-relaxed">
        {{ t('dataStudio.agent.message.sourceUnavailable') }}
      </p>
      <PaidGate v-else feature="mcp_bridge">
        <DefaultPermissionEditor v-if="denial.source === 'system' || !denial.connectionId" />
        <fieldset v-else :disabled="saving" class="min-w-0 space-y-4">
          <ConnectionPermissionEditor ref="editor" v-model="rules" :connection-id="denial.connectionId" initially-expanded />
          <Button type="button" :disabled="saving || !editor?.ready" @click="saveConnection">
            {{ t(saving ? 'aiPermissions.saving' : 'dataStudio.modifySource.saveChanges') }}
          </Button>
        </fieldset>
      </PaidGate>
      <DialogFooter>
        <Button type="button" variant="outline" :disabled="saving" @click="emit('update:open', false)">
          {{ t('dataStudio.agent.message.backToChat') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
