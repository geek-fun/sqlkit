<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { Check, Loader2, LogOut, RefreshCw, X } from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { openUpgradeDialog } from '@/components/upgrade'
import { useAccountStore } from '@/store/accountStore'
import { useDeviceStore } from '@/store/deviceStore'
import { useEntitlementStore } from '@/store/entitlementStore'
import { openLoginUrl, openRegisterUrl } from '@/utils/authService'

const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const accountStore = useAccountStore()
const deviceStore = useDeviceStore()
const refreshing = ref(false)

onMounted(() => {
  if (accountStore.isLoggedIn)
    entitlementStore.refreshEntitlement(false)
})

const versionStateText = computed(() => {
  const release = entitlementStore.view?.appReleaseDate
  if (entitlementStore.isLocalUltimate) {
    return entitlementStore.view?.versionLocked
      ? t('plan.section.versionPermanent')
      : t('plan.section.subscriptionActive')
  }
  // Unknown (no answer yet) or failed check: say nothing rather than render a
  // sentence with an empty date placeholder that asserts a server-side fact.
  if (entitlementStore.view === null || entitlementStore.hasEntitlementError)
    return ''
  return t('plan.section.versionLockedOut', { date: release ?? '' })
})

const expiryText = computed(() => {
  const expiresAt = entitlementStore.view?.ultimateExpiresAt
  if (!expiresAt || !entitlementStore.isCloudUltimate)
    return ''
  return t('plan.section.expiresAt', { time: new Date(expiresAt).toLocaleString() })
})

const compareRows = [
  { key: 'plan.compare.ai' },
  { key: 'plan.compare.er_diagram' },
  { key: 'plan.compare.transfer' },
  { key: 'plan.compare.ssh' },
  { key: 'plan.compare.mcp' },
  { key: 'plan.compare.versionLock' },
]

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

async function handleStartFree() {
  await openRegisterUrl()
}

// Entitlements are account-scoped: the cached view and the device lease must
// never outlive the account session on this machine. Server-side revocation
// is best-effort — the local session clears even when the network or an
// older backend says no.
async function handleLogout() {
  await invoke('revoke_session', { refreshToken: accountStore.refreshToken || null }).catch(() => {})
  await entitlementStore.clearCachedEntitlement()
  accountStore.clearAuth()
  deviceStore.$reset()
}
</script>

<template>
  <Card>
    <CardContent class="px-5 py-4 space-y-4">
      <div class="flex flex-wrap gap-3 items-center">
        <Badge :variant="entitlementStore.isLocalUltimate ? 'default' : 'secondary'">
          <Loader2 v-if="entitlementStore.planState === 'checking'" class="mr-1 h-3 w-3 animate-spin" />
          {{ t(`plan.state.${entitlementStore.planState}`) }}
        </Badge>
        <span v-if="accountStore.isLoggedIn" class="text-sm text-muted-foreground">
          {{ accountStore.email || accountStore.username }}
        </span>
        <span class="text-xs text-muted-foreground">
          {{ versionStateText }}
        </span>
        <span v-if="expiryText" class="text-xs text-muted-foreground">
          {{ expiryText }}
        </span>
        <span v-if="entitlementStore.cancelScheduled" class="text-xs text-amber-600">
          {{ t('plan.section.cancelScheduled') }}
        </span>
        <span v-if="entitlementStore.sessionExpired" class="text-xs text-amber-600">
          {{ t('plan.section.sessionExpired') }}
        </span>
        <span
          v-else-if="accountStore.isLoggedIn && !accountStore.refreshToken && deviceStore.activationError"
          class="text-xs text-amber-600"
        >
          {{ t('plan.section.deviceActivationFailed') }}
        </span>
        <span
          v-else-if="entitlementStore.hasEntitlementError && accountStore.isLoggedIn"
          class="text-xs text-destructive"
        >
          {{ t('plan.section.checkFailed') }}
        </span>
        <span v-if="!accountStore.isLoggedIn" class="text-xs text-muted-foreground">
          {{ t('plan.section.notLoggedIn') }}
          <button class="text-primary underline cursor-pointer hover:opacity-80" @click="handleLogin">
            {{ t('plan.section.loginLink') }}
          </button>
        </span>
        <div class="ml-auto flex gap-2 items-center">
          <Button
            v-if="entitlementStore.sessionExpired"
            size="sm"
            @click="handleLogin"
          >
            {{ t('plan.section.loginLink') }}
          </Button>
          <Button
            v-else-if="accountStore.isLoggedIn"
            variant="outline"
            size="sm"
            :disabled="refreshing"
            @click="handleRefresh"
          >
            <RefreshCw v-if="refreshing" class="mr-2 h-4 w-4 animate-spin" />
            {{ t('plan.section.refresh') }}
          </Button>
          <Button
            v-if="accountStore.isLoggedIn && !entitlementStore.isLocalUltimate"
            size="sm"
            @click="openUpgradeDialog()"
          >
            {{ t('plan.upgrade.cta') }}
          </Button>
          <template v-if="!accountStore.isLoggedIn">
            <Button variant="outline" size="sm" @click="handleLogin">
              Log in
            </Button>
            <Button size="sm" @click="handleStartFree">
              Register
            </Button>
          </template>
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

      <div class="compare-wrap">
        <div class="compare-grid">
          <div class="compare-head compare-cell">
            {{ t('plan.gate.additive') }}
          </div>
          <div class="compare-head compare-cell compare-cell--plan">
            {{ t('plan.state.community') }}
          </div>
          <div class="compare-head compare-cell compare-cell--plan compare-cell--ultimate compare-cell--stack">
            <span>{{ t('plan.state.ultimate') }}</span>
            <span class="compare-recommend">{{ t('plan.gate.recommended') }}</span>
          </div>
          <template v-for="row in compareRows" :key="row.key">
            <div class="compare-cell compare-label">
              {{ t(row.key) }}
            </div>
            <div class="compare-cell compare-cell--plan">
              <X class="compare-no h-3.5 w-3.5" />
            </div>
            <div class="compare-cell compare-cell--plan compare-cell--ultimate">
              <Check class="compare-yes h-3.5 w-3.5" />
            </div>
          </template>
        </div>
      </div>
    </CardContent>
  </Card>
</template>

<style scoped>
.compare-wrap {
  margin: 0 -20px -20px;
  overflow: hidden;
}

.compare-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 84px 108px;
}

.compare-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 9px 14px;
  font-size: 13px;
  color: hsl(var(--foreground));
  border-top: 1px solid hsl(var(--border) / 0.7);
}

.compare-head {
  border-top: none;
  font-size: 12px;
  font-weight: 600;
  color: hsl(var(--muted-foreground));
}

.compare-head.compare-cell:first-child {
  font-weight: 500;
}

.compare-cell--plan {
  justify-content: center;
}

.compare-cell--ultimate {
  background-color: hsl(var(--primary) / 0.05);
}

.compare-cell--stack {
  flex-direction: column;
  justify-content: center;
  gap: 3px;
}

.compare-recommend {
  padding: 1px 8px;
  border-radius: 999px;
  background-color: hsl(var(--primary) / 0.12);
  color: hsl(var(--primary));
  font-size: 9px;
  font-weight: 700;
  white-space: nowrap;
}

.compare-yes {
  color: hsl(var(--primary));
}

.compare-no {
  color: hsl(var(--muted-foreground) / 0.55);
}
</style>
