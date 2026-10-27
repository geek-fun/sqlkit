<script setup lang="ts">
import type { SSHTunnelConfig } from '@/store/connectionStore'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const emit = defineEmits<{ save: [value: SSHTunnelConfig] }>()
const { t } = useI18n()
const dialogOpen = ref(false)
const draft = ref<SSHTunnelConfig>()
type ConfigHost = { host: string, hostName?: string, port?: number, user?: string, identityFile?: string }
const configHosts = ref<ConfigHost[]>([])
const showConfigHosts = ref(false)
const importError = ref('')
const valid = computed(() => {
  const config = draft.value
  return !!config && !!config.host.trim() && !!config.username.trim()
    && Number.isInteger(config.port) && config.port > 0 && config.port <= 65535
})

const show = (config: SSHTunnelConfig) => {
  draft.value = { ...config }
  showConfigHosts.value = false
  importError.value = ''
  dialogOpen.value = true
}
const updateDraft = (value: Partial<SSHTunnelConfig>) => {
  if (draft.value)
    draft.value = { ...draft.value, ...value }
}
const loadConfigHosts = async () => {
  importError.value = ''
  try {
    configHosts.value = await invoke('list_ssh_config_hosts')
    showConfigHosts.value = true
  }
  catch (error) {
    importError.value = String(error)
  }
}
const applyConfigHost = (host: ConfigHost) => {
  updateDraft({
    host: host.hostName || host.host,
    port: host.port ?? 22,
    username: host.user ?? '',
    authMethod: host.identityFile ? 'privateKey' : 'password',
    privateKey: host.identityFile ?? '',
  })
  showConfigHosts.value = false
}
const save = () => {
  if (!valid.value || !draft.value)
    return
  emit('save', { ...draft.value, host: draft.value.host.trim(), username: draft.value.username.trim() })
  dialogOpen.value = false
}
defineExpose({ show })
</script>

<template>
  <Dialog v-model:open="dialogOpen">
    <DialogContent class="flex flex-col max-h-[85vh] max-w-md">
      <DialogHeader>
        <DialogTitle>{{ t('components.serverForm.ssh.editTunnel') }}</DialogTitle>
        <DialogDescription>{{ t('components.serverForm.ssh.connectionOnly') }}</DialogDescription>
      </DialogHeader>
      <div v-if="draft" class="pr-1 min-h-0 overflow-y-auto space-y-3">
        <div class="space-y-2">
          <Label for="ssh-host">{{ t('components.serverForm.ssh.sshHost') }}</Label>
          <Input id="ssh-host" :model-value="draft?.host" placeholder="ssh.example.com" @update:model-value="(v: string | number) => updateDraft({ host: String(v) })" />
        </div>

        <div class="gap-4 grid grid-cols-2">
          <div class="space-y-2">
            <Label for="ssh-port">{{ t('components.serverForm.ssh.sshPort') }}</Label>
            <Input id="ssh-port" :model-value="draft?.port ?? 22" type="number" placeholder="22" @update:model-value="(v: string | number) => updateDraft({ port: Number(v) })" />
          </div>
          <div class="space-y-2">
            <Label for="ssh-user">{{ t('components.serverForm.ssh.username') }}</Label>
            <Input id="ssh-user" :model-value="draft?.username" placeholder="username" autocomplete="off" @update:model-value="(v: string | number) => updateDraft({ username: String(v) })" />
          </div>
        </div>

        <div class="space-y-2">
          <Label for="ssh-auth">{{ t('components.serverForm.ssh.authMethod') }}</Label>
          <Select :model-value="draft?.authMethod ?? 'password'" @update:model-value="(v: string | number) => updateDraft({ authMethod: String(v) as SSHTunnelConfig['authMethod'] })">
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

        <template v-if="(draft?.authMethod ?? 'password') === 'password'">
          <div class="space-y-2">
            <Label for="ssh-password">{{ t('components.serverForm.ssh.sshPassword') }}</Label>
            <Input id="ssh-password" :model-value="draft?.password" type="password" autocomplete="off" @update:model-value="(v: string | number) => updateDraft({ password: String(v) })" />
          </div>
        </template>

        <template v-if="draft?.authMethod === 'privateKey'">
          <div class="space-y-2">
            <Label for="ssh-key">{{ t('components.serverForm.ssh.privateKeyPath') }}</Label>
            <Input id="ssh-key" :model-value="draft?.privateKey" placeholder="/path/to/id_rsa" @update:model-value="(v: string | number) => updateDraft({ privateKey: String(v) })" />
          </div>
          <div class="space-y-2">
            <Label for="ssh-passphrase">{{ t('components.serverForm.ssh.passphraseOptional') }}</Label>
            <Input id="ssh-passphrase" :model-value="draft?.privateKeyPassphrase" type="password" autocomplete="off" @update:model-value="(v: string | number) => updateDraft({ privateKeyPassphrase: String(v) })" />
          </div>
        </template>

        <template v-if="draft?.authMethod === 'agent'">
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
        <p v-if="showConfigHosts && configHosts.length === 0" class="text-xs text-muted-foreground">
          {{ t('components.serverForm.ssh.noConfigHosts') }}
        </p>
        <p v-if="importError" role="alert" class="text-sm text-destructive-text break-words">
          {{ importError }}
        </p>
      </div>
      <DialogFooter>
        <Button variant="outline" type="button" @click="dialogOpen = false">
          {{ t('common.buttons.cancel') }}
        </Button>
        <Button type="button" :disabled="!valid" @click="save">
          {{ t('common.buttons.save') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
