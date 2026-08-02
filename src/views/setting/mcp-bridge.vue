<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Switch } from '@/components/ui/switch'
import { useConnectionStore } from '@/store'

type McpPermissionMode = 'ReadOnly' | 'DataReadWrite' | 'FullAccess'

type ConnectionOverride = { read_only: boolean }

type McpPolicy = {
  mode: McpPermissionMode
  confirm_destructive: boolean
  allowed_connection_ids: string[]
  connection_overrides: Record<string, ConnectionOverride>
}

const { t } = useI18n()
const connectionStore = useConnectionStore()

const status = ref<{ running: boolean, port: number | null }>({ running: false, port: null })
const portValue = ref<number | undefined>(undefined)
const autoStart = ref(true)
const loading = ref(false)

const policyMode = ref<McpPermissionMode>('ReadOnly')
const confirmDestructive = ref(false)
const allowedConnectionIds = ref<string[]>([])
const connectionOverrides = ref<Record<string, ConnectionOverride>>({})

const permissionModes: { value: McpPermissionMode, labelKey: string }[] = [
  { value: 'ReadOnly', labelKey: 'pages.settings.mcp.modeReadOnly' },
  { value: 'DataReadWrite', labelKey: 'pages.settings.mcp.modeDataReadWrite' },
  { value: 'FullAccess', labelKey: 'pages.settings.mcp.modeFullAccess' },
]

const connections = computed(() => connectionStore.connections)

function applyPolicy(policy: McpPolicy | undefined) {
  if (!policy)
    return
  policyMode.value = policy.mode
  confirmDestructive.value = policy.confirm_destructive
  allowedConnectionIds.value = [...policy.allowed_connection_ids]
  connectionOverrides.value = { ...policy.connection_overrides }
}

async function refreshStatus() {
  const raw = await invoke<string>('get_mcp_status')
  const data = JSON.parse(raw)
  status.value = { running: data.running, port: data.port ?? null }
  portValue.value = data.configuredPort ?? undefined
  autoStart.value = data.autoStart
  applyPolicy(data.policy)
}

function buildPolicy(): McpPolicy {
  return {
    mode: policyMode.value,
    confirm_destructive: confirmDestructive.value,
    allowed_connection_ids: [...allowedConnectionIds.value],
    connection_overrides: Object.fromEntries(
      Object.entries(connectionOverrides.value).map(([k, v]) => [k, { read_only: v.read_only }]),
    ),
  }
}

async function savePolicy() {
  try {
    await invoke('save_mcp_config', {
      port: portValue.value ?? null,
      autoStart: autoStart.value,
      policy: buildPolicy(),
    })
  }
  catch (e) {
    console.error(t('pages.settings.mcp.saveFailed'), e)
  }
}

onMounted(async () => {
  try {
    await Promise.all([
      refreshStatus(),
      connectionStore.fetchConnections(),
    ])
  }
  catch (e) {
    console.error('Failed to get MCP status:', e)
  }
})

function onPortChange(val: string | number) {
  const num = Number(val)
  portValue.value = Number.isInteger(num) && num > 0 ? num : undefined
}

async function onAutoStartChange(val: boolean) {
  autoStart.value = val
  try {
    await invoke('save_mcp_config', { port: portValue.value ?? null, autoStart: val })
  }
  catch (e) {
    console.error('Failed to save MCP config:', e)
  }
}

async function restartBridge() {
  loading.value = true
  try {
    await invoke('save_mcp_config', { port: portValue.value ?? null, autoStart: autoStart.value })
    await refreshStatus()
  }
  catch (e) {
    console.error('Failed to restart MCP bridge:', e)
  }
  finally {
    loading.value = false
  }
}

async function onModeChange(mode: string) {
  policyMode.value = mode as McpPermissionMode
  if (mode !== 'FullAccess') {
    confirmDestructive.value = false
  }
  await savePolicy()
}

async function onConfirmDestructiveChange(val: boolean) {
  confirmDestructive.value = val
  await savePolicy()
}

async function toggleAllowlist(id: string, checked: boolean) {
  allowedConnectionIds.value = checked
    ? [...allowedConnectionIds.value, id]
    : allowedConnectionIds.value.filter(x => x !== id)
  await savePolicy()
}

async function toggleReadOnlyOverride(id: string, checked: boolean) {
  const next = { ...connectionOverrides.value }
  if (checked) {
    next[id] = { read_only: true }
  }
  else {
    delete next[id]
  }
  connectionOverrides.value = next
  await savePolicy()
}

const isAllowlistChecked = (id: string) => allowedConnectionIds.value.includes(id)
const isOverrideChecked = (id: string) => connectionOverrides.value[id]?.read_only ?? false
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.settings.mcp.title') }}</CardTitle>
      <CardDescription>{{ t('pages.settings.mcp.portDesc') }}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-5">
      <!-- Status -->
      <div class="flex items-center justify-between">
        <Label>{{ t('pages.settings.mcp.status') }}</Label>
        <div class="flex gap-2 items-center">
          <span
            class="rounded-full h-2.5 w-2.5"
            :class="status.running ? 'bg-green-500' : 'bg-red-500'"
          />
          <span class="text-sm text-muted-foreground">
            {{
              status.running
                ? t('pages.settings.mcp.running', { port: status.port })
                : t('pages.settings.mcp.stopped')
            }}
          </span>
        </div>
      </div>

      <!-- Port + Restart -->
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.port') }}</Label>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.mcp.portDesc') }}
          </p>
        </div>
        <div class="flex gap-3 items-center">
          <Input
            type="number"
            :model-value="portValue"
            min="1024"
            max="65535"
            class="w-28"
            @update:model-value="onPortChange($event)"
          />
          <Button
            variant="outline"
            size="sm"
            :disabled="loading"
            @click="restartBridge"
          >
            {{ t('pages.settings.mcp.restart') }}
          </Button>
        </div>
      </div>

      <!-- Auto-start -->
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.autoStart') }}</Label>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.mcp.autoStartDesc') }}
          </p>
        </div>
        <Switch
          :checked="autoStart"
          @update:checked="onAutoStartChange"
        />
      </div>

      <!-- Permission Mode -->
      <div class="flex gap-4 items-start justify-between">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.permissionMode') }}</Label>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.mcp.permissionModeDesc') }}
          </p>
        </div>
        <RadioGroup
          :model-value="policyMode"
          class="flex flex-col gap-2"
          @update:model-value="onModeChange"
        >
          <div
            v-for="mode in permissionModes"
            :key="mode.value"
            class="flex gap-2 items-center"
          >
            <RadioGroupItem :id="`mcp-mode-${mode.value}`" :value="mode.value" />
            <Label :for="`mcp-mode-${mode.value}`" class="text-sm font-normal cursor-pointer">
              {{ t(mode.labelKey) }}
            </Label>
          </div>
        </RadioGroup>
      </div>

      <!-- Confirm Destructive -->
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.confirmDestructive') }}</Label>
          <p class="text-xs text-muted-foreground">
            {{
              policyMode === 'FullAccess'
                ? t('pages.settings.mcp.confirmDestructiveDesc')
                : t('pages.settings.mcp.confirmDestructiveDisabledHint')
            }}
          </p>
        </div>
        <Switch
          :checked="confirmDestructive"
          :disabled="policyMode !== 'FullAccess'"
          @update:checked="onConfirmDestructiveChange"
        />
      </div>

      <!-- Connection Allowlist -->
      <div class="space-y-2">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.allowlist') }}</Label>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.mcp.allowlistDesc') }}
          </p>
          <p
            v-if="allowedConnectionIds.length === 0"
            class="text-xs text-muted-foreground italic"
          >
            {{ t('pages.settings.mcp.allowlistEmpty') }}
          </p>
        </div>
        <div class="space-y-2">
          <div
            v-for="conn in connections"
            :key="conn.id"
            class="flex gap-2 items-center"
          >
            <Checkbox
              :checked="isAllowlistChecked(conn.id!)"
              @update:checked="(val: boolean) => toggleAllowlist(conn.id!, val)"
            />
            <Label class="text-sm font-normal cursor-pointer">
              {{ conn.name }}
            </Label>
          </div>
        </div>
      </div>

      <!-- Connection Overrides -->
      <div v-if="connections.length > 0" class="space-y-2">
        <div class="space-y-1">
          <Label>{{ t('pages.settings.mcp.connectionOverrides') }}</Label>
        </div>
        <div class="space-y-2">
          <div
            v-for="conn in connections"
            :key="conn.id"
            class="flex items-center justify-between"
          >
            <Label class="text-sm font-normal cursor-pointer">
              {{ conn.name }}
            </Label>
            <div class="flex gap-2 items-center">
              <span class="text-xs text-muted-foreground">
                {{ t('pages.settings.mcp.overrideReadOnly') }}
              </span>
              <Switch
                :checked="isOverrideChecked(conn.id!)"
                @update:checked="(val: boolean) => toggleReadOnlyOverride(conn.id!, val)"
              />
            </div>
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
