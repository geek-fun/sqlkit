<script setup lang="ts">
import type { SSHTunnelConfig } from '@/store/connectionStore'
import type { SshProfile } from '@/store/sshProfileStore'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Pencil, Plus } from 'lucide-vue-next'
import Sortable from 'sortablejs'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { UPGRADE_URL } from '@/common'
import SshProfileDialog from '@/components/connections/SshProfileDialog.vue'
import { Button } from '@/components/ui/button'
import { SearchableSelect } from '@/components/ui/combobox'
import { ProBadge } from '@/components/upgrade'
import { useEntitlementStore } from '@/store/entitlementStore'
import { emptySshProfile, profileToTransportLayer, runSshTest, useSshProfileStore } from '@/store/sshProfileStore'
import SshTunnelDetailsDialog from './SshTunnelDetailsDialog.vue'

const props = defineProps<{
  modelValue?: SSHTunnelConfig
  remoteHost?: string
  remotePort?: number
}>()

const emit = defineEmits<{
  'update:modelValue': [value: SSHTunnelConfig | undefined]
}>()

const openUpgrade = () => {
  void openUrl(UPGRADE_URL)
}

const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const profileStore = useSshProfileStore()

const enabled = computed(() => props.modelValue?.enabled ?? false)
const isGated = computed(() => !entitlementStore.isLocalUltimate)
const detectedProxy = ref<string | null>(null)
const legacyDialogRef = ref<InstanceType<typeof SshTunnelDetailsDialog> | null>(null)
const converting = ref(false)
const conversionError = ref('')
const profileDialogRef = ref<InstanceType<typeof SshProfileDialog> | null>(null)
const testing = ref(false)
const testResult = ref<TestSshResult | null>(null)

const hopProfiles = computed(() =>
  (props.modelValue?.profileIds ?? [])
    .map(id => profileStore.byId(id))
    .filter((p): p is SshProfile => !!p),
)

const unusedProfiles = computed(() =>
  profileStore.profiles.filter(p => !(props.modelValue?.profileIds ?? []).includes(p.id)),
)

const useSystemProxy = computed(() => props.modelValue?.useSystemProxy ?? false)

const legacyInline = computed(() =>
  enabled.value
  && (props.modelValue?.profileIds?.length ?? 0) === 0
  && Boolean(props.modelValue?.host?.trim() || props.modelValue?.username?.trim() || props.modelValue?.privateKey?.trim()),
)

/** Chain preview: proxy → hop hosts → final target, breadcrumb style. */
const chainSegments = computed(() => {
  if (!enabled.value)
    return []
  const segments: string[] = []
  if (useSystemProxy.value && detectedProxy.value)
    segments.push(t('components.serverForm.ssh.systemProxy'))
  if (legacyInline.value && props.modelValue?.host)
    segments.push(`${props.modelValue.host}:${props.modelValue?.port ?? 22}`)
  for (const hop of hopProfiles.value)
    segments.push(`${hop.host}:${hop.port}`)
  if (props.remoteHost)
    segments.push(`${props.remoteHost}:${props.remotePort ?? 22}`)
  return segments
})

function patch(value: Partial<SSHTunnelConfig>) {
  const base: SSHTunnelConfig = {
    enabled: false,
    host: '',
    port: 22,
    username: '',
    authMethod: 'password',
    password: '',
    privateKey: '',
    privateKeyPassphrase: '',
    profileIds: [],
    useSystemProxy: false,
    ...props.modelValue,
    ...value,
  }
  // Configure-as-intent: any hop, an attached system proxy, or inline fields
  // means enabled; an empty chain without inline fields means off — the
  // explicit "Use SSH Tunnel" toggle is gone. Detached hops are dormant
  // references and never count as intent.
  base.enabled
    = (base.profileIds?.length ?? 0) > 0
      || base.useSystemProxy === true
      || Boolean(base.host?.trim() || base.username?.trim() || base.privateKey?.trim())
  emit('update:modelValue', base)
}

function addHop(id: string) {
  if (!id || legacyInline.value)
    return
  patch({ profileIds: [...(props.modelValue?.profileIds ?? []), id] })
}

/**
 * Detach just takes it off the chain: the row disappears from the list, the
 *  profile stays in the global store and can be re-added via the selector
 *  anytime — the profile itself is never deleted.
 */
const detachHop = (index: number) => {
  patch({ profileIds: (props.modelValue?.profileIds ?? []).filter((_, i) => i !== index) })
}

const moveHop = (from: number, to: number) => {
  const ids = props.modelValue?.profileIds ?? []
  if (from < 0 || to < 0 || from >= ids.length || to >= ids.length || from === to)
    return
  const remaining = ids.filter((_, index) => index !== from)
  patch({ profileIds: [...remaining.slice(0, to), ids[from]!, ...remaining.slice(to)] })
}

const hopListRef = ref<HTMLElement | null>(null)
let sortable: Sortable | null = null

function initSortable() {
  if (!hopListRef.value || sortable)
    return
  sortable = Sortable.create(hopListRef.value, {
    animation: 150,
    handle: '.ssh-hop-handle',
    onEnd: (evt) => {
      const { oldIndex, newIndex } = evt
      if (oldIndex === undefined || newIndex === undefined || oldIndex === newIndex)
        return
      const parent = evt.item.parentNode
      if (parent) {
        parent.removeChild(evt.item)
        parent.insertBefore(evt.item, parent.children[oldIndex] ?? null)
      }
      moveHop(oldIndex, newIndex)
    },
  })
}

function destroySortable() {
  sortable?.destroy()
  sortable = null
}

watch(hopListRef, () => {
  destroySortable()
  initSortable()
}, { flush: 'post' })

onBeforeUnmount(destroySortable)

function openNewProfile() {
  if (legacyInline.value)
    return
  profileDialogRef.value?.show()
}

function openEditProfile(profile: SshProfile) {
  profileDialogRef.value?.show(profile)
}

function onProfileSaved(profile: SshProfile) {
  // Profile edits (opened from a hop row) are already part of the chain —
  // only newly created profiles get appended. Adding a hop is intent enough
  // to enable the tunnel, so no enabled-gate here.
  if (!(props.modelValue?.profileIds ?? []).includes(profile.id))
    addHop(profile.id)
}

const openLegacyDetails = () => {
  if (props.modelValue)
    legacyDialogRef.value?.show(props.modelValue)
}

const detachLegacy = () => patch({ host: '', username: '', privateKey: '', password: '', privateKeyPassphrase: '' })

const convertLegacyToProfile = async () => {
  const config = props.modelValue
  if (!config || !legacyInline.value || converting.value)
    return
  converting.value = true
  conversionError.value = ''
  try {
    const saved = await profileStore.save({
      ...emptySshProfile(),
      name: t('components.serverForm.ssh.convertedProfileName', { host: config.host || 'SSH' }),
      host: config.host || '',
      port: config.port ?? 22,
      username: config.username || '',
      authMethod: config.authMethod === 'privateKey' ? 'key' : (config.authMethod || 'password'),
      password: config.password || '',
      keyPath: config.privateKey || '',
      keyPassphrase: config.privateKeyPassphrase || '',
      useSshAgent: config.authMethod === 'agent',
    })
    patch({ profileIds: [saved.id], host: '', username: '', privateKey: '', password: '', privateKeyPassphrase: '' })
  }
  catch (error) {
    conversionError.value = String(error)
  }
  finally {
    converting.value = false
  }
}

type TestSshResult = {
  success: boolean
  message: string
  /** Structured success actors from the backend — used to render localized copy */
  user?: string
  host?: string
  port?: number
}

async function testFirstHop() {
  const first = hopProfiles.value[0]
  if (!first || testing.value)
    return
  testing.value = true
  testResult.value = null
  try {
    // Shared profile → Rust-contract layer mapping (camelCase), so the test
    // exercises EXACTLY the credentials the real chain will use.
    testResult.value = await runSshTest(
      profileToTransportLayer(first),
      props.remoteHost ?? '',
      props.remotePort ?? 22,
    )
  }
  catch (err) {
    testResult.value = {
      success: false,
      message: String(err).includes('ssh-test-timeout')
        ? t('components.serverForm.ssh.testTimedOut')
        : String(err),
    }
  }
  finally {
    testing.value = false
  }
}

/**
 * Probe the OS for ANY system proxy (DockIt parity): the row is an "add the
 * system proxy" affordance, so the probe target must not be the connect
 * destination — a DB host inside the proxy exception list (localhost, LAN) or
 * an empty form host would otherwise hide a proxy that DockIt reports.
 * Host-scoped resolution happens at tunnel start, plus the connect-time
 * "enabled but not detected" warning against the first hop.
 */
async function detectProxy() {
  try {
    detectedProxy.value = await invoke<string | null>('detect_system_proxy')
  }
  catch {
    detectedProxy.value = null
  }
}

profileStore.fetch()

// Probe ONCE per panel instance. The probe asks the OS for ANY proxy — its
// result does not depend on the connection being edited, so there is nothing
// to re-detect later. Under configure-as-intent there is no enable toggle:
// the proxy row must be discoverable from the empty state on.
onMounted(detectProxy)
</script>

<template>
  <div class="space-y-4">
    <!-- Gated: the strip replaces the whole SSH config, covering pre-existing connections too. -->
    <button
      v-if="isGated"
      type="button"
      class="group px-3 py-2.5 text-left border rounded-lg border-dashed flex gap-2.5 w-full cursor-pointer transition-colors items-center hover:bg-accent/50"
      @click="openUpgrade"
    >
      <ProBadge size="xs" />
      <span class="text-sm text-muted-foreground truncate transition-colors group-hover:text-foreground/80">
        {{ t('components.serverForm.ssh.ultimateOnly') }}
      </span>
      <span class="text-xs text-primary-text font-medium ml-auto px-2 py-1 rounded-md bg-primary/10 shrink-0 transition-colors group-hover:bg-primary/15">
        {{ t('plan.upgrade.cta') }}
      </span>
    </button>

    <template v-else>
      <div class="flex gap-2 items-center">
        <span class="i-carbon-locked text-green-500 shrink-0 h-4 w-4" />
        <span class="text-sm font-medium whitespace-nowrap">{{ t('components.serverForm.ssh.useSshTunnel') }}</span>
        <span v-if="hopProfiles.length > 0" class="text-xs text-muted-foreground whitespace-nowrap">
          {{ hopProfiles.length }} {{ t('components.serverForm.ssh.hops', hopProfiles.length) }}
        </span>

        <Button
          v-if="hopProfiles.length > 0"
          variant="ghost"
          size="icon"
          type="button"
          class="shrink-0 h-6 w-6"
          :disabled="testing"
          :title="hopProfiles.length > 1
            ? t('components.serverForm.ssh.testFirstHop')
            : t('components.serverForm.ssh.testChain')"
          @click="testFirstHop"
        >
          <span v-if="testing" class="i-carbon-circle-dash text-blue-500 h-3.5 w-3.5 animate-spin" />
          <span v-else class="i-carbon-network-3 h-3.5 w-3.5" />
        </Button>

        <div class="ml-auto max-w-[320px] min-w-0 w-[280px]">
          <SearchableSelect
            model-value=""
            :disabled="legacyInline"
            :options="unusedProfiles.map(p => ({ label: `${p.name} (${p.username}@${p.host})`, value: p.id }))"
            :search-threshold="0"
            :placeholder="t('components.serverForm.ssh.addProfilePlaceholder')"
            :search-placeholder="t('components.serverForm.ssh.searchProfilePlaceholder')"
            @update:model-value="(id: unknown) => addHop(String(id))"
          />
        </div>
        <Button
          variant="outline"
          size="icon"
          type="button"
          class="shrink-0 h-7 w-7"
          :title="t('components.serverForm.ssh.newProfile')"
          :disabled="legacyInline"
          @click="openNewProfile"
        >
          <Plus class="h-4 w-4" />
        </Button>
      </div>

      <!-- Success renders localized from structured actors; failures keep the
           backend diagnostic and wrap — details must stay readable. -->
      <p
        v-if="testResult"
        class="text-xs leading-relaxed break-words"
        :class="testResult.success ? 'text-green-600 dark:text-green-400' : 'text-destructive'"
      >
        {{ testResult.success && testResult.user && testResult.host && testResult.port
          ? t('components.serverForm.ssh.testSuccess', {
            user: testResult.user,
            host: testResult.host,
            port: testResult.port,
          })
          : testResult.message }}
      </p>

      <!-- Empty state: suppressed while a proxy-only chain is active — that is
           a configured state, not an unscaffolded one. -->
      <div
        v-if="hopProfiles.length === 0 && !legacyInline && !useSystemProxy"
        class="text-xs text-muted-foreground py-6 text-center border rounded-md border-dashed"
      >
        {{ t('components.serverForm.ssh.notConfigured') }}
      </div>

      <!-- System proxy: OS-detected up front (DockIt parity). Attached → pinned
           as the implicit hop 0 (never persisted as a URL); detected but not
           attached → an Add affordance. Visible from the empty state on:
           attaching the proxy alone is enough intent to enable the tunnel. -->
      <div
        v-if="detectedProxy && useSystemProxy"
        class="px-2.5 py-1.5 border border-primary/40 rounded-md bg-primary/5 flex gap-2 items-center"
      >
        <span class="text-caption text-muted-foreground font-bold shrink-0 w-4">0</span>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium">
            {{ t('components.serverForm.ssh.systemProxy') }}
          </div>
          <div class="text-caption text-muted-foreground font-mono truncate">
            {{ detectedProxy }}
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          class="text-muted-foreground shrink-0 h-6 w-6 hover:text-destructive-text"
          :title="t('components.serverForm.ssh.systemProxyDetach')"
          @click="patch({ useSystemProxy: false })"
        >
          <span class="i-carbon-unlink h-3 w-3" />
        </Button>
      </div>
      <!-- Persisted-but-undetected: the setting pools invisibly otherwise —
           show it ON so the user can turn it off in the form (not only at
           connect time). -->
      <div
        v-else-if="!detectedProxy && useSystemProxy"
        class="px-2.5 py-1.5 border border-amber-500/50 rounded-md bg-amber-50 flex gap-2 items-center dark:bg-amber-900/20"
      >
        <span class="text-caption text-muted-foreground font-bold shrink-0 w-4">0</span>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium">
            {{ t('components.serverForm.ssh.systemProxy') }}
          </div>
          <div class="text-caption text-amber-600 truncate dark:text-amber-400">
            {{ t('components.serverForm.ssh.systemProxyNotDetected') }}
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          class="text-muted-foreground shrink-0 h-6 w-6 hover:text-destructive-text"
          :title="t('components.serverForm.ssh.systemProxyDetach')"
          @click="patch({ useSystemProxy: false })"
        >
          <span class="i-carbon-unlink h-3 w-3" />
        </Button>
      </div>

      <div
        v-else-if="detectedProxy"
        class="px-2.5 py-1.5 border rounded-md flex gap-2 items-center"
      >
        <span class="i-carbon-network-public text-muted-foreground shrink-0 h-3.5 w-3.5" />
        <span class="text-xs text-muted-foreground flex-1 min-w-0 truncate">
          {{ t('components.serverForm.ssh.systemProxyDetected', { url: detectedProxy }) }}
        </span>
        <Button
          variant="outline"
          size="sm"
          type="button"
          class="text-xs shrink-0 h-6"
          @click="patch({ useSystemProxy: true })"
        >
          {{ t('components.serverForm.ssh.systemProxyAdd') }}
        </Button>
      </div>

      <div v-if="hopProfiles.length > 0" class="space-y-1.5">
        <div ref="hopListRef" class="space-y-1.5">
          <div
            v-for="(profile, index) in hopProfiles"
            :key="profile.id"
            class="px-2.5 py-1.5 border rounded-md bg-background flex gap-2 items-center"
          >
            <span class="ssh-hop-handle i-carbon-draggable text-muted-foreground shrink-0 h-3.5 w-3.5 cursor-grab" />
            <span class="text-caption text-muted-foreground font-bold w-4">{{ index + 1 }}</span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium truncate">
                {{ profile.name }}
              </div>
              <div class="text-caption text-muted-foreground truncate">
                {{ profile.username }}@{{ profile.host }}:{{ profile.port }}
              </div>
            </div>
            <template v-if="hopProfiles.length > 1">
              <Button variant="ghost" size="icon" type="button" class="shrink-0 h-6 w-6" :disabled="index === 0" :aria-label="t('components.serverForm.ssh.moveUp')" :title="t('components.serverForm.ssh.moveUp')" @click="moveHop(index, index - 1)">
                <span class="i-carbon-chevron-up h-3.5 w-3.5" />
              </Button>
              <Button variant="ghost" size="icon" type="button" class="shrink-0 h-6 w-6" :disabled="index === hopProfiles.length - 1" :aria-label="t('components.serverForm.ssh.moveDown')" :title="t('components.serverForm.ssh.moveDown')" @click="moveHop(index, index + 1)">
                <span class="i-carbon-chevron-down h-3.5 w-3.5" />
              </Button>
            </template>
            <Button variant="ghost" size="icon" type="button" class="h-6 w-6" :title="t('components.serverForm.ssh.editProfile')" @click="openEditProfile(profile)">
              <Pencil class="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              type="button"
              class="text-muted-foreground h-6 w-6 hover:text-destructive-text"
              :title="t('components.serverForm.ssh.detachAction')"
              @click="detachHop(index)"
            >
              <span class="i-carbon-unlink h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>

      <div v-if="legacyInline" class="space-y-2">
        <div class="px-2.5 py-1.5 border rounded-md bg-background flex gap-2 items-center">
          <span class="text-caption text-muted-foreground font-bold shrink-0 w-4">1</span>
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium truncate">
              {{ props.modelValue?.host }}
            </div>
            <div class="text-xs text-muted-foreground truncate">
              {{ props.modelValue?.username }}@{{ props.modelValue?.host }}:{{ props.modelValue?.port ?? 22 }}
            </div>
          </div>
          <Button variant="ghost" size="icon" type="button" class="shrink-0 h-6 w-6" :disabled="converting" :aria-label="t('components.serverForm.ssh.editTunnel')" :title="t('components.serverForm.ssh.editTunnel')" @click="openLegacyDetails">
            <Pencil class="h-3.5 w-3.5" />
          </Button>
          <Button variant="ghost" size="icon" type="button" class="text-muted-foreground shrink-0 h-6 w-6 hover:text-destructive-text" :disabled="converting" :aria-label="t('components.serverForm.ssh.detachAction')" :title="t('components.serverForm.ssh.detachAction')" @click="detachLegacy">
            <span class="i-carbon-unlink h-3.5 w-3.5" />
          </Button>
        </div>
        <div class="flex flex-wrap gap-2 items-center justify-between">
          <p class="text-xs text-muted-foreground flex-1 min-w-48">
            {{ t('components.serverForm.ssh.convertHint') }}
          </p>
          <Button variant="outline" type="button" size="sm" :disabled="converting" @click="convertLegacyToProfile">
            <span v-if="converting" class="i-carbon-circle-dash h-3.5 w-3.5 animate-spin" />
            {{ t('components.serverForm.ssh.convertToProfile') }}
          </Button>
        </div>
        <p v-if="conversionError" role="alert" class="text-sm text-destructive-text break-words">
          {{ conversionError }}
        </p>
      </div>

      <!-- Chain preview: proxy → hops → target -->
      <p v-if="chainSegments.length > 1" class="text-caption text-muted-foreground font-mono break-words">
        {{ chainSegments.join('  →  ') }}
      </p>
      <p v-if="hopProfiles.length > 1" class="text-caption text-muted-foreground">
        {{ t('components.serverForm.ssh.hopOrderHint') }}
      </p>

      <SshTunnelDetailsDialog ref="legacyDialogRef" @save="patch" />

      <SshProfileDialog ref="profileDialogRef" @save="onProfileSaved" />
    </template>
  </div>
</template>
