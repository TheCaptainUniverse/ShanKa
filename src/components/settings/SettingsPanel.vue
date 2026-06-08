<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "@/i18n/useI18n";
import type { Locale, TranslationKey } from "@/i18n/messages";

const { locale, locales, setLocale, t } = useI18n();

const navItems = [
  "settings.nav.general",
  "settings.nav.personas",
  "settings.nav.hotkeys",
] as const satisfies readonly TranslationKey[];

const personas = [
  "persona.workplaceEq.name",
  "persona.academicConcise.name",
  "persona.cleanCorrection.name",
] as const satisfies readonly TranslationKey[];

const localeLabels = computed<Record<Locale, string>>(() => ({
  "zh-CN": t("settings.locale.zh"),
  "en-US": t("settings.locale.en"),
}));
</script>

<template>
  <section class="mx-auto grid min-h-screen max-w-5xl grid-cols-[220px_1fr]">
    <aside class="border-r border-shanka-border px-3 py-5">
      <div class="mb-5 px-2 text-sm font-medium text-white">{{ t("app.name") }}</div>
      <nav class="space-y-1">
        <button
          v-for="(item, index) in navItems"
          :key="item"
          class="flex h-9 w-full items-center rounded-md px-2 text-left text-sm transition"
          :class="index === 0 ? 'bg-white/5 text-white' : 'text-zinc-400 hover:bg-white/5 hover:text-white'"
          type="button"
        >
          {{ t(item) }}
        </button>
      </nav>
    </aside>

    <section class="px-8 py-6">
      <div class="max-w-2xl">
        <div class="flex items-center justify-between gap-4">
          <h1 class="text-xl font-semibold text-white">{{ t("settings.title") }}</h1>

          <div class="flex items-center gap-2">
            <span class="text-xs text-zinc-400">{{ t("settings.language") }}</span>
            <div class="flex rounded-md border border-shanka-border bg-shanka-panel p-0.5">
              <button
                v-for="localeOption in locales"
                :key="localeOption"
                class="h-7 rounded px-2 text-xs transition"
                :class="locale === localeOption ? 'bg-white/10 text-white' : 'text-zinc-400 hover:text-white'"
                type="button"
                @click="setLocale(localeOption)"
              >
                {{ localeLabels[localeOption] }}
              </button>
            </div>
          </div>
        </div>

        <div class="mt-6 space-y-5">
          <label class="block">
            <span class="mb-2 block text-sm text-zinc-300">{{ t("settings.field.apiKey") }}</span>
            <input
              class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-white outline-none focus:border-white"
              placeholder="sk-..."
              type="password"
            />
          </label>

          <label class="block">
            <span class="mb-2 block text-sm text-zinc-300">{{ t("settings.field.baseUrl") }}</span>
            <input
              class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-white outline-none focus:border-white"
              placeholder="https://api.openai.com/v1"
              type="url"
            />
          </label>

          <div>
            <div class="mb-2 text-sm text-zinc-300">{{ t("settings.field.activePersona") }}</div>
            <div class="divide-y divide-shanka-border rounded-md border border-shanka-border">
              <button
                v-for="(persona, index) in personas"
                :key="persona"
                class="flex h-11 w-full items-center justify-between px-3 text-sm text-zinc-200 hover:bg-white/5"
                type="button"
              >
                <span>{{ t(persona) }}</span>
                <span v-if="index === 0" class="text-xs text-shanka-success">
                  {{ t("settings.status.active") }}
                </span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>
  </section>
</template>
