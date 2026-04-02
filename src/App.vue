<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { RouterView } from "vue-router";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import AppNotifications from "@/components/ui/notification/AppNotifications.vue";
import { useAppUpdater } from "@/composables/useAppUpdater";
import { useAppStore } from "@/store/appStore";
import { useAccountStore } from "@/store/accountStore";

const appStore = useAppStore();
const accountStore = useAccountStore();
const { checkForUpdates } = useAppUpdater();

let unlistenAuth: UnlistenFn | null = null;

onMounted(async () => {
  appStore.setThemeType(appStore.themeType);
  checkForUpdates(false).then((update) => {
    if (update) {
    }
  });

  unlistenAuth = await listen<{
    token: string;
    username: string;
    email: string;
  }>("sqlkit://auth", ({ payload }) => {
    accountStore.setAuth(payload.token, payload.username, payload.email);
  });
});

onUnmounted(() => {
  unlistenAuth?.();
});
</script>

<template>
  <RouterView />
  <AppNotifications />
</template>
