<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AppLayout from '@/components/layout/AppLayout.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTheme } from '@/composables/useTheme'

const { theme, setTheme } = useTheme()
const { t, locale } = useI18n()

// Initialize language from localStorage or current locale
const currentLanguage = ref(localStorage.getItem('lang') || 'auto')

function handleThemeChange(value: string) {
  setTheme(value as 'light' | 'dark' | 'system')
}

function handleLanguageChange(value: string) {
  currentLanguage.value = value
  localStorage.setItem('lang', value)
  
  // Update locale
  if (value === 'auto') {
    locale.value = navigator.language === 'zh-CN' ? 'zhCN' : 'enUS'
  }
  else {
    locale.value = value
  }
}

</script>

<template>
  <AppLayout>
    <div class="p-6 h-full">
      <div class="space-y-6">
        <!-- Page Header -->
        <div class="flex gap-3 items-center">
          <h1 class="text-xl font-semibold">
            {{ t('pages.settings.title') }}
          </h1>
          <span class="text-muted-foreground">|</span>
          <span class="text-sm text-muted-foreground">{{ t('pages.settings.subtitle') }}</span>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>{{ t('pages.settings.appearance.title') }}</CardTitle>
            <CardDescription>
              {{ t('pages.settings.appearance.description') }}
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="space-y-2">
              <Label>{{ t('pages.settings.appearance.theme') }}</Label>
              <Select :model-value="theme" @update:model-value="handleThemeChange">
                <SelectTrigger class="w-48">
                  <SelectValue :placeholder="t('pages.settings.appearance.themeSelect')" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">
                    {{ t('pages.settings.appearance.themes.light') }}
                  </SelectItem>
                  <SelectItem value="dark">
                    {{ t('pages.settings.appearance.themes.dark') }}
                  </SelectItem>
                  <SelectItem value="system">
                    {{ t('pages.settings.appearance.themes.system') }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <p class="text-sm text-muted-foreground">
                {{ t('pages.settings.appearance.themeHelp') }}
              </p>
            </div>

            <div class="space-y-2">
              <Label>{{ t('pages.settings.appearance.language') }}</Label>
              <Select :model-value="currentLanguage" @update:model-value="handleLanguageChange">
                <SelectTrigger class="w-48">
                  <SelectValue :placeholder="t('pages.settings.appearance.languageSelect')" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="auto">
                    {{ t('pages.settings.appearance.languages.auto') }}
                  </SelectItem>
                  <SelectItem value="enUS">
                    {{ t('pages.settings.appearance.languages.enUS') }}
                  </SelectItem>
                  <SelectItem value="zhCN">
                    {{ t('pages.settings.appearance.languages.zhCN') }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <p class="text-sm text-muted-foreground">
                {{ t('pages.settings.appearance.languageHelp') }}
              </p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{{ t('pages.settings.preferences.title') }}</CardTitle>
            <CardDescription>
              {{ t('pages.settings.preferences.description') }}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p class="text-muted-foreground">
              {{ t('pages.settings.preferences.comingSoon') }}
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
