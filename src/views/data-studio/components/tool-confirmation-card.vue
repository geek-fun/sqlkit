<script setup lang="ts">
import type { ArgBlock, ImpactHint } from '@/common'
import type { AgentToolCall, RiskLevel } from '@/store/dataStudioStore'
import { computed, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { highlightCodeHtml, presentToolCall } from '@/common'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useHighlightTheme } from '@/composables'

const props = defineProps<{
  toolCall: AgentToolCall
}>()

const emit = defineEmits<{
  confirm: [event: { toolCallId: string, action: 'allow_once' | 'allow_always' | 'deny' | 'deny_always' | 'cancel' }]
}>()

const { t } = useI18n()
useHighlightTheme()

const EXPAND_LINE_THRESHOLD = 10
const EXPAND_CHAR_THRESHOLD = 480

const presentation = computed(() =>
  presentToolCall(props.toolCall.toolName, props.toolCall.args),
)

const fieldBlocks = computed(() =>
  presentation.value.blocks.filter(
    (block): block is Extract<ArgBlock, { kind: 'field' }> => block.kind === 'field',
  ),
)

const codeBlocks = computed(() =>
  presentation.value.blocks.filter(
    (block): block is Extract<ArgBlock, { kind: 'code' }> => block.kind === 'code',
  ),
)

const hasMetadata = computed(() =>
  Boolean(presentation.value.target || presentation.value.operation || fieldBlocks.value.length > 0),
)

const expandedKeys = ref<Record<string, boolean>>({})
const copiedKey = ref<string | null>(null)
let copyResetTimer: ReturnType<typeof setTimeout> | null = null

onBeforeUnmount(() => {
  if (copyResetTimer)
    clearTimeout(copyResetTimer)
})

const isExpandable = (text: string): boolean =>
  text.length > EXPAND_CHAR_THRESHOLD || text.split('\n').length > EXPAND_LINE_THRESHOLD

const toggleExpand = (key: string) => {
  expandedKeys.value = {
    ...expandedKeys.value,
    [key]: !expandedKeys.value[key],
  }
}

const copyBlock = async (block: Extract<ArgBlock, { kind: 'code' }>) => {
  try {
    await navigator.clipboard.writeText(block.text)
    copiedKey.value = block.key
    if (copyResetTimer)
      clearTimeout(copyResetTimer)
    copyResetTimer = setTimeout(() => {
      copiedKey.value = null
    }, 1800)
  }
  catch {
    // Clipboard unavailable
  }
}

const highlightedBlock = (block: Extract<ArgBlock, { kind: 'code' }>): string =>
  highlightCodeHtml(block.text, block.language)

const blockLabel = (block: Extract<ArgBlock, { kind: 'code' }>): string => {
  const langTag = block.language === 'text'
    ? ''
    : block.language.toUpperCase()
  const keyLabel = block.key.toLowerCase() === block.language
    ? ''
    : block.key
  if (keyLabel && langTag)
    return `${keyLabel} · ${langTag}`
  return langTag || keyLabel || block.key
}

const riskLabel = (risk: RiskLevel): string => {
  switch (risk) {
    case 'destructive': return t('dataStudio.agent.riskDestructive')
    case 'elevated': return t('dataStudio.agent.riskElevated')
    default: return t('dataStudio.agent.riskSafe')
  }
}

const riskDescription = (risk: RiskLevel): string => {
  switch (risk) {
    case 'destructive': return t('dataStudio.agent.toolConfirmation.riskDescription.destructive')
    case 'elevated': return t('dataStudio.agent.toolConfirmation.riskDescription.elevated')
    default: return t('dataStudio.agent.toolConfirmation.riskDescription.safe')
  }
}

const riskBadgeVariant = computed(() => {
  switch (props.toolCall.riskLevel) {
    case 'destructive': return 'destructive' as const
    case 'elevated': return 'warning' as const
    default: return 'secondary' as const
  }
})

const riskIconClass = computed(() => {
  switch (props.toolCall.riskLevel) {
    case 'destructive': return 'i-carbon-warning-filled text-destructive-text'
    case 'elevated': return 'i-carbon-warning-alt text-warning'
    default: return 'i-carbon-view text-muted-foreground'
  }
})

const cardRiskClass = computed(() => {
  switch (props.toolCall.riskLevel) {
    case 'destructive': return 'border-destructive-border border-l-destructive'
    case 'elevated': return 'border-warning-border border-l-warning'
    default: return 'border-border border-l-muted-foreground/50'
  }
})

const headerRiskClass = computed(() => {
  switch (props.toolCall.riskLevel) {
    case 'destructive': return 'bg-destructive-muted border-destructive-border/70'
    case 'elevated': return 'bg-warning-muted border-warning-border/70'
    default: return 'bg-muted/40 border-border'
  }
})

const hintBannerClass = computed(() => {
  const isSevere = props.toolCall.riskLevel === 'destructive'
    || presentation.value.hints.some(hint => hint === 'dropsObject' || hint === 'truncates')
  return isSevere
    ? 'border-destructive-border bg-destructive-muted text-destructive-text'
    : 'border-warning-border bg-warning-muted text-warning'
})

const hintMessage = (hint: ImpactHint): string => {
  switch (hint) {
    case 'noWhereClause': return t('dataStudio.agent.toolConfirmation.hints.noWhereClause')
    case 'dropsObject': return t('dataStudio.agent.toolConfirmation.hints.dropsObject')
    case 'truncates': return t('dataStudio.agent.toolConfirmation.hints.truncates')
    case 'multipleStatements': return t('dataStudio.agent.toolConfirmation.hints.multipleStatements')
  }
}
</script>

<template>
  <div
    class="text-foreground mx-4 my-2 border border-l-[3px] rounded-lg bg-card overflow-hidden"
    :class="cardRiskClass"
  >
    <div
      class="px-3 py-2 border-b flex gap-2 items-start justify-between"
      :class="headerRiskClass"
    >
      <div class="flex flex-1 gap-2 min-w-0 items-start">
        <span class="mt-0.5 shrink-0 h-4 w-4" :class="riskIconClass" aria-hidden="true" />
        <div class="min-w-0">
          <div class="flex flex-wrap gap-1.5 items-center">
            <Badge :variant="riskBadgeVariant" class="text-caption px-2 py-0">
              {{ riskLabel(props.toolCall.riskLevel) }}
            </Badge>
            <span class="text-xs text-muted-foreground font-mono break-all">
              {{ props.toolCall.toolName }}
            </span>
          </div>
          <p class="text-xs text-foreground/85 leading-snug mt-1">
            {{ riskDescription(props.toolCall.riskLevel) }}
          </p>
        </div>
      </div>
    </div>

    <div
      v-if="hasMetadata"
      class="text-xs px-3 py-2 border-b border-border/70 bg-muted/25 flex flex-wrap gap-x-4 gap-y-1.5 items-center"
    >
      <div v-if="presentation.target" class="inline-flex gap-1.5 min-w-0 items-center">
        <span class="i-carbon-data-base text-muted-foreground shrink-0 h-3.5 w-3.5" aria-hidden="true" />
        <span class="text-muted-foreground">{{ t('dataStudio.agent.toolConfirmation.targetSource') }}</span>
        <span class="text-foreground font-mono font-semibold break-all">{{ presentation.target }}</span>
      </div>

      <div v-if="presentation.operation" class="inline-flex gap-1.5 min-w-0 items-center">
        <span class="text-muted-foreground">{{ t('dataStudio.agent.toolConfirmation.operation') }}</span>
        <Badge variant="outline" class="text-caption font-mono px-1.5 py-0">
          {{ presentation.operation.verb }}
        </Badge>
        <span v-if="presentation.operation.object" class="text-foreground font-mono font-semibold break-all">
          → {{ presentation.operation.object }}
        </span>
      </div>

      <div
        v-for="field in fieldBlocks"
        :key="field.key"
        class="inline-flex gap-1.5 min-w-0 items-center"
      >
        <span class="text-muted-foreground font-mono">{{ field.key }}</span>
        <span class="text-foreground font-mono break-all">{{ field.value }}</span>
      </div>
    </div>

    <div class="px-3 py-2.5 space-y-2.5">
      <div
        v-for="block in codeBlocks"
        :key="block.key"
        class="border border-border rounded-md bg-background overflow-hidden"
      >
        <div class="px-2.5 py-1 border-b border-border/70 bg-muted/45 flex gap-2 items-center justify-between">
          <span class="text-caption text-muted-foreground tracking-wide font-mono uppercase">
            {{ blockLabel(block) }}
          </span>
          <div class="flex gap-1 items-center">
            <button
              v-if="isExpandable(block.text)"
              type="button"
              class="text-caption text-muted-foreground px-1.5 py-0.5 rounded inline-flex gap-1 transition-colors items-center hover:text-foreground hover:bg-accent"
              @click="toggleExpand(block.key)"
            >
              <span
                class="h-3 w-3"
                :class="expandedKeys[block.key] ? 'i-carbon-chevron-up' : 'i-carbon-chevron-down'"
                aria-hidden="true"
              />
              <span>
                {{
                  expandedKeys[block.key]
                    ? t('dataStudio.agent.toolConfirmation.showLess')
                    : t('dataStudio.agent.toolConfirmation.showAll')
                }}
              </span>
            </button>
            <button
              type="button"
              class="text-caption text-muted-foreground px-1.5 py-0.5 rounded inline-flex gap-1 transition-colors items-center hover:text-foreground hover:bg-accent"
              :aria-label="t('dataStudio.agent.toolConfirmation.copy')"
              @click="copyBlock(block)"
            >
              <span
                class="h-3 w-3"
                :class="copiedKey === block.key ? 'i-carbon-checkmark text-success' : 'i-carbon-copy'"
                aria-hidden="true"
              />
              <span>
                {{
                  copiedKey === block.key
                    ? t('dataStudio.agent.toolConfirmation.copied')
                    : t('dataStudio.agent.toolConfirmation.copy')
                }}
              </span>
            </button>
          </div>
        </div>
        <!-- eslint-disable vue/no-v-html -->
        <pre
          class="text-xs leading-relaxed font-mono m-0 p-2.5 whitespace-pre-wrap break-words overflow-x-auto overflow-y-auto"
          :class="expandedKeys[block.key] ? 'max-h-[480px]' : 'max-h-60'"
        ><code v-if="block.language === 'text'">{{ block.text }}</code><code v-else class="hljs tool-code-content" v-html="highlightedBlock(block)" /></pre>
        <!-- eslint-enable vue/no-v-html -->
      </div>

      <div
        v-if="presentation.hints.length > 0"
        class="text-xs px-2.5 py-2 border rounded-md space-y-1"
        :class="hintBannerClass"
        role="note"
      >
        <div
          v-for="hint in presentation.hints"
          :key="hint"
          class="flex gap-1.5 items-start"
        >
          <span class="i-carbon-warning mt-0.5 shrink-0 h-3.5 w-3.5" aria-hidden="true" />
          <span>{{ hintMessage(hint) }}</span>
        </div>
      </div>

      <div class="pt-0.5 flex flex-wrap gap-1.5 items-center">
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
          class="text-muted-foreground hover:text-foreground"
          @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'deny' })"
        >
          {{ t('dataStudio.agent.deny') }}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          class="text-muted-foreground hover:text-destructive-text"
          @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'deny_always' })"
        >
          {{ t('dataStudio.agent.denyAlways') }}
        </Button>
        <span class="flex-1 min-w-2" />
        <Button
          size="sm"
          variant="ghost"
          class="text-muted-foreground hover:text-foreground"
          @click="emit('confirm', { toolCallId: props.toolCall.id, action: 'cancel' })"
        >
          {{ t('dataStudio.agent.cancelRun') }}
        </Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tool-code-content {
  display: block;
  background: transparent;
  padding: 0;
  font-family: inherit;
  font-size: inherit;
  line-height: inherit;
}
</style>
