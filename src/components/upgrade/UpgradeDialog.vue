<script setup lang="ts">
import type { PaidFeature } from '../../common'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Lock, RefreshCw } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { UPGRADE_URL } from '../../common'
import { useEntitlementStore } from '../../store'
import { registerUpgradeDialog } from './upgradeDialogService'

const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const { view } = storeToRefs(entitlementStore)

const showModal = ref(false)
const refreshing = ref(false)
const feature = ref<PaidFeature | undefined>(undefined)

const versionLockedPermanently = computed(() => view.value?.versionLocked ?? false)

function show(paidFeature?: PaidFeature) {
  feature.value = paidFeature
  showModal.value = true
}

function hide() {
  showModal.value = false
}

function handleClose(openState: boolean) {
  if (!openState) {
    hide()
  }
}

async function handleUpgrade() {
  await openUrl(UPGRADE_URL)
}

async function handleRefresh() {
  refreshing.value = true
  try {
    await entitlementStore.refreshEntitlement(true)
    if (entitlementStore.isLocalUltimate) {
      hide()
    }
  }
  finally {
    refreshing.value = false
  }
}

onMounted(() => {
  registerUpgradeDialog(show)
})

onUnmounted(() => {
  registerUpgradeDialog(null)
})
</script>

<template>
  <Dialog :open="showModal" @update:open="handleClose">
    <DialogContent class="max-w-[400px]">
      <div class="text-center flex flex-col gap-3 items-center">
        <div class="rounded-full bg-accent flex h-12 w-12 items-center justify-center">
          <Lock class="text-primary h-6 w-6" />
        </div>
        <h2 class="text-lg font-semibold">
          {{ t('plan.upgrade.title') }}
        </h2>
        <p class="text-sm text-muted-foreground">
          {{
            feature ? t(`plan.features.${feature}`) : t('plan.upgrade.description')
          }}
        </p>
        <Badge :variant="versionLockedPermanently ? 'secondary' : 'outline'">
          {{
            versionLockedPermanently
              ? t('plan.upgrade.versionPermanent')
              : t('plan.upgrade.versionLockedOut')
          }}
        </Badge>
        <div class="mt-2 flex gap-2">
          <Button variant="outline" size="sm" :disabled="refreshing" @click="handleRefresh">
            <RefreshCw v-if="refreshing" class="mr-2 h-4 w-4 animate-spin" />
            {{ t('plan.upgrade.refresh') }}
          </Button>
          <Button size="sm" @click="handleUpgrade">
            {{ t('plan.upgrade.cta') }}
          </Button>
        </div>
        <p class="text-xs text-muted-foreground">
          {{ t('plan.pricing') }}
        </p>
      </div>
    </DialogContent>
  </Dialog>
</template>
