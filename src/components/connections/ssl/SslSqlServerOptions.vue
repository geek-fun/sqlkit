<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'

const trustServerCertificate = defineModel<boolean>('trustServerCertificate', { default: false })

const { t } = useI18n()
</script>

<template>
  <div class="p-3 border rounded-md space-y-3">
    <div class="flex gap-2 items-center">
      <Checkbox
        id="trust-server-cert"
        v-model:checked="trustServerCertificate"
      />
      <Label for="trust-server-cert" class="text-sm cursor-pointer">
        {{ t('ssl.sqlServer.trustServerCert') }}
      </Label>
    </div>
    <p class="text-xs text-muted-foreground pl-6">
      {{ t('ssl.sqlServer.trustServerCertHint') }}
    </p>
  </div>
</template>
