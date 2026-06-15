<script setup lang="ts">
import type { DriverInfo, JreStatus } from '@/datasources'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { jdbcApi } from '@/datasources'

const { t } = useI18n()

const jreStatus = ref<JreStatus | null>(null)
const jreLoading = ref(false)

const drivers = ref<DriverInfo[]>([])
const driversLoading = ref(false)
const downloadingDriver = ref<string | null>(null)
const removingDriver = ref<string | null>(null)

async function loadJreStatus() {
  jreLoading.value = true
  try {
    jreStatus.value = await jdbcApi.checkJreStatus()
  }
  finally {
    jreLoading.value = false
  }
}

async function loadDrivers() {
  driversLoading.value = true
  try {
    drivers.value = await jdbcApi.listDrivers()
  }
  finally {
    driversLoading.value = false
  }
}

async function handleDownloadJre() {
  jreLoading.value = true
  try {
    await jdbcApi.downloadJre()
    await loadJreStatus()
  }
  finally {
    jreLoading.value = false
  }
}

async function handleRemoveJre() {
  jreLoading.value = true
  try {
    await jdbcApi.removeJre()
    await loadJreStatus()
  }
  finally {
    jreLoading.value = false
  }
}

async function handleDownloadDriver(dbType: string) {
  downloadingDriver.value = dbType
  try {
    await jdbcApi.downloadDriver(dbType)
    await loadDrivers()
  }
  finally {
    downloadingDriver.value = null
  }
}

async function handleRemoveDriver(dbType: string) {
  removingDriver.value = dbType
  try {
    await jdbcApi.removeDriver(dbType)
    await loadDrivers()
  }
  finally {
    removingDriver.value = null
  }
}

onMounted(() => {
  loadJreStatus()
  loadDrivers()
})
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>{{ t('pages.settings.jre.title') }}</CardTitle>
      <CardDescription>{{ t('pages.settings.jre.description') }}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-6">
      <!-- JRE Status -->
      <div class="gap-4 grid grid-cols-2 items-start">
        <div class="space-y-1">
          <p class="text-sm font-medium">
            {{ t('pages.settings.jre.jreCard.title') }}
          </p>
          <p class="text-xs text-muted-foreground">
            {{ t('pages.settings.jre.jreCard.description') }}
          </p>
        </div>
        <div class="space-y-2">
          <div v-if="jreLoading && !jreStatus" class="text-sm text-muted-foreground">
            {{ t('pages.settings.jre.jreCard.status.checking') }}
          </div>
          <div v-else-if="jreStatus" class="flex gap-2 items-center">
            <span v-if="jreStatus.installed" class="text-sm text-green-600">✅ {{ t('pages.settings.jre.jreCard.status.installed') }}</span>
            <span v-else class="text-sm text-muted-foreground">⬇️ {{ t('pages.settings.jre.jreCard.status.notInstalled') }}</span>
            <span class="text-xs text-muted-foreground">
              ({{ jreStatus.source === 'managed' ? t('pages.settings.jre.jreCard.status.managed') : jreStatus.source === 'system' ? t('pages.settings.jre.jreCard.status.system') : '' }})
            </span>
          </div>
          <div class="flex gap-2">
            <Button
              v-if="!jreStatus?.installed"
              size="sm"
              variant="outline"
              :disabled="jreLoading"
              @click="handleDownloadJre"
            >
              {{ t('pages.settings.jre.jreCard.actions.download') }}
            </Button>
            <Button
              v-if="jreStatus?.installed && jreStatus?.source === 'managed'"
              size="sm"
              variant="outline"
              :disabled="jreLoading"
              @click="handleRemoveJre"
            >
              {{ t('pages.settings.jre.jreCard.actions.remove') }}
            </Button>
          </div>
        </div>
      </div>

      <!-- Driver List -->
      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <div class="space-y-1">
            <p class="text-sm font-medium">
              {{ t('pages.settings.jre.driversCard.title') }}
            </p>
            <p class="text-xs text-muted-foreground">
              {{ t('pages.settings.jre.driversCard.description') }}
            </p>
          </div>
        </div>

        <div v-if="driversLoading && drivers.length === 0" class="text-sm text-muted-foreground py-4 text-center">
          {{ t('pages.settings.jre.driversCard.status.loading') }}
        </div>

        <div v-else-if="drivers.length === 0" class="py-6 text-center border rounded-md">
          <p class="text-sm text-muted-foreground">
            {{ t('pages.settings.jre.driversCard.empty.title') }}
          </p>
          <p class="text-xs text-muted-foreground mt-1">
            {{ t('pages.settings.jre.driversCard.empty.message') }}
          </p>
        </div>

        <div v-else class="border rounded-md divide-y">
          <div
            v-for="driver in drivers"
            :key="driver.db_type"
            class="px-4 py-3 flex gap-3 items-center"
          >
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium truncate">
                {{ driver.name }}
              </p>
              <p class="text-xs text-muted-foreground">
                {{ driver.driver_count }} {{ t('pages.settings.jre.driversCard.label.versions') }}
              </p>
            </div>
            <div class="flex-shrink-0">
              <span
                v-if="driver.installed"
                class="text-xs text-green-600"
              >✅ {{ t('pages.settings.jre.driversCard.status.installed') }}</span>
              <span
                v-else
                class="text-xs text-muted-foreground"
              >⬇️ {{ t('pages.settings.jre.driversCard.status.notInstalled') }}</span>
            </div>
            <div class="flex flex-shrink-0 gap-2">
              <Button
                v-if="!driver.installed"
                size="sm"
                variant="outline"
                :disabled="downloadingDriver === driver.db_type"
                @click="handleDownloadDriver(driver.db_type)"
              >
                {{ t('pages.settings.jre.driversCard.actions.download') }}
              </Button>
              <Button
                v-if="driver.installed"
                size="sm"
                variant="outline"
                :disabled="removingDriver === driver.db_type"
                @click="handleRemoveDriver(driver.db_type)"
              >
                {{ t('pages.settings.jre.driversCard.actions.remove') }}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
