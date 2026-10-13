<script setup lang="ts">
import type { SSHTunnelConfig } from '@/store/connectionStore'
import type { SshProfile } from '@/store/sshProfileStore'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Pencil, Plus, Trash2 } from 'lucide-vue-next'
import Sortable from 'sortablejs'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { UPGRADE_URL } from '@/common/entitlement'
import SshProfileDialog from '@/components/connections/SshProfileDialog.vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { SearchableSelect } from '@/components/ui/combobox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ProBadge } from '@/components/upgrade'
import { useEntitlementStore } from '@/store/entitlementStore'
import { emptySshProfile, useSshProfileStore } from '@/store/sshProfileStore'

const props = defineProps<{
  modelValue?: SSHTunnelConfig
  remoteHost?: string
  remotePort?: number
}>()

const emit = defineEmits<{
  'update:modelValue': [value: SSHTunnelConfig | undefined]
}>()

const { t } = useI18n()
const entitlementStore = useEntitlementStore()
const profileStore = useSshProfileStore()

const enabled = computed(() => props.modelValue?.enabled ?? false)
const isGated = computed(() => !entitlementStore.isLocalUltimate)
const detectedProxy = ref<string | null>(null)
const configHosts = ref<Array<{ host: string, hostName?: string, port?: number, user?: string, identityFile?: string }>>([])
const showConfigHosts = ref(false)
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

/**
 * Legacy single-tunnel connections (pre-profiles schema): enabled with inline
 * fields and no hop chain. They keep editing their inline fields; new
 * configurations always go through profiles.
 */
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
  // explicit "Use SSH Tunnel" toggle is gone.
  base.enabled
    = (base.profileIds?.length ?? 0) > 0
      || base.useSystemProxy === true
      || Boolean(base.host?.trim() || base.username?.trim() || base.privateKey?.trim())
  emit('update:modelValue', base)
}

function addHop(id: string) {
  if (!id)
    return
  patch({ profileIds: [...(props.modelValue?.profileIds ?? []), id] })
}

function removeHop(index: number) {
  const ids = [...(props.modelValue?.profileIds ?? [])]
  ids.splice(index, 1)
  patch({ profileIds: ids })
}

function moveHop(from: number, to: number) {
  const ids = [...(props.modelValue?.profileIds ?? [])]
  if (from < 0 || to < 0 || from >= ids.length || to >= ids.length || from === to)
    return
  const [moved] = ids.splice(from, 1)
  ids.splice(to, 0, moved as string)
  patch({ profileIds: ids })
}

// ── Drag-to-reorder (SortableJS, DocKit-style drag handle) ──

const hopListRef = ref<HTMLElement | null>(null)
let sortable: Sortable | null = null

function initSortable() {
  if (!hopListRef.value || sortable)
    return
  sortable = Sortable.create(hopListRef.value, {
    animation: 150,
    handle: '.ssh-hop-handle',
    // The reorder is committed through state (patch), never through the DOM
    // mutation Sortable performed — Vue re-renders from profileIds.
    onEnd: (evt) => {
      const { oldIndex, newIndex } = evt
      if (oldIndex === undefined || newIndex === undefined || oldIndex === newIndex)
        return
      moveHop(oldIndex, newIndex)
    },
  })
}

function destroySortable() {
  sortable?.destroy()
  sortable = null
}

watch(() => hopProfiles.value.length, (count, previous) => {
  // The list mounts when the chain goes from empty to populated; re-init then.
  if (count > 0 && !previous)
    requestAnimationFrame(initSortable)
})

onMounted(() => {
  if (hopProfiles.value.length > 0)
    initSortable()
})

onBeforeUnmount(destroySortable)

function openNewProfile() {
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

/** Convert a legacy inline config into a reusable profile and hop onto it. */
async function convertLegacyToProfile() {
  const mv = props.modelValue
  if (!mv)
    return
  const saved = await profileStore.save({
    ...emptySshProfile(),
    name: t('components.serverForm.ssh.convertedProfileName', { host: mv.host }),
    host: mv.host || '',
    port: mv.port ?? 22,
    username: mv.username || '',
    authMethod: mv.authMethod === 'privateKey' ? 'key' : (mv.authMethod || 'password'),
    password: mv.password || '',
    keyPath: mv.privateKey || '',
    keyPassphrase: mv.privateKeyPassphrase || '',
  })
  // Reset inline fields and hop onto the new profile — the chain now owns
  // the credentials and the inline shape stays cleared.
  patch({ profileIds: [saved.id], host: '', username: '', privateKey: '', password: '', privateKeyPassphrase: '' })
}

type TestSshResult = { success: boolean, message: string }

/** Convert a profile into a single-hop tunnel config for the test command. */
const profileToTunnelConfig = (profile: SshProfile): SSHTunnelConfig => ({
  enabled: true,
  host: profile.host,
  port: profile.port,
  username: profile.username,
  authMethod: profile.authMethod === 'key'
    ? 'privateKey'
    : profile.authMethod === 'none'
      ? 'password'
      : (profile.authMethod || 'password') as SSHTunnelConfig['authMethod'],
  password: profile.password,
  privateKey: profile.keyPath,
  privateKeyPassphrase: profile.keyPassphrase,
})

async function testFirstHop() {
  const first = hopProfiles.value[0]
  if (!first || testing.value)
    return
  testing.value = true
  testResult.value = null
  try {
    testResult.value = await invoke<TestSshResult>('test_ssh_connection', {
      config: profileToTunnelConfig(first),
      remoteHost: props.remoteHost ?? '',
      remotePort: props.remotePort ?? 22,
    })
  }
  catch (err) {
    testResult.value = { success: false, message: String(err) }
  }
  finally {
    testing.value = false
  }
}

async function loadConfigHosts() {
  try {
    configHosts.value = await invoke('list_ssh_config_hosts')
    showConfigHosts.value = true
  }
  catch {
    configHosts.value = []
    showConfigHosts.value = true
  }
}

function applyConfigHost(host: { host: string, hostName?: string, port?: number, user?: string, identityFile?: string }) {
  patch({
    enabled: true,
    host: host.hostName || host.host,
    port: host.port ?? 22,
    username: host.user ?? '',
    authMethod: host.identityFile ? 'privateKey' : 'password',
    privateKey: host.identityFile ?? '',
  })
  showConfigHosts.value = false
}

async function detectProxy() {
  try {
    detectedProxy.value = await invoke<string | null>('detect_system_proxy', {
      host: props.remoteHost ?? null,
      port: props.remotePort ? Number(props.remotePort) : null,
    })
  }
  catch {
    detectedProxy.value = null
  }
}

profileStore.fetch()

// Detect on mount — under configure-as-intent there is no enable toggle, so
// the pinned system-proxy row must be discoverable from the empty state on.
onMounted(detectProxy)

// Re-detect when the target host/port changes (proxies can be host-scoped).
watch(() => [props.remoteHost, props.remotePort], () => detectProxy())

// Re-detect when the tunnel first turns on (editing an existing connection).
watch(() => props.modelValue?.enabled, (on) => {
  if (on)
    detectProxy()
}, { immediate: true })
</script>

<template>
  <div class="space-y-4">
    <!-- Gated: the strip replaces the whole SSH config, covering pre-existing connections too.
         Layering the upgrade modal on top of the server form dialog is awkward UX (two stacked
         modals), so this strip routes straight to the pricing site instead. -->
    <button
      v-if="isGated"
      type="button"
      class="group px-3 py-2.5 text-left border rounded-lg border-dashed flex gap-2.5 w-full cursor-pointer transition-colors items-center hover:bg-accent/50"
      @click="openUrl(UPGRADE_URL)"
    >
      <ProBadge size="xs" />
      <span class="text-sm text-muted-foreground truncate transition-colors group-hover:text-foreground/80">
        {{ t('components.serverForm.ssh.ultimateOnly') }}
      </span>
      <span class="text-xs text-primary font-medium ml-auto px-2 py-1 rounded-md bg-primary/10 shrink-0 transition-colors group-hover:bg-primary/15">
        {{ t('plan.upgrade.cta') }}
      </span>
    </button>

    <template v-else>
      <div class="flex gap-2 items-center">
        <span class="i-carbon-locked text-green-500 shrink-0 h-4 w-4" />
        <span class="text-sm font-medium whitespace-nowrap">{{ t('components.serverForm.ssh.useSshTunnel') }}</span>
        <span v-if="hopProfiles.length > 0" class="text-xs text-muted-foreground whitespace-nowrap">
          {{ hopProfiles.length }} {{ t('components.serverForm.ssh.hops') }}
        </span>

        <Button
          v-if="hopProfiles.length > 0"
          variant="ghost"
          size="icon"
          type="button"
          class="shrink-0 h-6 w-6"
          :disabled="testing"
          :title="t('components.serverForm.ssh.testChain')"
          @click="testFirstHop"
        >
          <span v-if="testing" class="i-carbon-circle-dash text-blue-500 h-3.5 w-3.5 animate-spin" />
          <span v-else class="i-carbon-network-3 h-3.5 w-3.5" />
        </Button>

        <div class="ml-auto max-w-[240px] min-w-0">
          <SearchableSelect
            model-value=""
            :options="unusedProfiles.map(p => ({ label: `${p.name} (${p.username}@${p.host})`, value: p.id }))"
            :search-threshold="0"
            :placeholder="t('components.serverForm.ssh.addProfilePlaceholder')"
            @update:model-value="(id: unknown) => addHop(String(id))"
          />
        </div>
        <Button
          variant="outline"
          size="icon"
          type="button"
          class="shrink-0 h-7 w-7"
          :title="t('components.serverForm.ssh.newProfile')"
          @click="openNewProfile"
        >
          <Plus class="h-4 w-4" />
        </Button>
      </div>

      <p
        v-if="testResult"
        class="text-xs truncate"
        :class="testResult.success ? 'text-green-600 dark:text-green-400' : 'text-destructive'"
      >
        {{ testResult.message }}
      </p>

      <!-- Empty state -->
      <div
        v-if="hopProfiles.length === 0 && !legacyInline"
        class="text-xs text-muted-foreground py-6 text-center border rounded-md border-dashed"
      >
        {{ t('components.serverForm.ssh.notConfigured') }}
      </div>

      <!-- System proxy: OS-detected, pinned as the implicit hop 0 (never persisted as a URL) -->
      <div
        v-if="detectedProxy && (hopProfiles.length > 0 || legacyInline)"
        class="px-2.5 py-1.5 border rounded-md flex gap-2 items-center"
        :class="useSystemProxy ? 'border-primary/40 bg-primary/5' : ''"
      >
        <span class="text-[10px] text-muted-foreground font-bold shrink-0 w-4">0</span>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium flex gap-1.5 items-center">
            {{ t('components.serverForm.ssh.systemProxy') }}
            <span
              v-if="useSystemProxy"
              class="text-[10px] text-primary px-1.5 py-0.5 rounded bg-primary/10"
            >
              {{ t('components.serverForm.ssh.proxyActive') }}
            </span>
          </div>
          <div class="text-[11px] text-muted-foreground font-mono truncate">
            {{ detectedProxy }}
          </div>
        </div>
        <Button
          variant="ghost"
          size="sm"
          type="button"
          class="text-xs shrink-0 h-6"
          @click="patch({ useSystemProxy: !useSystemProxy })"
        >
          {{ useSystemProxy ? t('components.serverForm.ssh.detachProxy') : t('components.serverForm.ssh.attachProxy') }}
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
            <span class="text-[10px] text-muted-foreground font-bold w-4">{{ index + 1 }}</span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium truncate">
                {{ profile.name }}
              </div>
              <div class="text-[11px] text-muted-foreground truncate">
                {{ profile.username }}@{{ profile.host }}:{{ profile.port }}
              </div>
            </div>
            <Button variant="ghost" size="icon" type="button" class="h-6 w-6" @click="openEditProfile(profile)">
              <Pencil class="h-3.5 w-3.5" />
            </Button>
            <Button variant="ghost" size="icon" type="button" class="text-destructive h-6 w-6" @click="removeHop(index)">
              <Trash2 class="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>

      <!-- Chain preview: proxy → hops → target -->
      <p v-if="chainSegments.length > 1" class="text-[11px] text-muted-foreground font-mono break-all">
        {{ chainSegments.join('  →  ') }}
      </p>
      <p v-if="hopProfiles.length > 1" class="text-[11px] text-muted-foreground">
        {{ t('components.serverForm.ssh.hopOrderHint') }}
      </p>

      <!-- Legacy manual (inline) config — pre-profiles connections keep editing
         their inline fields; new configurations always go through profiles -->
      <template v-if="legacyInline">
        <p class="text-[11px] text-amber-600 dark:text-amber-400">
          {{ t('components.serverForm.ssh.legacyManual') }}
        </p>
        <div class="space-y-2">
          <Label for="ssh-host">{{ t('components.serverForm.ssh.sshHost') }}</Label>
          <Input id="ssh-host" :model-value="props.modelValue?.host" placeholder="ssh.example.com" @update:model-value="(v: string | number) => patch({ host: String(v) })" />
        </div>

        <div class="gap-4 grid grid-cols-2">
          <div class="space-y-2">
            <Label for="ssh-port">{{ t('components.serverForm.ssh.sshPort') }}</Label>
            <Input id="ssh-port" :model-value="props.modelValue?.port ?? 22" type="number" placeholder="22" @update:model-value="(v: string | number) => patch({ port: Number(v) })" />
          </div>
          <div class="space-y-2">
            <Label for="ssh-user">{{ t('components.serverForm.ssh.username') }}</Label>
            <Input id="ssh-user" :model-value="props.modelValue?.username" placeholder="username" autocomplete="off" @update:model-value="(v: string | number) => patch({ username: String(v) })" />
          </div>
        </div>

        <div class="space-y-2">
          <Label for="ssh-auth">{{ t('components.serverForm.ssh.authMethod') }}</Label>
          <Select :model-value="props.modelValue?.authMethod ?? 'password'" @update:model-value="(v: string | number) => patch({ authMethod: String(v) as SSHTunnelConfig['authMethod'] })">
            <SelectTrigger id="ssh-auth">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="password">
                  {{ t('components.serverForm.ssh.password') }}
                </SelectItem>
                <SelectItem value="privateKey">
                  {{ t('components.serverForm.ssh.privateKey') }}
                </SelectItem>
                <SelectItem value="agent">
                  {{ t('components.serverForm.ssh.sshAgent') }}
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <template v-if="(props.modelValue?.authMethod ?? 'password') === 'password'">
          <div class="space-y-2">
            <Label for="ssh-password">{{ t('components.serverForm.ssh.sshPassword') }}</Label>
            <Input id="ssh-password" :model-value="props.modelValue?.password" type="password" autocomplete="off" @update:model-value="(v: string | number) => patch({ password: String(v) })" />
          </div>
        </template>

        <template v-if="props.modelValue?.authMethod === 'privateKey'">
          <div class="space-y-2">
            <Label for="ssh-key">{{ t('components.serverForm.ssh.privateKeyPath') }}</Label>
            <Input id="ssh-key" :model-value="props.modelValue?.privateKey" placeholder="/path/to/id_rsa" @update:model-value="(v: string | number) => patch({ privateKey: String(v) })" />
          </div>
          <div class="space-y-2">
            <Label for="ssh-passphrase">{{ t('components.serverForm.ssh.passphraseOptional') }}</Label>
            <Input id="ssh-passphrase" :model-value="props.modelValue?.privateKeyPassphrase" type="password" autocomplete="off" @update:model-value="(v: string | number) => patch({ privateKeyPassphrase: String(v) })" />
          </div>
        </template>

        <template v-if="props.modelValue?.authMethod === 'agent'">
          <p class="text-sm text-muted-foreground">
            {{ t('components.serverForm.ssh.agentHelpText') }}
          </p>
        </template>

        <div class="flex flex-wrap gap-2 items-center">
          <Button variant="outline" type="button" size="sm" @click="loadConfigHosts">
            {{ t('components.serverForm.ssh.importFromConfig') }}
          </Button>
          <Button variant="outline" type="button" size="sm" @click="convertLegacyToProfile">
            {{ t('components.serverForm.ssh.convertToProfile') }}
          </Button>
        </div>
        <div v-if="showConfigHosts" class="border rounded-md max-h-40 overflow-y-auto divide-y">
          <button
            v-for="host in configHosts"
            :key="host.host"
            type="button"
            class="text-sm px-3 py-1.5 text-left w-full hover:bg-muted"
            @click="applyConfigHost(host)"
          >
            {{ host.host }}
            <span class="text-xs text-muted-foreground ml-1">
              {{ host.user ? `${host.user}@` : '' }}{{ host.hostName || '' }}{{ host.port ? `:${host.port}` : '' }}
            </span>
          </button>
        </div>
      </template>

      <!-- System proxy: only a force-enable fallback when OS detection found nothing;
             when a proxy IS detected it renders as the pinned row above instead -->
      <div v-if="!detectedProxy" class="flex gap-2 items-center">
        <Checkbox
          id="ssh-system-proxy"
          :checked="props.modelValue?.useSystemProxy ?? false"
          @update:checked="checked => patch({ useSystemProxy: checked === true })"
        />
        <Label for="ssh-system-proxy" class="text-sm font-normal">
          {{ t('components.serverForm.ssh.useSystemProxy') }}
        </Label>
      </div>

      <SshProfileDialog ref="profileDialogRef" @save="onProfileSaved" />
    </template>
  </div>
</template>
