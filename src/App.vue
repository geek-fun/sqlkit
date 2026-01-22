<script setup lang="ts">
import { RouterView } from 'vue-router'
import { useTheme } from '@/composables/useTheme'

useTheme()
</script>

<template>
  <RouterView />
</template>
