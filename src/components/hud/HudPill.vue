<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "@/i18n/useI18n";
import type { HudStatus } from "@shared";

const { t } = useI18n();
const previewStatus: HudStatus = "refining";

const message = computed(() => {
  if (previewStatus === "refining") {
    return t("hud.refining");
  }

  return t("hud.ready");
});
</script>

<template>
  <div
    class="inline-flex h-9 items-center gap-2 rounded-md border border-shanka-border bg-shanka-panel px-3 text-xs text-shanka-secondary shadow-xl transition-colors"
  >
    <span
      class="size-3 animate-spin rounded-full border border-shanka-muted/30 border-t-shanka-primary"
      aria-hidden="true"
    />
    <span>{{ message }}</span>
  </div>
</template>
