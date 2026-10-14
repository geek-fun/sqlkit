<script setup lang="ts">
import type { SshProfile } from '@/store/sshProfileStore'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { emptySshProfile, profileToTransportLayer, useSshProfileStore } from '@/store/sshProfileStore'

const emit = defineEmits<{
  save: [profile: SshProfile]
}>()
const { t } = useI18n()
const profileStore = useSshProfileStore()

const dialogOpen = ref(false)
const form = ref<SshProfile>(emptySshProfile())
const isEdit = ref(false)
const saving = ref(false)
const error = ref('')
const showSecrets = ref(false)
const testing = ref(false)
const testResult = ref<TestSshResult | null>(null)

/**
 * The tabs cover the concrete methods only; legacy rows stored with ''/none
 * (auto-probe) normalize to password on open like DockIt's resetForm does —
 * and a brand-new profile starts on the password tab.
 */
const normalizeAuthMethod = (method?: string) =>
  method === 'key' || method === 'agent' ? method : 'password'

function show(profile?: SshProfile) {
  form.value = profile
    ? { ...emptySshProfile(), ...profile, authMethod: normalizeAuthMethod(profile.authMethod) }
    : { ...emptySshProfile(), authMethod: 'password' }
  isEdit.value = !!profile?.id
  error.value = ''
  showSecrets.value = false
  testResult.value = null
  dialogOpen.value = true
}

defineExpose({ show })

const valid = computed(() => !!form.value.name.trim() && !!form.value.host.trim() && form.value.port > 0)

async function selectKeyFile() {
  try {
    const selected = await open({ multiple: false })
    if (selected)
      form.value.keyPath = selected
  }
  catch {
    // user cancelled
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

async function testProfile() {
  if (testing.value)
    return
  testing.value = true
  testResult.value = null
  try {
    // DockIt parity: test the bastion against itself with the same Rust-contract
    // layer the real chain builds, so test and connect can't diverge.
    testResult.value = await invoke<TestSshResult>('test_ssh_connection', {
      config: profileToTransportLayer(form.value),
      remoteHost: form.value.host,
      remotePort: form.value.port,
    })
  }
  catch (err) {
    testResult.value = { success: false, message: String(err) }
  }
  finally {
    testing.value = false
  }
}

async function save() {
  if (!valid.value || saving.value)
    return
  saving.value = true
  error.value = ''
  try {
    const saved = await profileStore.save({
      ...form.value,
      name: form.value.name.trim(),
      useSshAgent: form.value.authMethod === 'agent',
    })
    emit('save', saved)
    dialogOpen.value = false
  }
  catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
  finally {
    saving.value = false
  }
}
</script>

<template>
  <Dialog :open="dialogOpen" @update:open="dialogOpen = $event">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <DialogTitle>{{ isEdit ? t('components.serverForm.ssh.editProfile') : t('components.serverForm.ssh.newProfile') }}</DialogTitle>
      </DialogHeader>

      <div class="space-y-3">
        <div class="space-y-1.5">
          <Label for="profile-name">{{ t('components.serverForm.ssh.profileName') }}</Label>
          <Input id="profile-name" v-model="form.name" :placeholder="t('components.serverForm.ssh.profileNamePlaceholder')" />
        </div>

        <div class="space-y-1.5">
          <Label for="profile-host">{{ t('components.serverForm.ssh.sshHost') }}</Label>
          <Input id="profile-host" v-model="form.host" placeholder="ssh.example.com" />
        </div>

        <div class="gap-3 grid grid-cols-2">
          <div class="space-y-1.5">
            <Label for="profile-port">{{ t('components.serverForm.ssh.sshPort') }}</Label>
            <Input id="profile-port" v-model.number="form.port" type="number" placeholder="22" />
          </div>
          <div class="space-y-1.5">
            <Label for="profile-user">{{ t('components.serverForm.ssh.username') }}</Label>
            <Input id="profile-user" v-model="form.username" placeholder="username" autocomplete="off" />
          </div>
        </div>

        <div class="space-y-2">
          <Label>{{ t('components.serverForm.ssh.authMethod') }}</Label>
          <Tabs :model-value="form.authMethod" @update:model-value="form.authMethod = $event as SshProfile['authMethod']">
            <TabsList class="w-full">
              <TabsTrigger value="password" class="flex-1">
                {{ t('components.serverForm.ssh.password') }}
              </TabsTrigger>
              <TabsTrigger value="key" class="flex-1">
                {{ t('components.serverForm.ssh.privateKey') }}
              </TabsTrigger>
              <TabsTrigger value="agent" class="flex-1">
                {{ t('components.serverForm.ssh.sshAgent') }}
              </TabsTrigger>
            </TabsList>

            <!-- min-height absorbs the differences between the three tabs so
                 switching never jiggles the dialog height -->
            <div class="min-h-36">
              <TabsContent value="password" class="mt-3 space-y-1.5">
                <Label for="profile-password">{{ t('components.serverForm.ssh.sshPassword') }}</Label>
                <div class="relative">
                  <Input id="profile-password" v-model="form.password" :type="showSecrets ? 'text' : 'password'" autocomplete="off" class="pr-8" />
                  <button type="button" class="text-muted-foreground right-2 top-1/2 absolute hover:text-foreground -translate-y-1/2" @click="showSecrets = !showSecrets">
                    <span class="i-carbon-view h-4 w-4 block" />
                  </button>
                </div>
              </TabsContent>

              <TabsContent value="key" class="mt-3 space-y-3">
                <div class="space-y-1.5">
                  <Label for="profile-key">{{ t('components.serverForm.ssh.privateKeyPath') }}</Label>
                  <div class="flex gap-2">
                    <div class="flex-1 min-w-0">
                      <Input id="profile-key" v-model="form.keyPath" placeholder="/path/to/id_rsa" />
                    </div>
                    <Button variant="outline" size="icon" type="button" class="shrink-0" :title="t('common.browse')" @click="selectKeyFile">
                      <span class="i-carbon-folder h-4 w-4" />
                    </Button>
                  </div>
                </div>
                <div class="space-y-1.5">
                  <Label for="profile-passphrase">{{ t('components.serverForm.ssh.passphraseOptional') }}</Label>
                  <div class="relative">
                    <Input id="profile-passphrase" v-model="form.keyPassphrase" :type="showSecrets ? 'text' : 'password'" autocomplete="off" class="pr-8" />
                    <button type="button" class="text-muted-foreground right-2 top-1/2 absolute hover:text-foreground -translate-y-1/2" @click="showSecrets = !showSecrets">
                      <span class="i-carbon-view h-4 w-4 block" />
                    </button>
                  </div>
                </div>
              </TabsContent>

              <TabsContent value="agent" class="mt-3 space-y-1.5">
                <Label for="profile-agent-sock">{{ t('components.serverForm.ssh.agentSockPath') }}</Label>
                <Input id="profile-agent-sock" v-model="form.sshAgentSockPath" placeholder="$SSH_AUTH_SOCK" />
                <p class="text-xs text-muted-foreground">
                  {{ t('components.serverForm.ssh.agentHelpText') }}
                </p>
              </TabsContent>
            </div>
          </Tabs>
        </div>

        <!-- Test result: success is localized from structured actors; failure
             keeps the backend diagnostic and wraps so details stay readable -->
        <p
          v-if="testResult"
          class="text-xs leading-relaxed flex gap-1.5 break-words items-start"
          :class="testResult.success ? 'text-green-600 dark:text-green-400' : 'text-destructive'"
        >
          <span v-if="testing" class="i-carbon-circle-dash mt-0.5 shrink-0 h-3.5 w-3.5 animate-spin" />
          <span v-else class="i-carbon-network-3 mt-0.5 shrink-0 h-3.5 w-3.5" />
          <span class="min-w-0">{{ testResult.success && testResult.user && testResult.host && testResult.port
            ? t('components.serverForm.ssh.testSuccess', {
              user: testResult.user,
              host: testResult.host,
              port: testResult.port,
            })
            : testResult.message }}</span>
        </p>

        <p v-if="error" class="text-sm text-destructive">
          {{ error }}
        </p>
      </div>

      <DialogFooter>
        <Button
          variant="outline"
          type="button"
          :disabled="testing || !valid"
          @click="testProfile"
        >
          <span v-if="testing" class="i-carbon-circle-dash text-blue-500 h-3.5 w-3.5 animate-spin" />
          <span v-else class="i-carbon-network-3 h-3.5 w-3.5" />
          {{ t('common.buttons.testConnection') }}
        </Button>
        <Button variant="outline" @click="dialogOpen = false">
          {{ t('common.buttons.cancel') }}
        </Button>
        <Button :disabled="!valid || saving" @click="save">
          {{ t('common.buttons.save') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
