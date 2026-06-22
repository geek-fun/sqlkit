<script setup lang="ts">
import type { DatabaseType } from '@/store/connectionStore'
import type { SslConfig, SslMode } from '@/types/connection'
import { computed } from 'vue'
import {
  DEFAULT_SSL_MODE,
  isSslSupported,
  needsCertFields,
  needsSqlServerOptions,
} from '@/types/connection'
import SslCertFields from './SslCertFields.vue'
import SslModeSelect from './SslModeSelect.vue'
import SslSqlServerOptions from './SslSqlServerOptions.vue'

const props = defineProps<{
  dbType: DatabaseType
  errors?: Record<string, string>
}>()

const sslConfig = defineModel<SslConfig>({
  default: () => ({ mode: DEFAULT_SSL_MODE }),
})

const dbTypeLabel = computed(() => props.dbType)

const showSslSection = computed(() => isSslSupported(dbTypeLabel.value))

const showCertFields = computed(() =>
  needsCertFields(dbTypeLabel.value, sslConfig.value.mode),
)

const showSqlServerOptions = computed(() =>
  needsSqlServerOptions(dbTypeLabel.value, sslConfig.value.mode),
)

const sslErrors = computed(() => {
  const errors: Record<string, string> = {}
  if (!props.errors)
    return errors
  Object.entries(props.errors).forEach(([key, value]) => {
    if (key.startsWith('ssl.')) {
      errors[key.replace('ssl.', '')] = value
    }
  })
  return errors
})

function handleModeChange(mode: SslMode) {
  sslConfig.value = { ...sslConfig.value, mode }
}
</script>

<template>
  <div v-if="showSslSection" class="space-y-3">
    <SslModeSelect
      :model-value="sslConfig.mode"
      :error="sslErrors.mode"
      @update:model-value="handleModeChange"
    />

    <SslCertFields
      v-if="showCertFields"
      v-model:ca-cert-path="sslConfig.caCertPath"
      v-model:client-cert-path="sslConfig.clientCertPath"
      v-model:client-key-path="sslConfig.clientKeyPath"
      :errors="sslErrors"
    />

    <SslSqlServerOptions
      v-if="showSqlServerOptions"
      v-model:trust-server-certificate="sslConfig.trustServerCertificate"
    />
  </div>
</template>
