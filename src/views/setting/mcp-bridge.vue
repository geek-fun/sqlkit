<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { SettingsFeedback } from '@/components/settings'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Switch } from '@/components/ui/switch'
import { toast } from '@/composables/useNotifications'
import { useConnectionStore } from '@/store'

type McpPermissionMode = 'ReadOnly' | 'DataReadWrite' | 'FullAccess'
type McpAction = 'read' | 'write' | 'delete'

type ConnectionOverride = {
  read_only: boolean
  allowed_actions?: McpAction[]
}

type McpPolicy = {
  mode: McpPermissionMode
  confirm_destructive: boolean
  allowed_connection_ids: string[]
  connection_overrides: Record<string, ConnectionOverride>
}

const { t } = useI18n()
const connectionStore = useConnectionStore()

const loading = ref(true)
const loaded = ref(false)
const saving = ref(false)
const feedback = ref<{ state: 'idle' | 'pending' | 'success' | 'error', label: string, detail?: string }>({ state: 'idle', label: '' })
const confirmedPolicy = ref<McpPolicy>()
const defaultPort = 9121
const status = ref<{ running: boolean, port: number | null }>({ running: false, port: null })
const portValue = ref<number | undefined>(undefined)
const autoStart = ref(true)
const restartPhase = ref<'idle' | 'shutting-down' | 'starting' | 'failed'>('idle')
const restarting = computed(() => restartPhase.value === 'shutting-down' || restartPhase.value === 'starting')

const policyMode = ref<McpPermissionMode>('DataReadWrite')
const confirmDestructive = ref(true)
const allowedConnectionIds = ref<string[]>([])
const connectionOverrides = ref<Record<string, ConnectionOverride>>({})

const permissionModes: { value: McpPermissionMode, labelKey: string }[] = [
  { value: 'ReadOnly', labelKey: 'pages.settings.mcp.modeReadOnly' },
  { value: 'DataReadWrite', labelKey: 'pages.settings.mcp.modeDataReadWrite' },
  { value: 'FullAccess', labelKey: 'pages.settings.mcp.modeFullAccess' },
]

const actionOptions = computed(() => [
  { value: 'read' as const, label: t('pages.settings.mcp.actionRead') },
  { value: 'write' as const, label: t('pages.settings.mcp.actionWrite') },
  { value: 'delete' as const, label: t('pages.settings.mcp.actionDelete') },
])

const connections = computed(() => connectionStore.connections)

const permissionModeDesc = computed(() => {
  switch (policyMode.value) {
    case 'ReadOnly':
      return t('pages.settings.mcp.modeReadOnlyDesc')
    case 'DataReadWrite':
      return t('pages.settings.mcp.modeDataReadWriteDesc')
    case 'FullAccess':
      return t('pages.settings.mcp.modeFullAccessDesc')
    default:
      return t('pages.settings.mcp.modeReadOnlyDesc')
  }
})

const statusDotClass = computed(() => {
  if (restartPhase.value === 'shutting-down' || restartPhase.value === 'starting')
    return 'bg-warning animate-pulse'
  return restartPhase.value === 'failed' ? 'bg-destructive-text' : status.value.running ? 'bg-success' : 'bg-muted-foreground'
})

const statusText = computed(() => {
  if (!loaded.value)
    return t(`pages.settings.feedback.${loading.value ? 'loading' : 'loadFailed'}`)
  switch (restartPhase.value) {
    case 'shutting-down':
      return t('pages.settings.mcp.shuttingDown')
    case 'starting':
      return t('pages.settings.mcp.starting')
    case 'failed':
      return t('pages.settings.mcp.restartFailed')
    default:
      return status.value.running
        ? t('pages.settings.mcp.running', { port: status.value.port })
        : t('pages.settings.mcp.stopped')
  }
})

const restartButtonText = computed(() => {
  switch (restartPhase.value) {
    case 'shutting-down':
      return t('pages.settings.mcp.shuttingDown')
    case 'starting':
      return t('pages.settings.mcp.starting')
    default:
      return t('pages.settings.mcp.restart')
  }
})

function applyPolicy(policy: McpPolicy | undefined) {
  if (!policy)
    return
  policyMode.value = policy.mode ?? 'DataReadWrite'
  confirmDestructive.value = policy.confirm_destructive ?? true
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
  confirmedPolicy.value = data.policy
}

function buildPolicy(): McpPolicy {
  return {
    mode: policyMode.value,
    confirm_destructive: confirmDestructive.value,
    allowed_connection_ids: [...allowedConnectionIds.value],
    connection_overrides: Object.fromEntries(
      Object.entries(connectionOverrides.value).map(([k, v]) => [
        k,
        { read_only: v.read_only, ...(v.allowed_actions ? { allowed_actions: v.allowed_actions } : {}) },
      ]),
    ),
  }
}

async function savePolicy() {
  if (saving.value || !loaded.value || loading.value || restarting.value)
    return
  saving.value = true
  feedback.value = { state: 'pending', label: t('pages.settings.feedback.saving') }
  try {
    // Policy-only save: hot-swaps into the running bridge without a restart.
    const policy = buildPolicy()
    await invoke('save_mcp_policy', { policy })
    confirmedPolicy.value = policy
    feedback.value = { state: 'success', label: t('pages.settings.feedback.saved') }
  }
  catch (e) {
    applyPolicy(confirmedPolicy.value)
    feedback.value = { state: 'error', label: t('pages.settings.mcp.saveFailed'), detail: String(e) }
  }
  finally {
    saving.value = false
  }
}

const loadSettings = async () => {
  loading.value = true
  feedback.value = { state: 'pending', label: t('pages.settings.feedback.loading') }
  try {
    await Promise.all([
      refreshStatus(),
      connectionStore.fetchConnections(),
    ])
    loaded.value = true
    feedback.value = { state: 'idle', label: '' }
  }
  catch (e) {
    feedback.value = { state: 'error', label: t('pages.settings.feedback.loadFailed'), detail: String(e) }
  }
  finally {
    loading.value = false
  }
}
onMounted(loadSettings)

function onPortChange(val: string | number) {
  const num = Number(val)
  portValue.value = Number.isInteger(num) && num > 0 ? num : undefined
}

async function onAutoStartChange(val: boolean) {
  const previous = autoStart.value
  autoStart.value = val
  saving.value = true
  feedback.value = { state: 'pending', label: t('pages.settings.feedback.saving') }
  try {
    await invoke('save_mcp_config', { port: portValue.value ?? null, autoStart: val })
    feedback.value = { state: 'success', label: t('pages.settings.feedback.saved') }
  }
  catch (e) {
    autoStart.value = previous
    feedback.value = { state: 'error', label: t('pages.settings.mcp.saveFailed'), detail: String(e) }
  }
  finally {
    saving.value = false
  }
}

async function restartBridge() {
  restartPhase.value = 'shutting-down'
  try {
    await invoke('save_mcp_config', {
      port: portValue.value ?? null,
      autoStart: autoStart.value,
      restart: true,
      policy: buildPolicy(),
    })
    restartPhase.value = 'starting'
  }
  catch (e) {
    restartPhase.value = 'failed'
    feedback.value = { state: 'error', label: t('pages.settings.mcp.restartFailedDetail'), detail: String(e) }
    console.error('Failed to restart MCP bridge:', e)
    toast.error(t('pages.settings.mcp.restartFailedDetail'))
    return
  }

  const deadline = Date.now() + 60_000
  while (Date.now() < deadline) {
    await new Promise(resolve => setTimeout(resolve, 500))
    try {
      const raw = await invoke<string>('get_mcp_status')
      const data = JSON.parse(raw)
      if (data.running) {
        status.value = { running: true, port: data.port ?? null }
        restartPhase.value = 'idle'
        feedback.value = { state: 'success', label: t('pages.settings.mcp.restartSuccess') }
        toast.success(t('pages.settings.mcp.restartSuccess'))
        return
      }
    }
    catch (e) {
      console.error('Bridge status check failed during restart:', e)
    }
  }

  restartPhase.value = 'failed'
  status.value = { running: false, port: null }
  feedback.value = { state: 'error', label: t('pages.settings.mcp.restartTimeout') }
  toast.error(t('pages.settings.mcp.restartTimeout'))
}

async function onModeChange(mode: string) {
  // RadioGroupItem click + wrapper div click both reach here — guard so a
  // single click saves once instead of double-restarting the bridge.
  if (saving.value || !loaded.value || loading.value || restarting.value || mode === policyMode.value)
    return
  policyMode.value = mode as McpPermissionMode
  await savePolicy()
}

async function onConfirmDestructiveChange(val: boolean) {
  confirmDestructive.value = val
  await savePolicy()
}

const allowlistEnabled = computed(() => allowedConnectionIds.value.length > 0)

async function onAllowlistEnableChange(val: boolean) {
  allowedConnectionIds.value = val
    ? connections.value.filter(c => c.id != null).map(c => String(c.id))
    : []
  await savePolicy()
}

function connectionActions(id: string): McpAction[] {
  const override = connectionOverrides.value[id]
  if (override?.allowed_actions)
    return override.allowed_actions
  if (override?.read_only)
    return ['read']
  return ['read', 'write', 'delete']
}

async function onActionToggle(id: string, action: McpAction) {
  const current = connectionActions(id)
  const next = current.includes(action) ? current.filter(a => a !== action) : [...current, action]
  const nextOverrides = { ...connectionOverrides.value }
  if (next.length === 3) {
    delete nextOverrides[id]
  }
  else {
    nextOverrides[id] = {
      read_only: next.length === 1 && next[0] === 'read',
      allowed_actions: next,
    }
  }
  connectionOverrides.value = nextOverrides
  await savePolicy()
}
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.settings.mcp.title') }}</CardTitle>
      <CardDescription>{{ t('pages.settings.mcp.portDesc') }}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-5">
      <SettingsFeedback v-bind="restartPhase === 'shutting-down' || restartPhase === 'starting' ? { state: 'pending', label: statusText } : feedback" />
      <Button v-if="feedback.state === 'error'" size="sm" variant="outline" @click="loadSettings">
        {{ t('pages.settings.feedback.retry') }}
      </Button>
      <fieldset class="p-0 border-0 min-w-0 space-y-5" :disabled="!loaded || loading || saving || restartPhase === 'shutting-down' || restartPhase === 'starting'">
        <!-- Status -->
        <div class="flex items-center justify-between">
          <Label>{{ t('pages.settings.mcp.status') }}</Label>
          <div class="flex gap-2 items-center">
            <span
              class="rounded-full h-2.5 w-2.5"
              :class="statusDotClass"
            />
            <span class="text-sm text-muted-foreground">
              {{ statusText }}
            </span>
          </div>
        </div>

        <!-- Port + Auto-start (card, same row) -->
        <div class="px-5 py-4 border border-border rounded-lg bg-card space-y-3">
          <div class="flex flex-wrap gap-6 items-end justify-between">
            <div class="flex-1 space-y-3">
              <div>
                <h4 class="text-sm font-semibold">
                  {{ t('pages.settings.mcp.port') }}
                </h4>
                <p class="text-xs text-muted-foreground mt-1">
                  {{ t('pages.settings.mcp.portDesc') }}
                </p>
              </div>
              <div class="flex gap-3 items-center">
                <Input
                  type="number"
                  :model-value="portValue"
                  :placeholder="String(defaultPort)"
                  min="1024"
                  max="65535"
                  class="w-28"
                  @update:model-value="onPortChange($event)"
                />
                <Button
                  variant="outline"
                  size="sm"
                  :disabled="restartPhase === 'shutting-down' || restartPhase === 'starting'"
                  @click="restartBridge"
                >
                  <span
                    v-if="restartPhase === 'shutting-down' || restartPhase === 'starting'"
                    class="i-carbon-circle-dash mr-2 shrink-0 h-4 w-4 animate-spin"
                  />
                  {{ restartButtonText }}
                </Button>
              </div>
            </div>
            <div class="space-y-3">
              <div>
                <h4 class="text-sm font-semibold">
                  {{ t('pages.settings.mcp.autoStart') }}
                </h4>
                <p class="text-xs text-muted-foreground mt-1">
                  {{ t('pages.settings.mcp.autoStartDesc') }}
                </p>
              </div>
              <Switch
                :checked="autoStart"
                @update:checked="onAutoStartChange"
              />
            </div>
          </div>
        </div>

        <!-- Permission Mode (Font Weight selector style) -->
        <div class="px-5 py-4 border border-border rounded-lg bg-card space-y-3">
          <div>
            <h4 class="text-sm font-semibold">
              {{ t('pages.settings.mcp.permissionMode') }}
            </h4>
            <p class="text-xs text-muted-foreground mt-1">
              {{ t('pages.settings.mcp.permissionModeDesc') }}
            </p>
          </div>
          <RadioGroup
            :model-value="policyMode"
            class="flex flex-row flex-wrap gap-3"
            @update:model-value="onModeChange"
          >
            <div
              v-for="mode in permissionModes"
              :key="mode.value"
              class="px-4 py-2.5 border rounded-lg flex gap-2.5 cursor-pointer transition-all items-center" :class="[
                policyMode === mode.value
                  ? 'border-primary bg-primary/5 shadow-sm ring-1 ring-primary/20'
                  : 'border-input hover:border-primary/50 hover:bg-accent/50',
              ]"
              @click="onModeChange(mode.value)"
            >
              <RadioGroupItem :id="`mcp-mode-${mode.value}`" :value="mode.value" />
              <Label
                :for="`mcp-mode-${mode.value}`"
                class="text-sm font-medium cursor-pointer whitespace-nowrap"
              >
                {{ t(mode.labelKey) }}
              </Label>
            </div>
          </RadioGroup>
          <p class="text-xs text-muted-foreground mt-2">
            {{ permissionModeDesc }}
          </p>

          <!-- Confirm Destructive — only shown for FullAccess -->
          <div
            v-if="policyMode === 'FullAccess'"
            class="mt-3 pt-3 border-t border-border/60 flex items-center justify-between"
          >
            <div class="flex-1">
              <h4 class="text-sm font-medium">
                {{ t('pages.settings.mcp.confirmDestructive') }}
              </h4>
              <p class="text-xs text-muted-foreground mt-1">
                {{ t('pages.settings.mcp.confirmDestructiveDesc') }}
              </p>
            </div>
            <Switch
              :checked="confirmDestructive"
              @update:checked="onConfirmDestructiveChange"
            />
          </div>
        </div>

        <!-- Connection Access Table (allowlist + overrides merged) -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <div class="space-y-1">
              <Label>{{ t('pages.settings.mcp.connectionAccess') }}</Label>
              <p class="text-xs text-muted-foreground">
                {{ t('pages.settings.mcp.connectionAccessDesc') }}
              </p>
            </div>
            <Switch
              :checked="allowlistEnabled"
              @update:checked="onAllowlistEnableChange"
            />
          </div>
          <p
            v-if="!allowlistEnabled"
            class="text-xs text-muted-foreground italic"
          >
            {{ t('pages.settings.mcp.allowlistEmpty') }}
          </p>
          <div
            v-else
            class="border border-border/70 rounded-3xl bg-card/70 shadow-sm overflow-hidden"
          >
            <div class="max-h-64 overflow-y-auto">
              <table class="text-sm w-full">
                <thead class="bg-card/95 top-0 sticky backdrop-blur">
                  <tr class="text-xs text-muted-foreground text-left">
                    <th class="font-medium px-4 py-2.5">
                      {{ t('pages.settings.mcp.connectionName') }}
                    </th>
                    <th class="font-medium px-4 py-2.5">
                      {{ t('pages.settings.mcp.connectionType') }}
                    </th>
                    <th class="font-medium px-4 py-2.5">
                      {{ t('pages.settings.mcp.allowedActions') }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="conn in connections"
                    :key="conn.id"
                    class="border-t border-border/60"
                  >
                    <td class="font-medium px-4 py-3 whitespace-nowrap">
                      {{ conn.name }}
                    </td>
                    <td class="text-muted-foreground px-4 py-3 whitespace-nowrap">
                      {{ conn.type }}
                    </td>
                    <td class="px-4 py-3">
                      <div class="flex gap-1 items-center">
                        <button
                          v-for="action in actionOptions"
                          :key="action.value"
                          type="button"
                          class="text-xs px-2 py-1 border rounded-md cursor-pointer transition-all"
                          :class="connectionActions(String(conn.id)).includes(action.value)
                            ? 'border-primary bg-primary/10 text-primary-text font-medium'
                            : 'border-input text-muted-foreground hover:border-primary/50'"
                          @click="onActionToggle(String(conn.id), action.value)"
                        >
                          {{ action.label }}
                        </button>
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </fieldset>
    </CardContent>
  </Card>
</template>
