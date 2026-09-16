<script setup lang="ts">
import { LogOut, RefreshCw } from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { openUpgradeDialog } from '@/components/upgrade'
import { useAccountStore } from '@/store/accountStore'
import { useDeviceStore } from '@/store/deviceStore'
import { useEntitlementStore } from '@/store/entitlementStore'
import { openLoginUrl } from '@/utils/authService'

const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const accountStore = useAccountStore()
const deviceStore = useDeviceStore()
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
  return t('plan.section.versionLockedOut', { date: release ?? '' })
})

const expiryText = computed(() => {
  const expiresAt = entitlementStore.view?.ultimateExpiresAt
  if (!expiresAt || !entitlementStore.isCloudUltimate)
    return ''
  return t('plan.section.expiresAt', { time: new Date(expiresAt).toLocaleString() })
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

// Entitlements are account-scoped: the cached view and the device lease must
// never outlive the account session on this machine.
async function handleLogout() {
  await entitlementStore.clearCachedEntitlement()
  accountStore.clearAuth()
  deviceStore.$reset()
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
          <Button
            v-if="accountStore.isLoggedIn"
            variant="ghost"
            size="sm"
            @click="handleLogout"
          >
            <LogOut class="mr-2 h-4 w-4" />
            {{ t('plan.section.logout') }}
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
