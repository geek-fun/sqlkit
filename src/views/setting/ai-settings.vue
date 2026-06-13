<script setup lang="ts">
import type { LlmProvider } from '@/store/appStore'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import FeatureRouting from './feature-routing.vue'
import ProviderFormDialog from './provider-form-dialog.vue'
import ProviderList from './provider-list.vue'

const { t } = useI18n()

const showFormDialog = ref(false)
const editingProvider = ref<LlmProvider | undefined>(undefined)

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

      <div class="border-t" />

      <FeatureRouting />

      <ProviderFormDialog
        :open="showFormDialog"
        :provider="editingProvider ?? null"
        @close="handleCloseDialog"
      />
    </CardContent>
  </Card>
</template>
