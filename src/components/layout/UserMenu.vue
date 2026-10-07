<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Loader2, LogOut, RefreshCw, Settings, UserRound } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { useAccountStore } from '@/store/accountStore'
import { useEntitlementStore } from '@/store/entitlementStore'
import { GEEKFUN_BASE_URL as CONSOLE_BASE_URL, openLoginUrl, openRegisterUrl } from '@/utils/authService'

const { t } = useI18n()
const accountStore = useAccountStore()
const entitlementStore = useEntitlementStore()
const refreshing = ref(false)

const planBadge = computed(() => {
  const s = entitlementStore.planState
  if (s === 'ultimate')
    return { label: t('plan.state.ultimate'), cls: 'bg-primary/15 text-primary border-primary/30' }
  if (s === 'community')
    return { label: t('plan.state.community'), cls: 'bg-muted text-muted-foreground border-transparent' }
  if (s === 'checking')
    return { label: t('plan.state.checking'), cls: 'bg-muted text-muted-foreground border-transparent' }
  return { label: t('plan.state.unknown'), cls: 'bg-muted text-muted-foreground border-transparent' }
})

const initials = computed(() => {
  const n = accountStore.username || accountStore.email
  return n ? n.slice(0, 2).toUpperCase() : ''
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

// Entitlements are account-scoped: the cached view must never outlive the
// account session on this machine. Server-side revocation is best-effort —
// the local session clears even when the network or an older backend says no.
async function handleLogout() {
  await invoke('revoke_session', { refreshToken: accountStore.refreshToken || null }).catch(() => {})
  await entitlementStore.clearCachedEntitlement()
  accountStore.clearAuth()
}
</script>

<template>
  <DropdownMenu>
    <DropdownMenuTrigger as-child>
      <button
        class="mx-auto rounded-md flex h-8 w-8 cursor-pointer transition-transform items-center justify-center relative hover:scale-105"
        :title="accountStore.isLoggedIn ? accountStore.username || accountStore.email : t('plan.section.loginLink')"
      >
        <span
          v-if="accountStore.isLoggedIn"
          class="text-lg text-primary-foreground font-bold rounded-md bg-primary flex h-8 w-8 items-center justify-center overflow-hidden"
        >
          <img
            v-if="accountStore.avatar"
            :src="accountStore.avatar"
            alt=""
            class="h-full w-full object-cover"
          >
          <template v-else>
            {{ initials || 'U' }}
          </template>
        </span>
        <span
          v-else
          class="text-muted-foreground border border-border/60 bg-muted/40 flex h-8 w-8 items-center justify-center"
        >
          <UserRound class="h-4 w-4" />
        </span>
        <span
          v-if="!accountStore.isLoggedIn"
          class="rounded-full bg-destructive h-2 w-2 ring-2 ring-background bottom-0.5 right-0.5 absolute"
          :title="t('plan.section.notLoggedIn')"
        />
      </button>
    </DropdownMenuTrigger>
    <DropdownMenuContent side="right" align="start" class="p-2 w-64">
      <template v-if="accountStore.isLoggedIn">
        <div class="p-2">
          <p class="text-sm text-foreground font-semibold truncate">
            {{ accountStore.username || accountStore.email }}
          </p>
          <p class="text-xs text-muted-foreground truncate">
            {{ accountStore.email }}
          </p>
          <span
            class="text-[10px] font-semibold mt-1.5 px-2 py-0.5 rounded-full inline-flex items-center"
            :class="planBadge.cls"
          >
            <Loader2
              v-if="entitlementStore.planState === 'checking'"
              class="mr-1 h-2.5 w-2.5 animate-spin"
            />
            {{ planBadge.label }}
          </span>
          <p v-if="expiryText" class="text-[11px] text-muted-foreground mt-1">
            {{ expiryText }}
          </p>
          <p v-if="entitlementStore.sessionExpired" class="text-[11px] text-amber-500 mt-1">
            {{ t('plan.section.sessionExpired') }}
          </p>
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          v-if="entitlementStore.sessionExpired"
          @click="openLoginUrl()"
        >
          {{ t('plan.section.loginLink') }}
        </DropdownMenuItem>
        <DropdownMenuItem v-else @click="openUrl(`${CONSOLE_BASE_URL}/subscribe`)">
          {{ t('plan.tab') }}
        </DropdownMenuItem>
        <DropdownMenuItem v-if="!entitlementStore.sessionExpired" :disabled="refreshing" @click="handleRefresh">
          <RefreshCw class="mr-2 h-3.5 w-3.5" :class="{ 'animate-spin': refreshing }" />
          {{ t('plan.section.refresh') }}
        </DropdownMenuItem>
        <DropdownMenuItem @click="openUrl(`${CONSOLE_BASE_URL}`)">
          <Settings class="mr-2 h-3.5 w-3.5" />
          {{ t('plan.section.manage') }}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem class="text-destructive focus:text-destructive" @click="handleLogout">
          <LogOut class="mr-2 h-3.5 w-3.5" />
          {{ t('plan.section.logout') }}
        </DropdownMenuItem>
      </template>
      <template v-else>
        <div class="p-2">
          <p class="text-sm text-foreground font-semibold">
            {{ t('plan.section.notLoggedIn') }}
          </p>
          <p class="text-[11px] text-muted-foreground mt-1">
            {{ t('plan.pricing') }}
          </p>
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem @click="openLoginUrl()">
          Log in
        </DropdownMenuItem>
        <DropdownMenuItem @click="openRegisterUrl()">
          Register
        </DropdownMenuItem>
      </template>
    </DropdownMenuContent>
  </DropdownMenu>
</template>
