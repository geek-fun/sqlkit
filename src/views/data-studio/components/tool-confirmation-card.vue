<script setup lang="ts">
import type { AgentToolCall } from '@/store/dataStudioStore'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'

const props = defineProps<{
  toolCall: AgentToolCall
}>()

const emit = defineEmits<{
  confirm: [event: { toolCallId: string, action: 'allow_once' | 'allow_always' | 'deny' | 'deny_always' | 'cancel' }]
}>()

const { t } = useI18n()

function riskLabel(risk: string): string {
  switch (risk) {
    case 'destructive': return t('dataStudio.agent.riskDestructive')
    case 'elevated': return t('dataStudio.agent.riskElevated')
    default: return t('dataStudio.agent.riskSafe')
  }
}

function riskColor(risk: string): string {
  switch (risk) {
    case 'destructive': return 'text-red-600 bg-red-50 border-red-200'
    case 'elevated': return 'text-amber-600 bg-amber-50 border-amber-200'
    default: return 'text-green-600 bg-green-50 border-green-200'
  }
}
</script>

<template>
  <div
    class="mx-4 my-2 p-3 border rounded-lg"
    :class="riskColor(props.toolCall.riskLevel)"
  >
    <div class="mb-2 flex gap-2 items-center">
      <span class="i-carbon-warning shrink-0 h-4 w-4" />
      <span class="text-xs font-semibold">{{ riskLabel(props.toolCall.riskLevel) }}</span>
    </div>
    <div class="text-xs font-mono mb-1">
      <span class="font-semibold">{{ props.toolCall.toolName }}</span>
    </div>
    <pre class="text-xs mb-3 opacity-80 max-h-24 whitespace-pre-wrap overflow-y-auto">{{ JSON.stringify(props.toolCall.args, null, 2) }}</pre>
    <div class="flex flex-wrap gap-1.5">
      <Button
        size="sm"
        variant="outline"
        class="text-foreground"
        @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'allow_once' })"
      >
        {{ t('dataStudio.agent.allowOnce') }}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        class="text-foreground"
        @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'allow_always' })"
      >
        {{ t('dataStudio.agent.allowAlways') }}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        class="text-muted-foreground"
        @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'deny' })"
      >
        {{ t('dataStudio.agent.deny') }}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        class="text-muted-foreground hover:text-destructive"
        @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'deny_always' })"
      >
        {{ t('dataStudio.agent.denyAlways') }}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        class="text-muted-foreground"
        @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'cancel' })"
      >
        {{ t('dataStudio.agent.cancelRun') }}
      </Button>
    </div>
  </div>
</template>
