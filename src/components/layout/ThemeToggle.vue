<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { useTheme } from '@/composables/useTheme'

const { isDark, toggleTheme } = useTheme()
</script>

<template>
  <Button variant="ghost" size="icon" class="h-8 w-8" @click="toggleTheme">
    <span v-if="isDark" class="i-carbon-sun h-4 w-4" />
    <span v-else class="i-carbon-moon h-4 w-4" />
  </Button>
</template>
