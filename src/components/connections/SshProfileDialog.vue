<script setup lang="ts">
import type { SshProfile } from '@/store/sshProfileStore'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
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
import { emptySshProfile, useSshProfileStore } from '@/store/sshProfileStore'

const emit = defineEmits<{
  save: [profile: SshProfile]
}>()
const { t } = useI18n()
const profileStore = useSshProfileStore()

const open = ref(false)
const form = ref<SshProfile>(emptySshProfile())
const isEdit = ref(false)
const saving = ref(false)
const error = ref('')

function show(profile?: SshProfile) {
  form.value = profile ? { ...emptySshProfile(), ...profile } : emptySshProfile()
  isEdit.value = !!profile?.id
  error.value = ''
  open.value = true
}

defineExpose({ show })

const valid = computed(() => !!form.value.name.trim() && !!form.value.host.trim() && form.value.port > 0)

async function save() {
  if (!valid.value || saving.value)
    return
  saving.value = true
  error.value = ''
  try {
    const saved = await profileStore.save({ ...form.value, name: form.value.name.trim() })
    emit('save', saved)
    open.value = false
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
  <Dialog :open="open" @update:open="open = $event">
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

        <div class="space-y-1.5">
          <Label>{{ t('components.serverForm.ssh.authMethod') }}</Label>
          <Select v-model="form.authMethod">
            <SelectTrigger>
              <SelectValue :placeholder="t('components.serverForm.ssh.authAuto')" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="password">
                  {{ t('components.serverForm.ssh.password') }}
                </SelectItem>
                <SelectItem value="key">
                  {{ t('components.serverForm.ssh.privateKey') }}
                </SelectItem>
                <SelectItem value="agent">
                  {{ t('components.serverForm.ssh.sshAgent') }}
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <div v-if="form.authMethod === 'password'" class="space-y-1.5">
          <Label for="profile-password">{{ t('components.serverForm.ssh.sshPassword') }}</Label>
          <Input id="profile-password" v-model="form.password" type="password" autocomplete="off" />
        </div>

        <template v-if="form.authMethod === 'key'">
          <div class="space-y-1.5">
            <Label for="profile-key">{{ t('components.serverForm.ssh.privateKeyPath') }}</Label>
            <Input id="profile-key" v-model="form.keyPath" placeholder="/path/to/id_rsa" />
          </div>
          <div class="space-y-1.5">
            <Label for="profile-passphrase">{{ t('components.serverForm.ssh.passphraseOptional') }}</Label>
            <Input id="profile-passphrase" v-model="form.keyPassphrase" type="password" autocomplete="off" />
          </div>
        </template>

        <p v-if="form.authMethod === 'agent'" class="text-sm text-muted-foreground">
          {{ t('components.serverForm.ssh.agentHelpText') }}
        </p>

        <p v-if="error" class="text-sm text-destructive">
          {{ error }}
        </p>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="open = false">
          {{ t('common.buttons.cancel') }}
        </Button>
        <Button :disabled="!valid || saving" @click="save">
          {{ t('common.buttons.save') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
