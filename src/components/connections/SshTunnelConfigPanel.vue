<script setup lang="ts">
import type { SSHTunnelConfig } from '@/store/connectionStore'
import type { SshProfile } from '@/store/sshProfileStore'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { UPGRADE_URL } from '@/common'
import SshProfileDialog from '@/components/connections/SshProfileDialog.vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
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
import { useEntitlementStore } from '@/store/entitlementStore'
import { useSshProfileStore } from '@/store/sshProfileStore'

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

const showAdvanced = ref(false)
const detectedProxy = ref<string | null>(null)
const configHosts = ref<Array<{ host: string, hostName?: string, port?: number, user?: string, identityFile?: string }>>([])
const showConfigHosts = ref(false)
const profileDialogRef = ref<InstanceType<typeof SshProfileDialog> | null>(null)

const enabled = computed(() => props.modelValue?.enabled ?? false)
const usingProfiles = computed(() => (props.modelValue?.profileIds?.length ?? 0) > 0)

const hopProfiles = computed(() =>
  (props.modelValue?.profileIds ?? [])
    .map(id => profileStore.byId(id))
    .filter((p): p is SshProfile => !!p),
)

const unusedProfiles = computed(() =>
  profileStore.profiles.filter(p => !(props.modelValue?.profileIds ?? []).includes(p.id)),
)

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
  emit('update:modelValue', base)
}

function toggleSsh(checked: boolean) {
  if (checked && !entitlementStore.isLocalUltimate) {
    // usage-side feature — send to the pricing site (matches dockit)
    openUrl(UPGRADE_URL)
    return
  }
  patch(checked
    ? { enabled: true }
    : { enabled: false, profileIds: [], useSystemProxy: false })
}

function setSource(source: 'profile' | 'manual') {
  if (source === 'profile')
    patch({ profileIds: props.modelValue?.profileIds ?? [] })
  else
    patch({ profileIds: [] })
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

function moveHop(index: number, delta: -1 | 1) {
  const ids = [...(props.modelValue?.profileIds ?? [])]
  const target = index + delta
  if (target < 0 || target >= ids.length) {
    return
  }
  const moved = ids[index]
  ids[index] = ids[target] as string
  ids[target] = moved as string
  patch({ profileIds: ids })
}

function openNewProfile() {
  profileDialogRef.value?.show()
}

function onProfileSaved(profile: SshProfile) {
  if (!props.modelValue?.enabled)
    return
  addHop(profile.id)
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
  if (!showAdvanced.value)
    return
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

watch(showAdvanced, (visible) => {
  if (visible) {
    profileStore.fetch()
    detectProxy()
  }
})

watch(() => props.modelValue?.enabled, (on) => {
  if (on)
    detectProxy()
})
</script>

<template>
  <div class="pt-2">
    <Button
      type="button"
      variant="ghost"
      size="sm"
      class="text-muted-foreground w-full justify-between"
      @click="showAdvanced = !showAdvanced"
    >
      {{ t('components.serverForm.ssh.advancedConfig') }}
      <span class="i-carbon-chevron-down h-4 w-4 transition-transform" :class="[showAdvanced ? 'rotate-180' : '']" />
    </Button>

    <div v-if="showAdvanced" class="mt-3 p-4 border rounded-md space-y-4">
      <div class="flex gap-2 items-center">
        <Checkbox
          id="use-ssh"
          :model-value="enabled"
          @update:model-value="(v: unknown) => toggleSsh(v === true)"
        />
        <Label for="use-ssh">{{ t('components.serverForm.ssh.useSshTunnel') }}</Label>
      </div>

      <template v-if="enabled">
        <!-- Config source -->
        <div class="flex gap-4 items-center">
          <span class="text-xs text-muted-foreground font-medium">{{ t('components.serverForm.ssh.configSource') }}</span>
          <label class="flex gap-1.5 cursor-pointer items-center">
            <input
              type="radio"
              class="h-4 w-4"
              name="ssh-source"
              :checked="usingProfiles"
              @change="setSource('profile')"
            >
            <span class="text-sm">{{ t('components.serverForm.ssh.sourceProfiles') }}</span>
          </label>
          <label class="flex gap-1.5 cursor-pointer items-center">
            <input
              type="radio"
              class="h-4 w-4"
              name="ssh-source"
              :checked="!usingProfiles"
              @change="setSource('manual')"
            >
            <span class="text-sm">{{ t('components.serverForm.ssh.sourceManual') }}</span>
          </label>
        </div>

        <!-- Profile hops mode -->
        <template v-if="usingProfiles">
          <div class="space-y-1.5">
            <div
              v-for="(profile, index) in hopProfiles"
              :key="profile.id"
              class="px-2.5 py-1.5 border rounded-md flex gap-2 items-center"
            >
              <span class="text-[10px] text-muted-foreground font-bold w-4">{{ index + 1 }}</span>
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium truncate">
                  {{ profile.name }}
                </div>
                <div class="text-[11px] text-muted-foreground truncate">
                  {{ profile.username }}@{{ profile.host }}:{{ profile.port }}
                </div>
              </div>
              <Button variant="ghost" size="icon" type="button" class="h-6 w-6" :disabled="index === 0" @click="moveHop(index, -1)">
                <ArrowUp class="h-3.5 w-3.5" />
              </Button>
              <Button variant="ghost" size="icon" type="button" class="h-6 w-6" :disabled="index === hopProfiles.length - 1" @click="moveHop(index, 1)">
                <ArrowDown class="h-3.5 w-3.5" />
              </Button>
              <Button variant="ghost" size="icon" type="button" class="text-destructive h-6 w-6" @click="removeHop(index)">
                <Trash2 class="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>

          <div class="flex gap-2 items-center">
            <Select @update:model-value="addHop">
              <SelectTrigger class="flex-1">
                <SelectValue :placeholder="t('components.serverForm.ssh.addHop')" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem v-for="profile in unusedProfiles" :key="profile.id" :value="profile.id">
                    {{ profile.name }} ({{ profile.username }}@{{ profile.host }})
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
            <Button variant="outline" type="button" class="shrink-0" @click="openNewProfile">
              <Plus class="mr-1 h-4 w-4" />
              {{ t('components.serverForm.ssh.newProfile') }}
            </Button>
          </div>
          <p class="text-[11px] text-muted-foreground">
            {{ t('components.serverForm.ssh.hopOrderHint') }}
          </p>
        </template>

        <!-- Manual mode -->
        <template v-else>
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

        <!-- System proxy -->
        <div class="flex gap-2 items-center">
          <Checkbox
            id="ssh-system-proxy"
            :model-value="props.modelValue?.useSystemProxy ?? false"
            @update:model-value="(v: unknown) => patch({ useSystemProxy: v === true })"
          />
          <Label for="ssh-system-proxy" class="text-sm font-normal">
            {{ t('components.serverForm.ssh.useSystemProxy') }}
          </Label>
          <span v-if="detectedProxy" class="text-[11px] text-muted-foreground truncate">
            ({{ detectedProxy }})
          </span>
        </div>
        <p v-if="usingProfiles" class="text-[11px] text-muted-foreground">
          {{ t('components.serverForm.ssh.hopOrderHint') }}
        </p>
      </template>

      <SshProfileDialog ref="profileDialogRef" @save="onProfileSaved" />
    </div>
  </div>
</template>
