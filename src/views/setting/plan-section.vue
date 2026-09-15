<script setup lang="ts">
import { RefreshCw } from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { openUpgradeDialog } from '@/components/upgrade'
import { useAccountStore } from '@/store/accountStore'
import { useEntitlementStore } from '@/store/entitlementStore'
import { openLoginUrl } from '@/utils/authService'

const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const accountStore = useAccountStore()
const refreshing = ref(false)

onMounted(() => {
  entitlementStore.refreshEntitlement(false)
})

const versionStateText = computed(() => {
  const release = entitlementStore.view?.appReleaseDate
  if (entitlementStore.isLocalUltimate) {
    return entitlementStore.view?.versionLocked
      ? t('plan.section.versionPermanent')
      : t('plan.section.subscriptionActive')
  }
  return t('plan.section.versionLockedOut').replace('{date}', release ?? '')
})

const expiryText = computed(() => {
  const expiresAt = entitlementStore.view?.ultimateExpiresAt
  if (!expiresAt || !entitlementStore.isCloudUltimate)
    return ''
  return t('plan.section.expiresAt').replace('{time}', new Date(expiresAt).toLocaleString())
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

async function handleLogin() {
  await openLoginUrl()
}
</script>

<template>
  <Card>
    <CardContent class="p-5 space-y-4">
      <div class="flex flex-wrap gap-4 items-center justify-between">
        <div class="space-y-1">
          <div class="flex gap-2 items-center">
            <Badge :variant="entitlementStore.isLocalUltimate ? 'default' : 'secondary'">
              {{ t(`plan.state.${entitlementStore.planState}`) }}
            </Badge>
            <span v-if="accountStore.isLoggedIn" class="text-sm text-muted-foreground">
              {{ accountStore.email || accountStore.username }}
            </span>
          </div>
          <p class="text-xs text-muted-foreground">
            {{ versionStateText }}
          </p>
          <p v-if="expiryText" class="text-xs text-muted-foreground">
            {{ expiryText }}
          </p>
          <p v-if="entitlementStore.cancelScheduled" class="text-xs text-amber-600">
            {{ t('plan.section.cancelScheduled') }}
          </p>
          <p v-if="entitlementStore.hasEntitlementError" class="text-xs text-destructive">
            {{ t('plan.section.checkFailed') }}
          </p>
        </div>
        <div class="flex gap-2 items-center">
          <Button variant="outline" size="sm" :disabled="refreshing" @click="handleRefresh">
            <RefreshCw v-if="refreshing" class="mr-2 h-4 w-4 animate-spin" />
            {{ t('plan.section.refresh') }}
          </Button>
          <Button v-if="!entitlementStore.isLocalUltimate" size="sm" @click="openUpgradeDialog()">
            {{ t('plan.upgrade.cta') }}
          </Button>
        </div>
      </div>
      <p v-if="!accountStore.isLoggedIn" class="text-xs text-muted-foreground">
        {{ t('plan.section.notLoggedIn') }}
        <button class="text-primary underline cursor-pointer hover:opacity-80" @click="handleLogin">
          {{ t('plan.section.loginLink') }}
        </button>
      </p>
    </CardContent>
  </Card>
</template>
