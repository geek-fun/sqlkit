<script setup lang="ts">
import type { LlmProvider } from '@/store/appStore'
import { storeToRefs } from 'pinia'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { SettingsFeedback } from '@/components/settings'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { useAppStore } from '@/store/appStore'
import ProviderFormDialog from './provider-form-dialog.vue'
import ProviderList from './provider-list.vue'

const { t } = useI18n()
const appStore = useAppStore()
const { llmSettings } = storeToRefs(appStore)

const showFormDialog = ref(false)
const editingProvider = ref<LlmProvider | undefined>(undefined)

const chatFeedback = ref<{ state: 'idle' | 'pending' | 'success' | 'error', label: string, detail?: string }>({ state: 'idle', label: '' })
let saveQueue: Promise<void> = Promise.resolve()
let saveRevision = 0
const saveChat = async (change: () => void) => {
  change()
  const revision = ++saveRevision
  chatFeedback.value = { state: 'pending', label: t('pages.settings.feedback.saving') }
  saveQueue = saveQueue.catch(() => {}).then(() => appStore.persistLlmSettings())
  try {
    await saveQueue
    if (revision === saveRevision)
      chatFeedback.value = { state: 'success', label: t('pages.settings.feedback.saved') }
  }
  catch (error) {
    if (revision === saveRevision)
      chatFeedback.value = { state: 'error', label: t('pages.settings.feedback.failed'), detail: error instanceof Error ? error.message : String(error) }
  }
}
function handleAdd() {
  editingProvider.value = undefined
  showFormDialog.value = true
}

function handleEdit(provider: LlmProvider) {
  editingProvider.value = provider
  showFormDialog.value = true
}

function handleCloseDialog() {
  showFormDialog.value = false
  editingProvider.value = undefined
}
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.settings.ai.title') }}</CardTitle>
      <CardDescription>{{ t('pages.settings.ai.description') }}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-8">
      <ProviderList
        @add="handleAdd"
        @edit="handleEdit"
      />

      <SettingsFeedback v-bind="chatFeedback" />
      <!-- Agent Config -->
      <section class="space-y-4">
        <div class="space-y-1">
          <h3 class="text-lg font-semibold">
            {{ t('pages.settings.ai.chat.title') }}
          </h3>
          <p class="text-sm text-muted-foreground">
            {{ t('pages.settings.ai.chat.description') }}
          </p>
        </div>

        <!-- Auto-compact -->
        <div class="px-5 py-4 border border-border/70 rounded-3xl bg-card/70 flex flex-col gap-3 shadow-sm md:flex-row md:items-center md:justify-between">
          <div class="min-w-0 space-y-1">
            <p class="text-base font-semibold">
              {{ t('pages.settings.ai.chat.autoCompactLabel') }}
            </p>
            <p class="text-sm text-muted-foreground">
              {{ t('pages.settings.ai.chat.autoCompactDescription') }}
            </p>
          </div>
          <Checkbox
            class="shrink-0"
            :checked="llmSettings.chat.autoCompact"
            @update:checked="(val: boolean | string) => saveChat(() => appStore.setAutoCompact(val === true || val === 'true'))"
          />
        </div>

        <!-- Max iterations -->
        <div class="px-5 py-4 border border-border/70 rounded-3xl bg-card/70 flex flex-col gap-3 shadow-sm md:flex-row md:items-center md:justify-between">
          <div class="min-w-0 space-y-1">
            <p class="text-base font-semibold">
              {{ t('pages.settings.ai.chat.maxIterationsLabel') }}
            </p>
            <p class="text-sm text-muted-foreground">
              {{ t('pages.settings.ai.chat.maxIterationsDescription') }}
            </p>
          </div>
          <Input
            type="number"
            min="1"
            max="1000"
            class="w-32"
            :model-value="llmSettings.chat.maxIterations"
            @update:model-value="val => saveChat(() => appStore.setMaxIterations(Number(val)))"
          />
        </div>

        <!-- Wall clock budget -->
        <div class="px-5 py-4 border border-border/70 rounded-3xl bg-card/70 flex flex-col gap-3 shadow-sm md:flex-row md:items-center md:justify-between">
          <div class="min-w-0 space-y-1">
            <p class="text-base font-semibold">
              {{ t('pages.settings.ai.chat.wallClockBudgetLabel') }}
            </p>
            <p class="text-sm text-muted-foreground">
              {{ t('pages.settings.ai.chat.wallClockBudgetDescription') }}
            </p>
          </div>
          <Input
            type="number"
            min="1"
            max="240"
            class="w-32"
            :model-value="llmSettings.chat.wallClockBudgetMin"
            @update:model-value="val => saveChat(() => appStore.setWallClockBudgetMin(Number(val)))"
          />
        </div>

        <!-- Token budget -->
        <div class="px-5 py-4 border border-border/70 rounded-3xl bg-card/70 flex flex-col gap-3 shadow-sm md:flex-row md:items-center md:justify-between">
          <div class="min-w-0 space-y-1">
            <p class="text-base font-semibold">
              {{ t('pages.settings.ai.chat.tokenBudgetLabel') }}
            </p>
            <p class="text-sm text-muted-foreground">
              {{ t('pages.settings.ai.chat.tokenBudgetDescription') }}
            </p>
          </div>
          <Input
            type="number"
            min="1000"
            step="1000"
            class="w-40"
            :model-value="llmSettings.chat.tokenBudget"
            @update:model-value="val => saveChat(() => appStore.setTokenBudget(Number(val)))"
          />
        </div>
      </section>

      <ProviderFormDialog
        :open="showFormDialog"
        :provider="editingProvider ?? null"
        @close="handleCloseDialog"
      />
    </CardContent>
  </Card>
</template>
