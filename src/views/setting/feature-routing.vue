<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useAppStore } from '@/store/appStore'

type FeatureInfo = {
  key: string
  name: string
  description: string
}

const appStore = useAppStore()
const { t } = useI18n()

const features: FeatureInfo[] = [
  {
    key: 'dataStudio',
    name: t('pages.settings.ai.featureRouting.dataStudio.name'),
    description: t('pages.settings.ai.featureRouting.dataStudio.description'),
  },
  {
    key: 'sidebarAssistant',
    name: t('pages.settings.ai.featureRouting.sidebarAssistant.name'),
    description: t('pages.settings.ai.featureRouting.sidebarAssistant.description'),
  },
]

const modelGroups = computed(() =>
  appStore.llmSettings.providers
    .filter(p => p.enabled && p.models?.length)
    .map(p => ({ providerName: p.name, models: p.models! })),
)

const hasModels = computed(() => modelGroups.value.length > 0)

function getRoute(featureKey: string) {
  return appStore.featureModelRoutes[featureKey] ?? { selectedModelId: '', useRecommendedModel: true }
}

function getSelectedModel(featureKey: string) {
  return getRoute(featureKey).selectedModelId || ''
}

function updateRoute(featureKey: string, modelId: string) {
  appStore.setFeatureModelRoute(featureKey, { selectedModelId: modelId, useRecommendedModel: false })
}

function updateRecommended(featureKey: string, useRec: boolean) {
  const current = getRoute(featureKey)
  appStore.setFeatureModelRoute(featureKey, { ...current, useRecommendedModel: useRec })
}
</script>

<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="space-y-1">
      <h3 class="text-lg font-semibold">
        {{ t('pages.settings.ai.featureRouting.title') }}
      </h3>
      <p class="text-sm text-muted-foreground">
        {{ t('pages.settings.ai.featureRouting.description') }}
      </p>
    </div>

    <!-- No models state -->
    <div v-if="!hasModels" class="text-sm text-muted-foreground p-6 text-center border rounded-lg border-dashed">
      <p>{{ t('pages.settings.ai.featureRouting.noModels') }}</p>
      <p class="mt-1">
        {{ t('pages.settings.ai.featureRouting.noModelsHint') }}
      </p>
    </div>

    <!-- Feature rows -->
    <div v-else class="space-y-4">
      <div
        v-for="feature in features"
        :key="feature.key"
        class="p-4 border rounded-lg flex flex-col gap-3"
      >
        <div class="space-y-1">
          <Label>{{ feature.name }}</Label>
          <p class="text-xs text-muted-foreground">
            {{ feature.description }}
          </p>
        </div>

        <div class="flex gap-3 items-center">
          <!-- Model selector -->
          <Select
            :model-value="getSelectedModel(feature.key)"
            @update:model-value="(val: string) => updateRoute(feature.key, val)"
          >
            <SelectTrigger
              :disabled="getRoute(feature.key)?.useRecommendedModel"
              class="w-[280px]"
            >
              <SelectValue :placeholder="t('pages.settings.ai.featureRouting.selectModel')" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup
                v-for="(group, gIdx) in modelGroups"
                :key="gIdx"
              >
                <SelectLabel>{{ group.providerName }}</SelectLabel>
                <SelectItem
                  v-for="m in group.models"
                  :key="m"
                  :value="m"
                >
                  {{ m }}
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>

          <!-- Use recommended checkbox -->
          <div class="flex gap-2 items-center">
            <Checkbox
              :id="`rec-${feature.key}`"
              :checked="getRoute(feature.key)?.useRecommendedModel ?? true"
              @update:checked="(val: boolean | string) => updateRecommended(feature.key, !!val)"
            />
            <Label
              :for="`rec-${feature.key}`"
              class="text-sm cursor-pointer"
            >
              {{ t('pages.settings.ai.featureRouting.useRecommended') }}
            </Label>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
