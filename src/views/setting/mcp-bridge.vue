<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { DefaultPermissionEditor } from '@/components/ai-permissions'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import CodeCopyBlock from './code-copy-block.vue'

type BridgeSession = {
  client: string | null
  version: string | null
  lastSeenSecs: number
}

type BridgeStatus = {
  running: boolean
  port: number | null
  sessions: BridgeSession[]
}

const SERVER_COMMAND = 'npx -y @geek-fun/data-studio-mcp'
const DOCS_URL = 'https://www.geekfun.club/products/data-studio-agent/'
const CLAUDE_COMMAND
  = 'claude mcp add --transport stdio data-studio -- npx -y @geek-fun/data-studio-mcp'
const CODEX_COMMAND = 'codex mcp add data-studio -- npx -y @geek-fun/data-studio-mcp'
const MCP_JSON = JSON.stringify(
  { mcpServers: { 'data-studio': { command: 'npx', args: ['-y', '@geek-fun/data-studio-mcp'] } } },
  null,
  2,
)

const { t } = useI18n()
const status = ref<BridgeStatus | null>(null)
const loading = ref(true)

const sessionNames = computed(() =>
  (status.value?.sessions ?? []).map(s => s.client ?? `data-studio-mcp/${s.version ?? '?'}`))

const serverText = computed(() => {
  if (!status.value)
    return loading.value ? t('aiPermissions.loading') : t('aiPermissions.unavailable')
  if (!status.value.running)
    return t('pages.settings.mcp.serverStopped')
  return status.value.sessions.length
    ? t('pages.settings.mcp.sessions', { count: status.value.sessions.length, clients: sessionNames.value.join(', ') }, status.value.sessions.length)
    : t('pages.settings.mcp.serverIdle')
})

let inFlight = false
const load = async () => {
  if (inFlight)
    return
  inFlight = true
  try {
    status.value = JSON.parse(await invoke<string>('get_mcp_status'))
  }
  catch {
    // Keep the last known good status; a transient failure is refreshed on the next tick.
  }
  finally {
    inFlight = false
    loading.value = false
  }
}

let poll: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  load()
  poll = setInterval(load, 5000)
})
onBeforeUnmount(() => clearInterval(poll))
</script>

<template>
  <div class="space-y-6">
    <Card>
      <CardHeader>
        <div class="flex flex-wrap gap-3 items-center justify-between">
          <CardTitle>{{ t('pages.settings.mcp.serverTitle') }}</CardTitle>
          <span role="status" class="text-xs text-muted-foreground flex gap-2 items-center">
            <span class="sr-only">{{ t('pages.settings.mcp.status') }}:</span>
            <span
              aria-hidden="true"
              class="rounded-full shrink-0 h-2 w-2"
              :class="status?.running ? 'bg-success' : 'bg-muted-foreground'"
            />
            {{ serverText }}
          </span>
        </div>
        <CardDescription>{{ t('pages.settings.mcp.serverDesc') }}</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div
          v-if="status?.running && !status.sessions.length"
          class="text-xs flex flex-wrap gap-x-2 gap-y-1 items-center"
          role="status"
        >
          <span aria-hidden="true" class="i-lucide-info shrink-0 h-3.5 w-3.5" />
          <span class="text-muted-foreground">{{ t('pages.settings.mcp.sessionsEmpty') }}</span>
          <a
            :href="DOCS_URL"
            target="_blank"
            rel="noopener noreferrer"
            class="text-primary-text hover:underline inline-flex gap-1 items-center"
          >
            {{ t('pages.settings.mcp.install.docsLink') }}
            <span aria-hidden="true" class="i-lucide-external-link h-3 w-3" />
          </a>
        </div>

        <div class="border-t" role="presentation" />

        <div class="space-y-1.5">
          <p class="text-xs font-medium">
            {{ t('pages.settings.mcp.install.serverCommand') }}
          </p>
          <CodeCopyBlock :code="SERVER_COMMAND" :label="t('pages.settings.mcp.install.copy')" />
        </div>
        <Tabs default-value="claude">
          <TabsList class="flex-wrap h-auto justify-start">
            <TabsTrigger value="claude" class="gap-1.5">
              <span aria-hidden="true" class="i-simple-icons-claude h-3.5 w-3.5" />
              {{ t('pages.settings.mcp.install.tabClaude') }}
            </TabsTrigger>
            <TabsTrigger value="codex" class="gap-1.5">
              <span aria-hidden="true" class="i-simple-icons-openai h-3.5 w-3.5" />
              {{ t('pages.settings.mcp.install.tabCodex') }}
            </TabsTrigger>
            <TabsTrigger value="cursor" class="gap-1.5">
              <span aria-hidden="true" class="i-simple-icons-cursor h-3.5 w-3.5" />
              {{ t('pages.settings.mcp.install.tabCursor') }}
            </TabsTrigger>
            <TabsTrigger value="generic" class="gap-1.5">
              <span aria-hidden="true" class="i-lucide-plug h-3.5 w-3.5" />
              {{ t('pages.settings.mcp.install.tabGeneric') }}
            </TabsTrigger>
          </TabsList>
          <TabsContent value="claude" class="mt-3">
            <CodeCopyBlock :code="CLAUDE_COMMAND" :label="t('pages.settings.mcp.install.copy')" />
          </TabsContent>
          <TabsContent value="codex" class="mt-3">
            <CodeCopyBlock :code="CODEX_COMMAND" :label="t('pages.settings.mcp.install.copy')" />
          </TabsContent>
          <TabsContent value="cursor" class="mt-3 space-y-1.5">
            <p class="text-xs text-muted-foreground leading-relaxed">
              {{ t('pages.settings.mcp.install.cursorDesc') }}
            </p>
            <CodeCopyBlock :code="MCP_JSON" :label="t('pages.settings.mcp.install.copy')" />
          </TabsContent>
          <TabsContent value="generic" class="mt-3 space-y-1.5">
            <p class="text-xs text-muted-foreground leading-relaxed">
              {{ t('pages.settings.mcp.install.genericDesc') }}
            </p>
            <CodeCopyBlock :code="MCP_JSON" :label="t('pages.settings.mcp.install.copy')" />
          </TabsContent>
        </Tabs>
        <p class="text-xs text-muted-foreground leading-relaxed">
          {{ t('pages.settings.mcp.install.portEnvNote') }}
        </p>
      </CardContent>
    </Card>
    <Card>
      <CardContent class="p-6">
        <DefaultPermissionEditor />
      </CardContent>
    </Card>
  </div>
</template>
