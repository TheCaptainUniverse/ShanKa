<script setup lang="ts">
import HudPill from "@/components/hud/HudPill.vue";
import SettingsPanel from "@/components/settings/SettingsPanel.vue";
</script>

<template>
  <main class="min-h-screen bg-shanka-canvas text-zinc-100">
    <SettingsPanel />
    <HudPill class="fixed right-6 top-6" />
  </main>
</template>
