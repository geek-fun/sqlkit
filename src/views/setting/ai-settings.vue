<script setup lang="ts">
import type { LlmProvider } from '@/store/appStore'
import { storeToRefs } from 'pinia'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
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
const isEditingPrompt = ref(false)
const draftPrompt = ref('')
const promptSaveError = ref('')
const systemPrompt = computed(() => llmSettings.value.chat.systemPrompt ?? '')
const hasPromptChanges = computed(() => draftPrompt.value !== systemPrompt.value)

const startEditPrompt = () => {
  draftPrompt.value = systemPrompt.value
  promptSaveError.value = ''
  isEditingPrompt.value = true
}

const cancelEditPrompt = () => {
  draftPrompt.value = ''
  isEditingPrompt.value = false
}

const savePrompt = async () => {
  if (!hasPromptChanges.value)
    return
  const result = await appStore.saveChatSettings({ systemPrompt: draftPrompt.value })
  if (result.success)
    isEditingPrompt.value = false
  else
    promptSaveError.value = result.error ?? t('pages.settings.ai.chat.systemPromptSaveFailed')
}

const deletePrompt = async () => {
  const result = await appStore.saveChatSettings({ systemPrompt: '' })
  if (result.success) {
    draftPrompt.value = ''
    isEditingPrompt.value = false
  }
  else {
    promptSaveError.value = result.error ?? t('pages.settings.ai.chat.systemPromptSaveFailed')
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

        <div class="p-4 border border-border/70 rounded-xl bg-card/70 space-y-3">
          <div class="flex gap-4 items-start justify-between">
            <div class="space-y-1">
              <p class="font-semibold">
                {{ t('pages.settings.ai.chat.systemPromptLabel') }}
              </p>
              <p class="text-sm text-muted-foreground">
                {{ t('pages.settings.ai.chat.systemPromptDescription') }}
              </p>
            </div>
            <div v-if="!isEditingPrompt" class="flex shrink-0 gap-1">
              <Button v-if="!systemPrompt" variant="outline" size="sm" @click="startEditPrompt">
                {{ t('pages.settings.ai.chat.systemPromptAdd') }}
              </Button>
              <template v-else>
                <Button variant="ghost" size="sm" @click="startEditPrompt">
                  {{ t('pages.settings.ai.chat.systemPromptEdit') }}
                </Button>
                <Button variant="ghost" size="sm" class="text-destructive" @click="deletePrompt">
                  {{ t('pages.settings.ai.chat.systemPromptDelete') }}
                </Button>
              </template>
            </div>
          </div>
          <textarea
            v-if="isEditingPrompt"
            v-model="draftPrompt"
            autocomplete="off"
            class="text-sm px-3 py-2 border border-input rounded-md bg-background min-h-28 w-full resize-y"
            :placeholder="t('pages.settings.ai.chat.systemPromptLabel')"
          />
          <p v-else class="text-sm text-muted-foreground whitespace-pre-wrap line-clamp-3">
            {{ systemPrompt || t('pages.settings.ai.chat.systemPromptEmpty') }}
          </p>
          <div v-if="isEditingPrompt" class="flex gap-2 justify-end">
            <Button variant="outline" size="sm" @click="cancelEditPrompt">
              {{ t('pages.settings.ai.chat.systemPromptCancel') }}
            </Button>
            <Button size="sm" :disabled="!hasPromptChanges" @click="savePrompt">
              {{ t('pages.settings.ai.chat.systemPromptSave') }}
            </Button>
          </div>
          <p v-if="promptSaveError" class="text-sm text-destructive" role="alert">
            {{ promptSaveError }}
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
            @update:checked="(val: boolean | string) => appStore.setAutoCompact(val === true || val === 'true')"
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
            @update:model-value="val => appStore.setMaxIterations(Number(val))"
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
            @update:model-value="val => appStore.setWallClockBudgetMin(Number(val))"
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
            @update:model-value="val => appStore.setTokenBudget(Number(val))"
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
