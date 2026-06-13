<script setup lang="ts">
import type { LlmProvider } from '@/store/appStore'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger } from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Spinner } from '@/components/ui/spinner'
import { useAppStore } from '@/store/appStore'

defineEmits<{
  add: []
  edit: [provider: LlmProvider]
}>()

const appStore = useAppStore()
const { t } = useI18n()

const providers = computed(() => appStore.llmSettings.providers)

const expandedProvider = ref<string | null>(null)
const testingId = ref<string | null>(null)
const testResult = ref<{ id: string, success: boolean } | null>(null)
const syncingId = ref<string | null>(null)

function toggleExpand(id: string) {
  expandedProvider.value = expandedProvider.value === id ? null : id
}

function handleToggleEnabled(id: string) {
  appStore.toggleProviderEnabled(id)
}

async function handleTest(id: string) {
  testingId.value = id
  testResult.value = null
  try {
    const result = await appStore.testProviderConnection(id)
    testResult.value = { id, success: result.success }
  }
  catch {
    testResult.value = { id, success: false }
  }
  finally {
    testingId.value = null
  }
}

async function handleSync(id: string) {
  syncingId.value = id
  try {
    await appStore.syncProviderModels(id)
  }
  catch {
    // silently fail — syncProviderModels already handles errors
  }
  finally {
    syncingId.value = null
  }
}
</script>

<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h3 class="text-lg font-semibold">
          {{ t('pages.settings.ai.providerList.title') }}
        </h3>
        <p class="text-sm text-muted-foreground">
          {{ t('pages.settings.ai.providerList.description') }}
        </p>
      </div>
      <Button @click="$emit('add')">
        {{ t('pages.settings.ai.providerList.addProvider') }}
      </Button>
    </div>

    <!-- Empty state -->
    <div
      v-if="providers.length === 0"
      class="text-muted-foreground py-8 text-center"
    >
      <p>{{ t('pages.settings.ai.providerList.empty') }}</p>
      <Button
        variant="outline"
        class="mt-2"
        @click="$emit('add')"
      >
        {{ t('pages.settings.ai.providerList.emptyAction') }}
      </Button>
    </div>

    <!-- Provider cards -->
    <div v-else class="space-y-2">
      <div
        v-for="provider in providers"
        :key="provider.id"
        class="border rounded-lg overflow-hidden"
      >
        <!-- Collapsed header (clickable) -->
        <div
          class="px-4 py-3 flex cursor-pointer items-center justify-between hover:bg-accent/50"
          @click="toggleExpand(provider.id)"
        >
          <div class="flex gap-2 items-center">
            <div
              :class="provider.enabled ? 'bg-green-500' : 'bg-gray-400'"
              class="rounded-full flex-shrink-0 h-2 w-2"
            />
            <span class="font-medium">{{ provider.name }}</span>
            <Badge variant="secondary">
              {{ provider.apiCompatibility }}
            </Badge>
            <Badge variant="outline">
              {{ t('pages.settings.ai.providerList.modelCount', { count: provider.models?.length || 0 }) }}
            </Badge>
          </div>
          <div class="flex gap-2 items-center">
            <!-- Enable/disable toggle (inline switch) -->
            <button
              role="switch"
              :aria-checked="provider.enabled"
              :class="provider.enabled ? 'bg-primary' : 'bg-muted'"
              class="border-2 border-transparent rounded-full inline-flex flex-shrink-0 h-6 w-11 cursor-pointer transition-colors relative focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              @click.stop="handleToggleEnabled(provider.id)"
            >
              <span
                :class="provider.enabled ? 'translate-x-5' : 'translate-x-0'"
                class="rounded-full bg-background h-5 w-5 inline-block pointer-events-none shadow transition-transform"
              />
            </button>
            <!-- Expand chevron -->
            <svg
              :class="expandedProvider === provider.id ? 'rotate-180' : ''"
              class="text-muted-foreground flex-shrink-0 h-4 w-4 transition-transform"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="6 9 12 15 18 9" />
            </svg>
          </div>
        </div>

        <!-- Expanded body -->
        <div
          v-if="expandedProvider === provider.id"
          class="px-4 py-3 border-t space-y-3"
        >
          <!-- Details -->
          <div class="text-sm gap-2 grid grid-cols-2">
            <div v-if="provider.baseUrl">
              <span class="text-muted-foreground">Base URL:</span>
              <span class="text-xs font-mono ml-1">{{ provider.baseUrl }}</span>
            </div>
            <div v-if="provider.proxy">
              <span class="text-muted-foreground">Proxy:</span>
              <span class="text-xs font-mono ml-1">{{ provider.proxy }}</span>
            </div>
            <div v-if="provider.contextWindowOverride">
              <span class="text-muted-foreground">Context:</span>
              <span class="ml-1">{{ provider.contextWindowOverride }} tokens</span>
            </div>
          </div>

          <!-- Models list -->
          <div
            v-if="provider.models?.length"
            class="flex flex-wrap gap-1"
          >
            <Badge
              v-for="m in provider.models"
              :key="m"
              variant="outline"
              class="text-xs"
            >
              {{ m }}
            </Badge>
          </div>

          <!-- Action buttons -->
          <div class="flex gap-2 items-center">
            <Button
              variant="ghost"
              size="sm"
              :disabled="testingId === provider.id"
              @click="handleTest(provider.id)"
            >
              <Spinner
                v-if="testingId === provider.id"
                size="sm"
                class="mr-1"
              />
              <template v-else-if="testResult?.id === provider.id && testResult.success">
                <svg
                  class="text-green-500 mr-1 h-3 w-3"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </template>
              <template v-else-if="testResult?.id === provider.id && !testResult.success">
                <svg
                  class="text-destructive mr-1 h-3 w-3"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </template>
              {{ testingId === provider.id
                ? t('pages.settings.ai.providerList.testing')
                : testResult?.id === provider.id && testResult.success
                  ? t('pages.settings.ai.providerList.testSuccess')
                  : testResult?.id === provider.id && !testResult.success
                    ? t('pages.settings.ai.providerList.testFailed')
                    : t('pages.settings.ai.providerList.testConnection')
              }}
            </Button>

            <Button
              variant="ghost"
              size="sm"
              :disabled="syncingId === provider.id"
              @click="handleSync(provider.id)"
            >
              <Spinner
                v-if="syncingId === provider.id"
                size="sm"
                class="mr-1"
              />
              {{ syncingId === provider.id
                ? t('pages.settings.ai.providerList.syncing')
                : t('pages.settings.ai.providerList.sync')
              }}
            </Button>

            <Button
              variant="ghost"
              size="sm"
              @click="$emit('edit', provider)"
            >
              {{ t('common.buttons.edit') }}
            </Button>

            <AlertDialog>
              <AlertDialogTrigger as-child>
                <Button
                  variant="ghost"
                  size="sm"
                  class="text-destructive"
                >
                  {{ t('common.buttons.delete') }}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>
                    {{ t('pages.settings.ai.providerList.deleteConfirmTitle') }}
                  </AlertDialogTitle>
                  <AlertDialogDescription>
                    {{ t('pages.settings.ai.providerList.deleteConfirm', { name: provider.name }) }}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>
                    {{ t('common.buttons.cancel') }}
                  </AlertDialogCancel>
                  <AlertDialogAction
                    class="text-destructive-foreground bg-destructive"
                    @click="appStore.removeProvider(provider.id)"
                  >
                    {{ t('common.buttons.delete') }}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
