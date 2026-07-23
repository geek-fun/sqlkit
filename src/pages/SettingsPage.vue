<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AppLayout from '@/components/layout/AppLayout.vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
// Separator is rendered inline as a styled div
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { ThemeType, useAppStore } from '@/store/appStore'
import AiSettings from '@/views/setting/ai-settings.vue'
import JreDriverSection from '@/views/setting/jre-driver-section.vue'

const appStore = useAppStore()
const { t, locale: _locale } = useI18n()
const { checkForUpdates, downloadAndInstall, isChecking, isDownloading, isInstalling, updateAvailable, updateInfo, downloadProgress } = useAppUpdater()

const version = ref('')

onMounted(async () => {
  try {
    version.value = await invoke<string>('get_app_version')
  }
  catch {
    version.value = import.meta.env.VITE_APP_VERSION ?? '0.0.0'
  }
})

// --- Theme ---
const currentTheme = computed(() => appStore.themeType)

const themeOptions = [
  { value: ThemeType.AUTO, label: () => t('pages.settings.appearance.themes.system') },
  { value: ThemeType.DARK, label: () => t('pages.settings.appearance.themes.dark') },
  { value: ThemeType.LIGHT, label: () => t('pages.settings.appearance.themes.light') },
]

const handleThemeChange = (theme: ThemeType) => appStore.setThemeType(theme)

// --- Language ---
// Note: Language change functionality removed - using i18n locale directly

const fontSizeInput = ref(String(appStore.editorConfig.fontSize))
const fontSizeError = ref('')

function handleFontSizeChange(val: string | number) {
  fontSizeInput.value = String(val)
  const num = Number(val)
  if (!Number.isInteger(num) || num < 8 || num > 32) {
    fontSizeError.value = t('pages.settings.editor.fontSizeError')
  }
  else {
    fontSizeError.value = ''
    appStore.setEditorConfig({ fontSize: num })
  }
}

const tabSizeInput = ref(String(appStore.editorConfig.tabSize))
const tabSizeError = ref('')

function handleTabSizeChange(val: string | number) {
  tabSizeInput.value = String(val)
  const num = Number(val)
  if (!Number.isInteger(num) || num < 1 || num > 8) {
    tabSizeError.value = t('pages.settings.editor.tabSizeError')
  }
  else {
    tabSizeError.value = ''
    appStore.setEditorConfig({ tabSize: num })
  }
}

const indentWidthInput = ref(String(appStore.editorConfig.indentWidth ?? 2))
const indentWidthError = ref('')

function handleIndentWidthChange(val: string | number) {
  indentWidthInput.value = String(val)
  const num = Number(val)
  if (!Number.isInteger(num) || num < 1 || num > 8) {
    indentWidthError.value = t('pages.settings.editor.indentWidthError')
  }
  else {
    indentWidthError.value = ''
    appStore.setEditorConfig({ indentWidth: num })
  }
}

const lineWidthInput = ref(String(appStore.editorConfig.lineWidth ?? 120))
const lineWidthError = ref('')

function handleLineWidthChange(val: string | number) {
  lineWidthInput.value = String(val)
  const num = Number(val)
  if (!Number.isInteger(num) || num < 40 || num > 200) {
    lineWidthError.value = t('pages.settings.editor.lineWidthError')
  }
  else {
    lineWidthError.value = ''
    appStore.setEditorConfig({ lineWidth: num })
  }
}

const wordWrap = computed(() => appStore.editorConfig.wordWrap)
const showLineNumbers = computed(() => appStore.editorConfig.showLineNumbers)
const showMinimap = computed(() => appStore.editorConfig.showMinimap)

const toggleWordWrap = () => appStore.setEditorConfig({ wordWrap: !appStore.editorConfig.wordWrap })
const toggleLineNumbers = () => appStore.setEditorConfig({ showLineNumbers: !appStore.editorConfig.showLineNumbers })
const toggleMinimap = () => appStore.setEditorConfig({ showMinimap: !appStore.editorConfig.showMinimap })

const defaultLimitInput = ref(String(appStore.queryConfig.defaultLimit))
const defaultLimitError = ref('')

function handleDefaultLimitChange(val: string | number) {
  defaultLimitInput.value = String(val)
  const num = Number(val)
  if (!Number.isInteger(num) || num < 10 || num > 10000) {
    defaultLimitError.value = t('pages.settings.query.defaultLimitError')
  }
  else {
    defaultLimitError.value = ''
    appStore.setDefaultLimit(num)
  }
}

const queryTimeoutInput = ref(String(Math.round(appStore.queryConfig.queryTimeout / 1000)))
const queryTimeoutError = ref('')

function handleQueryTimeoutChange(val: string | number) {
  queryTimeoutInput.value = String(val)
  const num = Number(val)
  if (!Number.isInteger(num) || num < 1 || num > 300) {
    queryTimeoutError.value = t('pages.settings.query.queryTimeoutError')
  }
  else {
    queryTimeoutError.value = ''
    appStore.setQueryTimeout(num * 1000)
  }
}

const autoSave = computed(() => appStore.queryConfig.autoSave)
const toggleAutoSave = () => appStore.setAutoSave(!appStore.queryConfig.autoSave)

async function handleCheckUpdates() {
  if (updateAvailable.value) {
    await downloadAndInstall()
    return
  }
  await checkForUpdates(true)
  // If an update was found, automatically start downloading
  if (updateAvailable.value && updateInfo.value) {
    await downloadAndInstall()
  }
}
</script>

<template>
  <AppLayout>
    <div class="flex flex-col h-full">
      <!-- Sticky header -->
      <div class="px-6 py-4 border-b bg-background top-0 sticky z-10">
        <div class="mx-auto flex gap-3 max-w-4xl items-center">
          <h1 class="text-xl font-semibold">
            {{ t('pages.settings.title') }}
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">{{ t('pages.settings.subtitle') }}</span>
        </div>
      </div>

      <!-- Tabbed settings content -->
      <Tabs
        default-value="appearance"
        class="px-6 py-4 flex-1 overflow-y-auto"
      >
        <div class="mx-auto max-w-4xl">
          <TabsList class="mb-6">
            <TabsTrigger value="appearance">
              {{ t('pages.settings.appearance.title') }}
            </TabsTrigger>
            <TabsTrigger value="editor">
              {{ t('pages.settings.editor.title') }}
            </TabsTrigger>
            <TabsTrigger value="query">
              {{ t('pages.settings.query.title') }}
            </TabsTrigger>
            <TabsTrigger value="ai">
              {{ t('pages.settings.ai.tabLabel') }}
            </TabsTrigger>
            <TabsTrigger value="jre">
              {{ t('pages.settings.jre.title') }}
            </TabsTrigger>
            <TabsTrigger value="about">
              {{ t('pages.settings.about.title') }}
            </TabsTrigger>
          </TabsList>

          <!-- Appearance Tab -->
          <TabsContent value="appearance" class="mt-0">
            <Card>
              <CardHeader>
                <CardTitle>{{ t('pages.settings.appearance.title') }}</CardTitle>
                <CardDescription>{{ t('pages.settings.appearance.description') }}</CardDescription>
              </CardHeader>
              <CardContent class="space-y-6">
                <!-- Theme selector -->
                <div class="space-y-3">
                  <Label>{{ t('pages.settings.appearance.theme') }}</Label>
                  <div class="gap-4 grid grid-cols-3">
                    <button
                      v-for="opt in themeOptions"
                      :key="opt.value"
                      class="p-2 border-2 rounded-lg cursor-pointer transition-colors relative focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      :class="currentTheme === opt.value
                        ? 'border-primary'
                        : 'border-border hover:border-muted-foreground'"
                      @click="handleThemeChange(opt.value)"
                    >
                      <!-- Theme preview -->
                      <div class="mb-2 rounded flex h-24 overflow-hidden" :class="opt.value === ThemeType.LIGHT ? 'bg-white' : 'bg-zinc-900'">
                        <!-- Auto: split preview -->
                        <template v-if="opt.value === ThemeType.AUTO">
                          <div class="flex h-full w-1/2">
                            <div class="p-1 bg-zinc-800 flex flex-col gap-1 h-full w-6">
                              <div class="rounded-sm bg-zinc-600 h-3 w-3" />
                              <div class="rounded-sm bg-zinc-600 h-3 w-3" />
                              <div class="rounded-sm bg-zinc-600 h-3 w-3" />
                            </div>
                            <div class="p-1 bg-zinc-900 flex-1 space-y-1">
                              <div class="rounded bg-zinc-700 h-1.5" />
                              <div class="rounded bg-zinc-700 h-1.5 w-3/4" />
                              <div class="rounded bg-zinc-700 h-1.5 w-1/2" />
                            </div>
                          </div>
                          <div class="flex h-full w-1/2">
                            <div class="p-1 bg-gray-200 flex flex-col gap-1 h-full w-6">
                              <div class="rounded-sm bg-gray-400 h-3 w-3" />
                              <div class="rounded-sm bg-gray-400 h-3 w-3" />
                              <div class="rounded-sm bg-gray-400 h-3 w-3" />
                            </div>
                            <div class="p-1 bg-white flex-1 space-y-1">
                              <div class="rounded bg-gray-200 h-1.5" />
                              <div class="rounded bg-gray-200 h-1.5 w-3/4" />
                              <div class="rounded bg-gray-200 h-1.5 w-1/2" />
                            </div>
                          </div>
                        </template>
                        <!-- Dark preview -->
                        <template v-else-if="opt.value === ThemeType.DARK">
                          <div class="p-1 bg-zinc-800 flex flex-col gap-1 h-full w-6">
                            <div class="rounded-sm bg-zinc-600 h-3 w-3" />
                            <div class="rounded-sm bg-zinc-600 h-3 w-3" />
                            <div class="rounded-sm bg-zinc-600 h-3 w-3" />
                          </div>
                          <div class="p-2 bg-zinc-900 flex-1 space-y-1.5">
                            <div class="rounded bg-zinc-700 h-2" />
                            <div class="rounded bg-zinc-700 h-2 w-3/4" />
                            <div class="rounded bg-zinc-700 h-2 w-1/2" />
                            <div class="rounded bg-zinc-800 h-2 w-5/6" />
                          </div>
                        </template>
                        <!-- Light preview -->
                        <template v-else>
                          <div class="p-1 bg-gray-200 flex flex-col gap-1 h-full w-6">
                            <div class="rounded-sm bg-gray-400 h-3 w-3" />
                            <div class="rounded-sm bg-gray-400 h-3 w-3" />
                            <div class="rounded-sm bg-gray-400 h-3 w-3" />
                          </div>
                          <div class="p-2 bg-white flex-1 space-y-1.5">
                            <div class="rounded bg-gray-200 h-2" />
                            <div class="rounded bg-gray-200 h-2 w-3/4" />
                            <div class="rounded bg-gray-200 h-2 w-1/2" />
                            <div class="rounded bg-gray-100 h-2 w-5/6" />
                          </div>
                        </template>
                      </div>
                      <!-- Selected checkmark -->
                      <div
                        v-if="currentTheme === opt.value"
                        class="rounded-full bg-primary flex h-5 w-5 items-center right-2 top-2 justify-center absolute"
                      >
                        <span class="i-carbon-checkmark text-primary-foreground h-3 w-3" />
                      </div>
                      <p class="text-sm font-medium text-center">
                        {{ opt.label() }}
                      </p>
                    </button>
                  </div>

                  <!-- Check for Updates -->
                  <div class="flex items-center justify-between">
                    <div class="space-y-1">
                      <Label>{{ t('pages.settings.updates.checkForUpdates') }}</Label>
                      <p class="text-xs text-muted-foreground">
                        {{ t('pages.settings.updates.checkForUpdatesHelp') }}
                      </p>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      :disabled="isChecking || isInstalling"
                      @click="handleCheckUpdates"
                    >
                      <!-- Spinning circle while checking or installing -->
                      <span
                        v-if="isChecking || isInstalling"
                        class="i-carbon-circle-dash mr-2 shrink-0 h-4 w-4 animate-spin"
                      />
                      <!-- Circular progress ring while downloading -->
                      <svg
                        v-else-if="isDownloading && downloadProgress !== null"
                        class="mr-2 shrink-0 h-4 w-4 -rotate-90"
                        viewBox="0 0 16 16"
                      >
                        <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="2" opacity="0.15" />
                        <circle
                          cx="8" cy="8" r="6"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2"
                          stroke-dasharray="37.6991"
                          :stroke-dashoffset="37.6991 * (1 - downloadProgress / 100)"
                          stroke-linecap="round"
                        />
                      </svg>
                      {{ isChecking ? t('pages.settings.updates.checking') : isDownloading && downloadProgress !== null ? `${t('pages.settings.updates.downloading')} ${downloadProgress}%` : isInstalling ? t('pages.settings.updates.installing') : updateAvailable ? t('pages.settings.updates.updateAvailable') : t('pages.settings.updates.check') }}
                    </Button>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <!-- Editor Tab -->
          <TabsContent value="editor" class="mt-0">
            <Card>
              <CardHeader>
                <CardTitle>{{ t('pages.settings.editor.title') }}</CardTitle>
                <CardDescription>{{ t('pages.settings.editor.description') }}</CardDescription>
              </CardHeader>
              <CardContent class="space-y-5">
                <!-- Font Size -->
                <div class="gap-4 grid grid-cols-2 items-start">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.editor.fontSize') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.editor.fontSizeHelp') }}
                    </p>
                  </div>
                  <div class="space-y-1">
                    <Input
                      type="number"
                      :model-value="fontSizeInput"
                      min="8"
                      max="32"
                      class="w-28"
                      @update:model-value="handleFontSizeChange($event)"
                    />
                    <p v-if="fontSizeError" class="text-xs text-destructive">
                      {{ fontSizeError }}
                    </p>
                  </div>
                </div>

                <!-- Tab Size -->
                <div class="gap-4 grid grid-cols-2 items-start">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.editor.tabSize') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.editor.tabSizeHelp') }}
                    </p>
                  </div>
                  <div class="space-y-1">
                    <Input
                      type="number"
                      :model-value="tabSizeInput"
                      min="1"
                      max="8"
                      class="w-28"
                      @update:model-value="handleTabSizeChange($event)"
                    />
                    <p v-if="tabSizeError" class="text-xs text-destructive">
                      {{ tabSizeError }}
                    </p>
                  </div>
                </div>

                <!-- Indent Width -->
                <div class="gap-4 grid grid-cols-2 items-start">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.editor.indentWidth') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.editor.indentWidthHelp') }}
                    </p>
                  </div>
                  <div class="space-y-1">
                    <Input
                      type="number"
                      :model-value="indentWidthInput"
                      min="1"
                      max="8"
                      class="w-28"
                      @update:model-value="handleIndentWidthChange($event)"
                    />
                    <p v-if="indentWidthError" class="text-xs text-destructive">
                      {{ indentWidthError }}
                    </p>
                  </div>
                </div>

                <!-- Line Width -->
                <div class="gap-4 grid grid-cols-2 items-start">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.editor.lineWidth') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.editor.lineWidthHelp') }}
                    </p>
                  </div>
                  <div class="space-y-1">
                    <Input
                      type="number"
                      :model-value="lineWidthInput"
                      min="40"
                      max="200"
                      class="w-28"
                      @update:model-value="handleLineWidthChange($event)"
                    />
                    <p v-if="lineWidthError" class="text-xs text-destructive">
                      {{ lineWidthError }}
                    </p>
                  </div>
                </div>

                <!-- Word Wrap -->
                <div class="flex items-center justify-between">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.editor.wordWrap') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.editor.wordWrapHelp') }}
                    </p>
                  </div>
                  <button
                    class="border-2 border-transparent rounded-full inline-flex flex-shrink-0 h-6 w-11 cursor-pointer transition-colors relative focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    :class="wordWrap ? 'bg-primary' : 'bg-muted'"
                    role="switch"
                    :aria-checked="wordWrap"
                    @click="toggleWordWrap"
                  >
                    <span
                      class="rounded-full bg-background h-5 w-5 inline-block pointer-events-none shadow transition-transform"
                      :class="wordWrap ? 'translate-x-5' : 'translate-x-0'"
                    />
                  </button>
                </div>

                <!-- Show Line Numbers -->
                <div class="flex items-center justify-between">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.editor.showLineNumbers') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.editor.showLineNumbersHelp') }}
                    </p>
                  </div>
                  <button
                    class="border-2 border-transparent rounded-full inline-flex flex-shrink-0 h-6 w-11 cursor-pointer transition-colors relative focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    :class="showLineNumbers ? 'bg-primary' : 'bg-muted'"
                    role="switch"
                    :aria-checked="showLineNumbers"
                    @click="toggleLineNumbers"
                  >
                    <span
                      class="rounded-full bg-background h-5 w-5 inline-block pointer-events-none shadow transition-transform"
                      :class="showLineNumbers ? 'translate-x-5' : 'translate-x-0'"
                    />
                  </button>
                </div>

                <!-- Show Minimap -->
                <div class="flex items-center justify-between">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.editor.showMinimap') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.editor.showMinimapHelp') }}
                    </p>
                  </div>
                  <button
                    class="border-2 border-transparent rounded-full inline-flex flex-shrink-0 h-6 w-11 cursor-pointer transition-colors relative focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    :class="showMinimap ? 'bg-primary' : 'bg-muted'"
                    role="switch"
                    :aria-checked="showMinimap"
                    @click="toggleMinimap"
                  >
                    <span
                      class="rounded-full bg-background h-5 w-5 inline-block pointer-events-none shadow transition-transform"
                      :class="showMinimap ? 'translate-x-5' : 'translate-x-0'"
                    />
                  </button>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <!-- Query Tab -->
          <TabsContent value="query" class="mt-0">
            <Card>
              <CardHeader>
                <CardTitle>{{ t('pages.settings.query.title') }}</CardTitle>
                <CardDescription>{{ t('pages.settings.query.description') }}</CardDescription>
              </CardHeader>
              <CardContent class="space-y-5">
                <!-- Default Row Limit -->
                <div class="gap-4 grid grid-cols-2 items-start">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.query.defaultLimit') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.query.defaultLimitHelp') }}
                    </p>
                  </div>
                  <div class="space-y-1">
                    <Input
                      type="number"
                      :model-value="defaultLimitInput"
                      min="10"
                      max="10000"
                      class="w-32"
                      @update:model-value="handleDefaultLimitChange($event)"
                    />
                    <p v-if="defaultLimitError" class="text-xs text-destructive">
                      {{ defaultLimitError }}
                    </p>
                  </div>
                </div>

                <!-- Query Timeout -->
                <div class="gap-4 grid grid-cols-2 items-start">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.query.queryTimeout') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.query.queryTimeoutHelp') }}
                    </p>
                  </div>
                  <div class="space-y-1">
                    <Input
                      type="number"
                      :model-value="queryTimeoutInput"
                      min="1"
                      max="300"
                      class="w-32"
                      @update:model-value="handleQueryTimeoutChange($event)"
                    />
                    <p v-if="queryTimeoutError" class="text-xs text-destructive">
                      {{ queryTimeoutError }}
                    </p>
                  </div>
                </div>

                <!-- Auto Save -->
                <div class="flex items-center justify-between">
                  <div class="space-y-1">
                    <Label>{{ t('pages.settings.query.autoSave') }}</Label>
                    <p class="text-xs text-muted-foreground">
                      {{ t('pages.settings.query.autoSaveHelp') }}
                    </p>
                  </div>
                  <button
                    class="border-2 border-transparent rounded-full inline-flex flex-shrink-0 h-6 w-11 cursor-pointer transition-colors relative focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    :class="autoSave ? 'bg-primary' : 'bg-muted'"
                    role="switch"
                    :aria-checked="autoSave"
                    @click="toggleAutoSave"
                  >
                    <span
                      class="rounded-full bg-background h-5 w-5 inline-block pointer-events-none shadow transition-transform"
                      :class="autoSave ? 'translate-x-5' : 'translate-x-0'"
                    />
                  </button>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <!-- AI Tab -->
          <TabsContent value="ai" class="mt-0">
            <AiSettings />
          </TabsContent>

          <!-- JRE & Drivers Tab -->
          <TabsContent value="jre" class="mt-0">
            <JreDriverSection />
          </TabsContent>

          <!-- About Tab -->
          <TabsContent value="about" class="mt-0">
            <Card>
              <CardHeader>
                <CardTitle>{{ t('pages.settings.about.title') }}</CardTitle>
                <CardDescription>{{ t('pages.settings.about.description') }}</CardDescription>
              </CardHeader>
              <CardContent class="space-y-5">
                <!-- Version -->
                <div class="flex items-center justify-between">
                  <Label>{{ t('pages.settings.about.version') }}</Label>
                  <span class="text-sm text-muted-foreground font-mono">{{ version }}</span>
                </div>

                <!-- License -->
                <div class="flex items-center justify-between">
                  <Label>{{ t('pages.settings.about.license') }}</Label>
                  <span class="text-sm text-muted-foreground">{{ t('pages.settings.about.licenseName') }}</span>
                </div>

                <div class="border-t border-border/40" />

                <!-- Repository -->
                <a
                  href="https://github.com/geek-fun/sqlkit"
                  target="_blank"
                  class="p-2 rounded-md flex transition-colors items-center justify-between -mx-2 hover:bg-muted/50"
                >
                  <Label>{{ t('pages.settings.about.repository') }}</Label>
                  <div class="flex gap-2 items-center">
                    <span class="text-sm text-muted-foreground">geek-fun/sqlkit</span>
                    <span class="i-carbon-launch text-muted-foreground h-4 w-4" />
                  </div>
                </a>

                <!-- Website -->
                <a
                  href="https://www.geekfun.club"
                  target="_blank"
                  class="p-2 rounded-md flex transition-colors items-center justify-between -mx-2 hover:bg-muted/50"
                >
                  <Label>{{ t('pages.settings.about.website') }}</Label>
                  <div class="flex gap-2 items-center">
                    <span class="text-sm text-muted-foreground">geekfun.club</span>
                    <span class="i-carbon-launch text-muted-foreground h-4 w-4" />
                  </div>
                </a>

                <!-- Discord -->
                <a
                  href="https://discord.gg/5NSUyPK2E"
                  target="_blank"
                  class="p-2 rounded-md flex transition-colors items-center justify-between -mx-2 hover:bg-muted/50"
                >
                  <Label>{{ t('pages.settings.about.community') }}</Label>
                  <div class="flex gap-2 items-center">
                    <span class="text-sm text-muted-foreground">Discord</span>
                    <span class="i-carbon-launch text-muted-foreground h-4 w-4" />
                  </div>
                </a>

                <div class="border-t border-border/40" />

                <!-- Copyright -->
                <p class="text-xs text-muted-foreground pt-2 text-center">
                  {{ t('pages.settings.about.copyright', { year: new Date().getFullYear() }) }}
                </p>
              </CardContent>
            </Card>
          </TabsContent>
        </div>
      </Tabs>
    </div>
  </AppLayout>
</template>

<style scoped>
input[type='number']::-webkit-inner-spin-button,
input[type='number']::-webkit-outer-spin-button {
  cursor: pointer;
}
</style>
