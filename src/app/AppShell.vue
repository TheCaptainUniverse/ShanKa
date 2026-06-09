<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import HudPill from "@/components/hud/HudPill.vue";
import SettingsPanel from "@/components/settings/SettingsPanel.vue";
import { useTheme } from "@/theme/useTheme";

const currentWindowLabel = getCurrentWindow().label;
const isHudWindow = currentWindowLabel === "hud";

document.documentElement.dataset.window = currentWindowLabel;
useTheme();
</script>

<template>
  <main
    v-if="isHudWindow"
    class="flex min-h-screen items-center justify-center bg-transparent p-2 text-shanka-primary"
  >
    <HudPill />
  </main>

  <main v-else class="min-h-screen bg-shanka-canvas text-shanka-primary transition-colors">
    <SettingsPanel />
  </main>
</template>
