<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

defineProps<{
  errors?: Record<string, string>
}>()

const caCertPath = defineModel<string>('caCertPath', { default: '' })
const clientCertPath = defineModel<string>('clientCertPath', { default: '' })
const clientKeyPath = defineModel<string>('clientKeyPath', { default: '' })

const { t } = useI18n()

async function selectFile(type: 'ca' | 'client' | 'key') {
  const selected = await open({
    multiple: false,
    filters: [
      {
        name: 'Certificate',
        extensions: ['pem', 'crt', 'cer', 'key', 'p12', 'pfx'],
      },
    ],
  })

  if (typeof selected === 'string') {
    if (type === 'ca') {
      caCertPath.value = selected
    }
    else if (type === 'client') {
      clientCertPath.value = selected
    }
    else if (type === 'key') {
      clientKeyPath.value = selected
    }
  }
}
</script>

<template>
  <div class="p-3 border rounded-md space-y-3">
    <div class="text-sm text-muted-foreground font-medium">
      {{ t('ssl.certSettings.title') }}
    </div>

    <div class="space-y-1">
      <div class="flex gap-2 items-center">
        <Label class="text-xs min-w-28">
          {{ t('ssl.certSettings.caCert') }}
        </Label>
        <Input
          v-model="caCertPath"
          :placeholder="t('ssl.certSettings.caCertPlaceholder')"
          class="flex-1"
          :class="{ 'border-destructive': errors?.caCertPath }"
          readonly
        />
        <Button
          variant="outline"
          size="sm"
          @click="selectFile('ca')"
        >
          {{ t('common.browse') }}
        </Button>
      </div>
      <p v-if="errors?.caCertPath" class="text-sm text-destructive ml-28">
        {{ errors.caCertPath }}
      </p>
    </div>

    <div class="space-y-1">
      <div class="flex gap-2 items-center">
        <Label class="text-xs min-w-28">
          {{ t('ssl.certSettings.clientCert') }}
        </Label>
        <Input
          v-model="clientCertPath"
          :placeholder="t('ssl.certSettings.clientCertPlaceholder')"
          class="flex-1"
          :class="{ 'border-destructive': errors?.clientCertPath }"
          readonly
        />
        <Button
          variant="outline"
          size="sm"
          @click="selectFile('client')"
        >
          {{ t('common.browse') }}
        </Button>
      </div>
      <p v-if="errors?.clientCertPath" class="text-sm text-destructive ml-28">
        {{ errors.clientCertPath }}
      </p>
    </div>

    <div class="space-y-1">
      <div class="flex gap-2 items-center">
        <Label class="text-xs min-w-28">
          {{ t('ssl.certSettings.clientKey') }}
        </Label>
        <Input
          v-model="clientKeyPath"
          :placeholder="t('ssl.certSettings.clientKeyPlaceholder')"
          class="flex-1"
          :class="{ 'border-destructive': errors?.clientKeyPath }"
          readonly
        />
        <Button
          variant="outline"
          size="sm"
          @click="selectFile('key')"
        >
          {{ t('common.browse') }}
        </Button>
      </div>
      <p v-if="errors?.clientKeyPath" class="text-sm text-destructive ml-28">
        {{ errors.clientKeyPath }}
      </p>
    </div>
  </div>
</template>
