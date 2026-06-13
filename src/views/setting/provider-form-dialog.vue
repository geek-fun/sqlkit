<script setup lang="ts">
import type { LlmProvider } from '@/store/appStore'
import { ulid } from 'ulidx'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAppStore } from '@/store/appStore'

const props = defineProps<{
  open: boolean
  provider?: LlmProvider | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()
const appStore = useAppStore()

// ── Presets ───────────────────────────────────────────────────────────────────

type PresetId = 'openai' | 'anthropic' | 'azure' | 'custom'

type Preset = {
  id: PresetId
  name: string
  apiCompatibility: string
  baseUrl: string
}

const presets = computed<Preset[]>(() => [
  { id: 'openai', name: t('pages.settings.ai.form.presetOpenai'), apiCompatibility: 'openai', baseUrl: 'https://api.openai.com/v1' },
  { id: 'anthropic', name: t('pages.settings.ai.form.presetAnthropic'), apiCompatibility: 'anthropic', baseUrl: 'https://api.anthropic.com/v1' },
  { id: 'azure', name: t('pages.settings.ai.form.presetAzure'), apiCompatibility: 'azure', baseUrl: '' },
  { id: 'custom', name: t('pages.settings.ai.form.presetCustom'), apiCompatibility: '', baseUrl: '' },
])

// ── Proxy mode options ────────────────────────────────────────────────────────

const proxyModes = ['none', 'system', 'manual'] as const

const proxyModeLabels: Record<string, string> = {
  none: 'None',
  system: 'System',
  manual: 'Manual',
}

// ── Constants ──────────────────────────────────────────────────────────────────

const API_KEY_MASK = '••••••••••••'
const API_KEY_SENTINEL = '__SENTINEL__'

// ── Reactive state ────────────────────────────────────────────────────────────

const draft = ref<LlmProvider>(createEmptyDraft())
const showKey = ref(false)
const formErrors = ref<Record<string, string>>({})
const selectedPresetId = ref<PresetId | null>(null)

// ── Computed ──────────────────────────────────────────────────────────────────

const isEdit = computed(() => props.provider != null)

const isApiCompatibilityReadonly = computed(() => {
  if (isEdit.value)
    return true
  if (!selectedPresetId.value)
    return false
  return selectedPresetId.value !== 'custom'
})

const contextWindowInput = computed({
  get: () => draft.value.contextWindowOverride?.toString() ?? '',
  set: (val: string) => {
    const num = Number.parseInt(val, 10)
    draft.value.contextWindowOverride = Number.isNaN(num) ? undefined : num
  },
})

const modelsInput = computed({
  get: () => (draft.value.models ?? []).join(', '),
  set: (val: string) => {
    draft.value.models = val
      .split(',')
      .map(m => m.trim())
      .filter(Boolean)
  },
})

const apiKeyDisplayValue = computed({
  get: () => {
    if (draft.value.apiKey !== API_KEY_SENTINEL)
      return draft.value.apiKey
    if (showKey.value) {
      const original = props.provider?.apiKey ?? ''
      return original || API_KEY_MASK
    }
    return API_KEY_MASK
  },
  set: (val: string) => {
    if (val === API_KEY_MASK || val === '')
      return
    draft.value.apiKey = val
  },
})

// ── Helpers ───────────────────────────────────────────────────────────────────

function createEmptyDraft(): LlmProvider {
  return {
    id: ulid(),
    name: '',
    apiCompatibility: '',
    apiKey: '',
    baseUrl: '',
    enabled: true,
    proxy: '',
    proxyMode: 'none',
    contextWindowOverride: undefined,
  }
}

function resetForm(provider?: LlmProvider | null) {
  showKey.value = false
  formErrors.value = {}
  selectedPresetId.value = null

  if (provider) {
    draft.value = {
      ...provider,
      // Use sentinel to mask existing API key
      apiKey: provider.apiKey ? API_KEY_SENTINEL : '',
      proxy: provider.proxy ?? '',
      proxyMode: provider.proxyMode ?? 'none',
      contextWindowOverride: provider.contextWindowOverride ?? undefined,
    }
    // Derive the matching preset id from the provider's apiCompatibility
    const match = presets.value.find(p => p.apiCompatibility === provider.apiCompatibility && p.id !== 'custom')
    selectedPresetId.value = match ? match.id : 'custom'
  }
  else {
    draft.value = createEmptyDraft()
  }
}

function beginReplaceApiKey() {
  if (draft.value.apiKey === API_KEY_SENTINEL) {
    draft.value.apiKey = ''
    showKey.value = false
  }
}

// ── Actions ───────────────────────────────────────────────────────────────────

function selectPreset(preset: Preset) {
  selectedPresetId.value = preset.id
  draft.value.apiCompatibility = preset.apiCompatibility
  draft.value.baseUrl = preset.baseUrl
  formErrors.value = {}
}

function validateForm(): boolean {
  const errors: Record<string, string> = {}

  if (!draft.value.name.trim()) {
    errors.name = t('pages.settings.ai.form.validation.nameRequired')
  }

  if (!draft.value.apiCompatibility.trim()) {
    errors.apiCompatibility = t('pages.settings.ai.form.validation.apiCompatibilityRequired')
  }

  if (draft.value.baseUrl && !/^https?:\/\//.test(draft.value.baseUrl)) {
    errors.baseUrl = t('pages.settings.ai.form.validation.baseUrlInvalid')
  }

  formErrors.value = errors
  return Object.keys(errors).length === 0
}

function handleSave() {
  if (!validateForm())
    return

  // Resolve API key: if unchanged (sentinel or user clicked but didn't type), keep original
  let resolvedApiKey = draft.value.apiKey
  if (resolvedApiKey === API_KEY_SENTINEL || (!resolvedApiKey && isEdit.value)) {
    resolvedApiKey = props.provider?.apiKey ?? ''
  }

  const cleaned: LlmProvider = {
    ...draft.value,
    apiKey: resolvedApiKey,
    name: draft.value.name.trim(),
    proxy: draft.value.proxy?.trim() || undefined,
    proxyMode: draft.value.proxyMode ?? 'none',
    contextWindowOverride: draft.value.contextWindowOverride || undefined,
  }

  if (isEdit.value) {
    appStore.updateProvider(draft.value.id, { ...cleaned })
  }
  else {
    appStore.addProvider(cleaned)
  }

  emit('close')
}

// ── Watchers ──────────────────────────────────────────────────────────────────

watch(() => props.open, (open) => {
  if (open) {
    resetForm(props.provider)
    // Always start with key masked; user must click eye icon to reveal
  }
})
</script>

<template>
  <Dialog :open="open" @update:open="$emit('close')">
    <DialogContent class="sm:max-w-[480px]">
      <DialogTitle>
        {{ isEdit ? t('pages.settings.ai.form.editTitle') : t('pages.settings.ai.form.addTitle') }}
      </DialogTitle>
      <DialogDescription>
        {{ t('pages.settings.ai.form.providerTypeHelp') }}
      </DialogDescription>

      <!-- Provider type selector — only in create mode -->
      <div v-if="!isEdit" class="space-y-2">
        <Label>{{ t('pages.settings.ai.form.providerType') }}</Label>
        <div class="gap-2 grid grid-cols-2">
          <button
            v-for="preset in presets"
            :key="preset.id"
            type="button"
            class="text-sm font-medium px-3 py-2.5 border rounded-lg flex transition-colors items-center justify-center" :class="[
              selectedPresetId === preset.id
                ? 'border-primary bg-primary/10 text-primary'
                : 'border-input bg-background hover:bg-accent hover:text-accent-foreground',
            ]"
            @click="selectPreset(preset)"
          >
            {{ preset.name }}
          </button>
        </div>
      </div>

      <!-- Form fields -->
      <div class="space-y-4">
        <!-- Display Name -->
        <div class="space-y-1">
          <Label for="name">{{ t('pages.settings.ai.form.name') }}</Label>
          <Input
            id="name"
            v-model="draft.name"
            :placeholder="t('pages.settings.ai.form.namePlaceholder')"
            :class="{ 'border-destructive': formErrors.name }"
          />
          <p v-if="formErrors.name" class="text-sm text-destructive">
            {{ formErrors.name }}
          </p>
        </div>

        <!-- API Compatibility -->
        <div class="space-y-1">
          <Label for="api-compatibility">{{ t('pages.settings.ai.form.apiCompatibility') }}</Label>
          <Input
            id="api-compatibility"
            v-model="draft.apiCompatibility"
            :readonly="isApiCompatibilityReadonly"
            :placeholder="t('pages.settings.ai.form.apiCompatibilityPlaceholder')"
            :class="{
              'border-destructive': formErrors.apiCompatibility,
              'cursor-not-allowed opacity-60': isApiCompatibilityReadonly,
            }"
          />
          <p v-if="formErrors.apiCompatibility" class="text-sm text-destructive">
            {{ formErrors.apiCompatibility }}
          </p>
        </div>

        <!-- API Key -->
        <div class="space-y-1">
          <Label for="api-key">{{ t('pages.settings.ai.form.apiKey') }}</Label>
          <div class="relative">
            <Input
              id="api-key"
              :model-value="apiKeyDisplayValue"
              :type="showKey ? 'text' : 'password'"
              :readonly="draft.apiKey === API_KEY_SENTINEL"
              :placeholder="t('pages.settings.ai.form.apiKeyPlaceholder')"
              class="pr-9"
              autocomplete="off"
              @update:model-value="(val) => apiKeyDisplayValue = String(val)"
              @click="beginReplaceApiKey"
            />
            <button
              type="button"
              class="text-muted-foreground px-2.5 flex items-center inset-y-0 right-0 absolute hover:text-foreground"
              :title="showKey ? t('pages.settings.ai.form.hideKey') : t('pages.settings.ai.form.showKey')"
              @click="showKey = !showKey"
            >
              <!-- Eye icon (hidden → shown) -->
              <svg
                v-if="!showKey"
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
              <!-- Eye-off icon (shown → hidden) -->
              <svg
                v-else
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M9.88 9.88a3 3 0 1 0 4.24 4.24" />
                <path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68" />
                <path d="M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61" />
                <line x1="2" x2="22" y1="2" y2="22" />
              </svg>
            </button>
          </div>
        </div>

        <!-- Base URL -->
        <div class="space-y-1">
          <Label for="base-url">{{ t('pages.settings.ai.form.baseUrl') }}</Label>
          <Input
            id="base-url"
            v-model="draft.baseUrl"
            :placeholder="t('pages.settings.ai.form.baseUrlPlaceholder')"
            :class="{ 'border-destructive': formErrors.baseUrl }"
          />
          <p v-if="formErrors.baseUrl" class="text-sm text-destructive">
            {{ formErrors.baseUrl }}
          </p>
        </div>

        <!-- Proxy Mode + URL -->
        <div class="space-y-2">
          <Label>{{ t('pages.settings.ai.form.proxyMode') }}</Label>
          <div class="p-0.5 rounded-lg bg-muted/60 inline-flex">
            <button
              v-for="mode in proxyModes"
              :key="mode"
              type="button"
              class="text-sm font-medium px-3 py-1.5 rounded-md transition-colors" :class="[
                draft.proxyMode === mode
                  ? 'bg-background text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground',
              ]"
              @click="draft.proxyMode = mode"
            >
              {{ proxyModeLabels[mode] }}
            </button>
          </div>
          <Input
            v-if="draft.proxyMode === 'manual'"
            v-model="draft.proxy"
            :placeholder="t('pages.settings.ai.form.proxyPlaceholder')"
          />
        </div>

        <!-- Context Window Override -->
        <div class="space-y-1">
          <Label for="context-window">{{ t('pages.settings.ai.form.contextWindow') }}</Label>
          <Input
            id="context-window"
            v-model="contextWindowInput"
            type="number"
            step="1024"
            min="1024"
            placeholder="4096"
          />
        </div>

        <!-- Models (manual entry) -->
        <div class="space-y-1">
          <Label for="models">{{ t('pages.settings.ai.form.models') }}</Label>
          <Input
            id="models"
            v-model="modelsInput"
            :placeholder="t('pages.settings.ai.form.modelsPlaceholder')"
          />
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.ai.form.modelsHelp') }}
          </p>
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="$emit('close')">
          {{ t('common.buttons.cancel') }}
        </Button>
        <Button @click="handleSave">
          {{ isEdit ? t('common.buttons.save') : t('pages.settings.ai.form.save') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
