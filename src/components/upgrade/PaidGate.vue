<script setup lang="ts">
import type { PaidFeature } from '../../common'
import { Lock, RefreshCw } from 'lucide-vue-next'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { useEntitlementStore } from '../../store'
import { openUpgradeDialog } from './upgradeDialogService'

defineProps<{ feature: PaidFeature }>()

const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const refreshing = ref(false)

onMounted(() => {
  entitlementStore.refreshEntitlement(false)
})

async function handleRefresh() {
  refreshing.value = true
  try {
    await entitlementStore.refreshEntitlement(true)
  }
  finally {
    refreshing.value = false
  }
}
</script>

<template>
  <slot v-if="entitlementStore.isLocalUltimate" />
  <div v-else class="py-16 flex flex-col gap-4 items-center justify-center">
    <div class="rounded-full bg-accent flex h-14 w-14 items-center justify-center">
      <Lock class="text-primary h-6 w-6" />
    </div>
    <div class="text-center space-y-1">
      <h3 class="text-base font-semibold">
        {{ t('plan.upgrade.title') }}
      </h3>
      <p class="text-sm text-muted-foreground max-w-md">
        {{ t(`plan.features.${feature}`) }}
      </p>
      <p class="text-xs text-muted-foreground">
        {{ t('plan.pricing') }}
      </p>
    </div>
    <div class="flex gap-2 items-center">
      <Button variant="outline" size="sm" :disabled="refreshing" @click="handleRefresh">
        <RefreshCw v-if="refreshing" class="mr-2 h-4 w-4 animate-spin" />
        {{ t('plan.upgrade.refresh') }}
      </Button>
      <Button size="sm" @click="openUpgradeDialog(feature)">
        {{ t('plan.upgrade.cta') }}
      </Button>
    </div>
  </div>
</template>
