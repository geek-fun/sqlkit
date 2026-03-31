<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AppLayout from '@/components/layout/AppLayout.vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { LanguageType, ThemeType, useAppStore } from '@/store/appStore'

const LANG_TYPE_MAP: Record<string, LanguageType> = {
  auto: LanguageType.AUTO,
  enUS: LanguageType.EN_US,
  zhCN: LanguageType.ZH_CN,
}

const appStore = useAppStore()
const { t, locale } = useI18n()
const { checkForUpdates, downloadAndInstall: _dl, isChecking, isDownloading, isInstalling, updateAvailable } = useAppUpdater()

// --- Theme ---
const currentTheme = computed(() => appStore.themeType)

const themeOptions = [
  { value: ThemeType.AUTO, label: () => t('pages.settings.appearance.themes.system') },
  { value: ThemeType.DARK, label: () => t('pages.settings.appearance.themes.dark') },
  { value: ThemeType.LIGHT, label: () => t('pages.settings.appearance.themes.light') },
]

const handleThemeChange = (theme: ThemeType) => appStore.setThemeType(theme)

// --- Language ---
const _currentLanguage = computed(() => appStore.languageType as string)

const _languageOptions = [
  { value: 'auto', label: () => t('pages.settings.appearance.languages.auto') },
  { value: 'zhCN', label: () => t('pages.settings.appearance.languages.zhCN') },
  { value: 'enUS', label: () => t('pages.settings.appearance.languages.enUS') },
]

function _handleLanguageChange(value: string) {
  localStorage.setItem('lang', value)
  appStore.setLanguageType(LANG_TYPE_MAP[value] ?? LanguageType.AUTO)
  locale.value = value === 'auto'
    ? (navigator.language === 'zh-CN' ? 'zhCN' : 'enUS')
    : value
}

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

const fontFamilyInput = ref(appStore.editorConfig.fontFamily)

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
  await checkForUpdates(true)
  // The composable handles showing notifications and update info
}
</script>

<template>
  <AppLayout>
    <div class="flex flex-col h-full">
      <!-- Sticky header -->
      <div class="px-6 py-4 border-b bg-background top-0 sticky z-10">
        <div class="mx-auto flex gap-3 max-w-3xl items-center">
          <h1 class="text-xl font-semibold">
            {{ t('pages.settings.title') }}
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">{{ t('pages.settings.subtitle') }}</span>
        </div>
      </div>

      <!-- Scrollable content -->
      <div class="px-6 py-6 flex-1 overflow-y-auto">
        <div class="mx-auto max-w-3xl space-y-8">
          <!-- Appearance Card -->
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
                      <svg class="text-primary-foreground h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
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
                    :disabled="isChecking || isDownloading || isInstalling"
                    @click="handleCheckUpdates"
                  >
                    <svg
                      v-if="isChecking || isDownloading || isInstalling"
                      class="mr-2 h-4 w-4 animate-spin"
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                    >
                      <circle
                        class="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        stroke-width="4"
                      />
                      <path
                        class="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                      />
                    </svg>
                    {{ isChecking ? t('pages.settings.updates.checking') : isDownloading ? t('pages.settings.updates.downloading') : isInstalling ? t('pages.settings.updates.installing') : updateAvailable ? t('pages.settings.updates.updateAvailable') : t('pages.settings.updates.check') }}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>

          <!-- Editor Card -->
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

              <!-- Font Family -->
              <div class="gap-4 grid grid-cols-2 items-start">
                <div class="space-y-1">
                  <Label>{{ t('pages.settings.editor.fontFamily') }}</Label>
                  <p class="text-xs text-muted-foreground">
                    {{ t('pages.settings.editor.fontFamilyHelp') }}
                  </p>
                </div>
                <p class="text-sm text-muted-foreground font-mono px-3 py-2 border border-border rounded-md bg-muted cursor-default select-none">
                  {{ fontFamilyInput }}
                </p>
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

          <!-- Query Card -->
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
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
input[type='number']::-webkit-inner-spin-button,
input[type='number']::-webkit-outer-spin-button {
  cursor: pointer;
}
</style>
