<script setup lang="ts">
import type { AiAction, AiPermissions } from '@/common'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { AI_ACTIONS, aiPreset, getAiPolicy, resolveAiPermissions, UPGRADE_URL } from '@/common'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { ProBadge } from '@/components/upgrade'
import { useEntitlementStore } from '@/store'
import AccessLevelPicker from './AccessLevelPicker.vue'

const props = defineProps<{ modelValue?: AiPermissions, connectionId?: string | number, initiallyExpanded?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: AiPermissions] }>()
const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const canEdit = computed(() => entitlementStore.isLocalUltimate)
const loading = ref(true)
const error = ref('')
const defaults = ref<AiPermissions>({})
const editingCustom = ref(false)
const expanded = ref(props.initiallyExpanded === true)
watch(error, (value) => {
  if (value)
    expanded.value = true
})
let request = 0
const load = async () => {
  editingCustom.value = false
  const current = ++request
  loading.value = true
  error.value = ''
  if (!canEdit.value) {
    loading.value = false
    return
  }
  try {
    const policy = await getAiPolicy()
    if (current !== request)
      return
    defaults.value = policy.permission_defaults
    emit('update:modelValue', policy.connection_permissions[String(props.connectionId)] ?? {})
  }
  catch (e) {
    if (current === request)
      error.value = String(e)
  }
  finally {
    if (current === request)
      loading.value = false
  }
}
watch(() => [props.connectionId, canEdit.value], load, { immediate: true })
const presets = ['inherit', 'none', 'read', 'data', 'full']
const preset = computed(() => editingCustom.value
  ? 'custom'
  : (
      presets.find(value => AI_ACTIONS.every(action => aiPreset(value)[action] === props.modelValue?.[action])) ?? 'custom'
    ))
const effective = computed(() => resolveAiPermissions(defaults.value, props.modelValue))
const decisionText = (action: AiAction) => props.modelValue?.[action] === undefined
  ? t('aiPermissions.defaultDecision', { decision: t(defaults.value[action] ? 'aiPermissions.allow' : 'aiPermissions.deny') })
  : `${t(props.modelValue[action] ? 'aiPermissions.allow' : 'aiPermissions.deny')} · ${t('aiPermissions.fromConnection')}`
const update = (action: AiAction, value: boolean) => {
  if (!canEdit.value)
    return
  editingCustom.value = true
  const { [action]: _previous, ...rest } = props.modelValue ?? {}
  emit('update:modelValue', { ...rest, [action]: value })
}
defineExpose({ ready: computed(() => !canEdit.value || (!loading.value && !error.value)) })
const selectPreset = (value: string) => {
  if (!canEdit.value)
    return
  editingCustom.value = value === 'custom'
  if (value === 'custom')
    return
  emit('update:modelValue', aiPreset(value))
}
</script>

<template>
  <button
    v-if="!canEdit"
    type="button"
    class="group px-3 py-2.5 text-left border rounded-lg border-dashed flex gap-2.5 w-full cursor-pointer transition-colors items-center hover:bg-accent/50"
    @click="openUrl(UPGRADE_URL)"
  >
    <ProBadge size="xs" />
    <span class="text-sm text-muted-foreground truncate transition-colors group-hover:text-foreground/80">{{ t('aiPermissions.ultimateOnly') }}</span>
    <span class="text-xs text-primary-text font-medium ml-auto px-2 py-1 rounded-md bg-primary/10 shrink-0 transition-colors group-hover:bg-primary/15">{{ t('plan.upgrade.cta') }}</span>
  </button>
  <details v-else :open="expanded" class="group border border-border rounded-lg" @toggle="expanded = ($event.target as HTMLDetailsElement).open">
    <summary class="text-sm font-medium p-3 list-none rounded-lg flex gap-2 cursor-pointer items-center justify-between focus-visible:outline-none [&::-webkit-details-marker]:hidden focus-visible:ring-2 focus-visible:ring-ring">
      <span class="flex gap-2 items-center">
        <span aria-hidden="true" class="i-lucide-chevron-right text-muted-foreground shrink-0 h-4 w-4 group-open:rotate-90" />
        {{ t('aiPermissions.title') }}
      </span>
      <span class="text-xs text-muted-foreground font-normal text-right" role="status">
        {{ loading ? t('aiPermissions.loading') : error ? t('aiPermissions.unavailable') : t(`aiPermissions.${preset === 'read' ? 'readPreset' : preset}`) }}
      </span>
    </summary>
    <div class="px-3 pb-3 space-y-3">
      <p class="text-xs text-muted-foreground leading-relaxed">
        {{ t('aiPermissions.help') }}
      </p>
      <p v-if="loading" role="status" class="text-sm text-muted-foreground">
        {{ t('aiPermissions.loading') }}
      </p>
      <div v-else-if="error" role="alert" class="space-y-2">
        <p class="text-sm text-destructive-text break-words">
          {{ t('aiPermissions.loadFailed') }} {{ error }}
        </p>
        <Button type="button" variant="outline" size="sm" @click="load">
          {{ t('aiPermissions.retry') }}
        </Button>
      </div>
      <template v-else>
        <AccessLevelPicker
          :id="`ai-preset-${connectionId ?? 'new'}`"
          :model-value="preset"
          inherit
          @update:model-value="selectPreset"
        />
        <p class="text-xs text-muted-foreground leading-relaxed" role="status">
          {{ t('aiPermissions.effective') }}:
          {{ AI_ACTIONS.filter(action => effective[action]).map(action => t(`aiPermissions.${action}`)).join(', ') || t('aiPermissions.none') }}
        </p>
        <div v-if="preset === 'custom'" :id="`ai-actions-${connectionId ?? 'new'}`" class="border-t border-border">
          <div v-for="action in AI_ACTIONS" :key="action" class="py-2 border-b border-border flex gap-3 items-center justify-between last:pb-0 last:border-b-0">
            <Label :for="`ai-${connectionId ?? 'new'}-${action}`" class="leading-snug">{{ t(`aiPermissions.${action}`) }}</Label>
            <div class="flex shrink-0 gap-3 items-center">
              <span class="text-xs text-muted-foreground">{{ decisionText(action) }}</span>
              <Switch :id="`ai-${connectionId ?? 'new'}-${action}`" :checked="effective[action] === true" @update:checked="update(action, $event)" />
            </div>
          </div>
        </div>
      </template>
    </div>
  </details>
</template>
