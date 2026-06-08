<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Keyboard, Save, Sun, Moon } from "lucide-vue-next";
import { useI18n } from "@/i18n/useI18n";
import type { Locale, TranslationKey } from "@/i18n/messages";
import { useTheme } from "@/theme/useTheme";
import type { Theme } from "@/theme/useTheme";

type SettingsTab = "general" | "personas" | "hotkeys";
type HotkeyField = "safe_mode" | "magic_mode";
type HotkeyConfig = {
  safe_mode: string;
  magic_mode: string;
};
type HotkeyStatus = "idle" | "saved" | "error";
type AppSettingsConfig = {
  api_key: string;
  base_url: string;
  model: string;
  timeout_ms: number;
};
type AppSettingsStatus = "idle" | "saved" | "error";

const { locale, locales, setLocale, t } = useI18n();
const { setTheme, theme, themes } = useTheme();

const navItems = [
  { id: "general", label: "settings.nav.general" },
  { id: "personas", label: "settings.nav.personas" },
  { id: "hotkeys", label: "settings.nav.hotkeys" },
] as const satisfies readonly { id: SettingsTab; label: TranslationKey }[];

const personas = [
  "persona.workplaceEq.name",
  "persona.academicConcise.name",
  "persona.cleanCorrection.name",
] as const satisfies readonly TranslationKey[];

const selectedTab = ref<SettingsTab>("general");
const appSettings = ref<AppSettingsConfig>({
  api_key: "",
  base_url: "",
  model: "",
  timeout_ms: 8000,
});
const settingsLoading = ref(false);
const settingsSaving = ref(false);
const settingsDirty = ref(false);
const settingsStatus = ref<AppSettingsStatus>("idle");
const settingsErrorKey = ref<TranslationKey | null>(null);
const hotkeyConfig = ref<HotkeyConfig>({
  safe_mode: "",
  magic_mode: "",
});
const hotkeysLoading = ref(false);
const hotkeysSaving = ref(false);
const hotkeysDirty = ref(false);
const hotkeyStatus = ref<HotkeyStatus>("idle");
const hotkeyErrorKey = ref<TranslationKey | null>(null);
const recordingHotkeyField = ref<HotkeyField | null>(null);

const localeLabels = computed<Record<Locale, string>>(() => ({
  "zh-CN": t("settings.locale.zh"),
  "en-US": t("settings.locale.en"),
}));

const themeLabels = computed<Record<Theme, string>>(() => ({
  light: t("settings.theme.light"),
  dark: t("settings.theme.dark"),
}));

const settingsErrorMessage = computed(() => (settingsErrorKey.value ? t(settingsErrorKey.value) : ""));
const hotkeyErrorMessage = computed(() => (hotkeyErrorKey.value ? t(hotkeyErrorKey.value) : ""));

const canSaveSettings = computed(
  () =>
    !settingsLoading.value &&
    !settingsSaving.value &&
    appSettings.value.base_url.trim() !== "",
);

const canSaveHotkeys = computed(
  () =>
    !hotkeysLoading.value &&
    !hotkeysSaving.value &&
    recordingHotkeyField.value === null &&
    hotkeyConfig.value.safe_mode.trim() !== "" &&
    hotkeyConfig.value.magic_mode.trim() !== "",
);

onMounted(() => {
  void loadAppSettings();
  void loadHotkeys();
});

onUnmounted(() => {
  void stopRecordingHotkey();
});

async function loadHotkeys() {
  hotkeysLoading.value = true;
  hotkeyErrorKey.value = null;

  try {
    hotkeyConfig.value = await invoke<HotkeyConfig>("get_hotkey_config");
    hotkeysDirty.value = false;
    hotkeyStatus.value = "idle";
  } catch (error) {
    hotkeyStatus.value = "error";
    hotkeyErrorKey.value = formatHotkeyError(error);
  } finally {
    hotkeysLoading.value = false;
  }
}

async function loadAppSettings() {
  settingsLoading.value = true;
  settingsErrorKey.value = null;

  try {
    appSettings.value = await invoke<AppSettingsConfig>("get_app_settings");
    settingsDirty.value = false;
    settingsStatus.value = "idle";
  } catch (error) {
    settingsStatus.value = "error";
    settingsErrorKey.value = formatSettingsError(error);
  } finally {
    settingsLoading.value = false;
  }
}

async function saveAppSettings() {
  if (!canSaveSettings.value) {
    return;
  }

  settingsSaving.value = true;
  settingsErrorKey.value = null;

  try {
    const savedSettings = await invoke<AppSettingsConfig>("save_app_settings", {
      settings: {
        api_key: appSettings.value.api_key.trim(),
        base_url: appSettings.value.base_url.trim(),
        model: appSettings.value.model.trim(),
        timeout_ms: Math.round(appSettings.value.timeout_ms),
      },
    });
    appSettings.value = savedSettings;
    settingsDirty.value = false;
    settingsStatus.value = "saved";
  } catch (error) {
    settingsStatus.value = "error";
    settingsErrorKey.value = formatSettingsError(error);
  } finally {
    settingsSaving.value = false;
  }
}

async function saveHotkeys() {
  if (!canSaveHotkeys.value) {
    return;
  }

  hotkeysSaving.value = true;
  hotkeyErrorKey.value = null;

  const nextHotkeys = {
    safe_mode: hotkeyConfig.value.safe_mode.trim(),
    magic_mode: hotkeyConfig.value.magic_mode.trim(),
  };

  if (nextHotkeys.safe_mode === nextHotkeys.magic_mode) {
    hotkeysSaving.value = false;
    hotkeyStatus.value = "error";
    hotkeyErrorKey.value = "settings.hotkeys.duplicate";
    return;
  }

  try {
    const savedConfig = await invoke<HotkeyConfig>("save_hotkey_config", {
      hotkeys: nextHotkeys,
    });
    hotkeyConfig.value = savedConfig;
    hotkeysDirty.value = false;
    hotkeyStatus.value = "saved";
  } catch (error) {
    hotkeyStatus.value = "error";
    hotkeyErrorKey.value = formatHotkeyError(error);
  } finally {
    hotkeysSaving.value = false;
  }
}

function markHotkeysDirty() {
  hotkeysDirty.value = true;
  if (hotkeyStatus.value === "saved") {
    hotkeyStatus.value = "idle";
  }
}

function markSettingsDirty() {
  settingsDirty.value = true;
  if (settingsStatus.value === "saved") {
    settingsStatus.value = "idle";
  }
}

async function startRecordingHotkey(field: HotkeyField) {
  await stopRecordingHotkey();

  try {
    await setHotkeyRecordingActive(true);
  } catch (error) {
    hotkeyStatus.value = "error";
    hotkeyErrorKey.value = formatHotkeyError(error);
    return;
  }

  recordingHotkeyField.value = field;
  hotkeyStatus.value = "idle";
  hotkeyErrorKey.value = null;
  window.addEventListener("keydown", recordHotkey, true);
}

async function stopRecordingHotkey() {
  const wasRecording = recordingHotkeyField.value !== null;
  window.removeEventListener("keydown", recordHotkey, true);
  recordingHotkeyField.value = null;

  if (wasRecording) {
    await setHotkeyRecordingActive(false);
  }
}

async function setHotkeyRecordingActive(active: boolean) {
  await invoke("set_hotkey_recording_active", { active });
}

function recordHotkey(event: KeyboardEvent) {
  if (!recordingHotkeyField.value) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();

  if (event.code === "Escape") {
    void stopRecordingHotkey();
    hotkeyStatus.value = "idle";
    hotkeyErrorKey.value = null;
    return;
  }

  if (isModifierCode(event.code)) {
    return;
  }

  const key = keyFromCode(event.code);
  if (!key) {
    hotkeyStatus.value = "error";
    hotkeyErrorKey.value = "settings.hotkeys.unsupportedKey";
    return;
  }

  const modifiers = modifiersFromEvent(event);
  if (modifiers.length === 0) {
    hotkeyStatus.value = "error";
    hotkeyErrorKey.value = "settings.hotkeys.needModifier";
    return;
  }

  hotkeyConfig.value[recordingHotkeyField.value] = [...modifiers, key].join("+");
  markHotkeysDirty();
  void stopRecordingHotkey();
}

function modifiersFromEvent(event: KeyboardEvent) {
  const modifiers: string[] = [];

  if (event.ctrlKey) {
    modifiers.push("Ctrl");
  }
  if (event.shiftKey) {
    modifiers.push("Shift");
  }
  if (event.altKey) {
    modifiers.push("Alt");
  }
  if (event.metaKey) {
    modifiers.push(isApplePlatform() ? "Cmd" : "Super");
  }

  return modifiers;
}

function isModifierCode(code: string) {
  return (
    code === "ControlLeft" ||
    code === "ControlRight" ||
    code === "ShiftLeft" ||
    code === "ShiftRight" ||
    code === "AltLeft" ||
    code === "AltRight" ||
    code === "MetaLeft" ||
    code === "MetaRight"
  );
}

function keyFromCode(code: string) {
  if (/^Key[A-Z]$/.test(code) || /^Digit[0-9]$/.test(code) || /^F([1-9]|1[0-9]|2[0-4])$/.test(code)) {
    return code;
  }

  const codeMap: Record<string, string> = {
    Backquote: "Backquote",
    Backslash: "Backslash",
    BracketLeft: "BracketLeft",
    BracketRight: "BracketRight",
    Backspace: "Backspace",
    CapsLock: "CapsLock",
    Comma: "Comma",
    Delete: "Delete",
    End: "End",
    Enter: "Enter",
    Equal: "Equal",
    Home: "Home",
    Insert: "Insert",
    Minus: "Minus",
    PageDown: "PageDown",
    PageUp: "PageUp",
    Period: "Period",
    PrintScreen: "PrintScreen",
    Quote: "Quote",
    ScrollLock: "ScrollLock",
    Semicolon: "Semicolon",
    Slash: "Slash",
    Space: "Space",
    Tab: "Tab",
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
    ArrowUp: "ArrowUp",
    NumLock: "NumLock",
    Numpad0: "Numpad0",
    Numpad1: "Numpad1",
    Numpad2: "Numpad2",
    Numpad3: "Numpad3",
    Numpad4: "Numpad4",
    Numpad5: "Numpad5",
    Numpad6: "Numpad6",
    Numpad7: "Numpad7",
    Numpad8: "Numpad8",
    Numpad9: "Numpad9",
    NumpadAdd: "NumpadAdd",
    NumpadDecimal: "NumpadDecimal",
    NumpadDivide: "NumpadDivide",
    NumpadEnter: "NumpadEnter",
    NumpadEqual: "NumpadEqual",
    NumpadMultiply: "NumpadMultiply",
    NumpadSubtract: "NumpadSubtract",
    AudioVolumeDown: "AudioVolumeDown",
    AudioVolumeMute: "AudioVolumeMute",
    AudioVolumeUp: "AudioVolumeUp",
    MediaPlayPause: "MediaPlayPause",
    MediaStop: "MediaStop",
    MediaTrackNext: "MediaTrackNext",
    MediaTrackPrevious: "MediaTrackPrevious",
  };

  return codeMap[code] ?? null;
}

function isApplePlatform() {
  return /Mac|iPhone|iPad|iPod/.test(navigator.platform);
}

function formatHotkeyError(error: unknown): TranslationKey {
  const message = error instanceof Error ? error.message : String(error);

  if (message.includes("safe and magic hotkeys must be different")) {
    return "settings.hotkeys.duplicate";
  }
  if (message.includes("invalid safe_mode hotkey")) {
    return "settings.hotkeys.invalidSafeMode";
  }
  if (message.includes("invalid magic_mode hotkey")) {
    return "settings.hotkeys.invalidMagicMode";
  }
  if (message.includes("AlreadyRegistered") || message.toLowerCase().includes("already registered")) {
    return "settings.hotkeys.alreadyRegistered";
  }
  if (message.includes("failed to register new hotkeys") || message.includes("failed to register")) {
    return "settings.hotkeys.registerFailed";
  }
  if (message.includes("failed to load") || message.includes("failed to read") || message.includes("failed to parse")) {
    return "settings.hotkeys.loadFailed";
  }

  return "settings.hotkeys.unknownError";
}

function formatSettingsError(error: unknown): TranslationKey {
  const message = error instanceof Error ? error.message : String(error);

  if (message.includes("base_url cannot be empty")) {
    return "settings.general.invalidBaseUrl";
  }

  return "settings.general.unknownError";
}
</script>

<template>
  <section class="mx-auto grid min-h-screen max-w-5xl grid-cols-[220px_1fr]">
    <aside class="border-r border-shanka-border px-3 py-5">
      <div class="mb-5 px-2 text-sm font-medium text-shanka-primary">{{ t("app.name") }}</div>
      <nav class="space-y-1">
        <button
          v-for="item in navItems"
          :key="item.id"
          class="flex h-9 w-full items-center rounded-md px-2 text-left text-sm transition"
          :class="selectedTab === item.id ? 'bg-shanka-hover/5 text-shanka-primary' : 'text-shanka-muted hover:bg-shanka-hover/5 hover:text-shanka-primary'"
          type="button"
          @click="selectedTab = item.id"
        >
          {{ t(item.label) }}
        </button>
      </nav>
    </aside>

    <section class="px-8 py-6">
      <div class="max-w-2xl">
        <div class="flex flex-wrap items-center justify-between gap-4">
          <h1 class="text-xl font-semibold text-shanka-primary">{{ t("settings.title") }}</h1>

          <div class="flex flex-wrap items-center justify-end gap-3">
            <div class="flex items-center gap-2">
              <span class="text-xs text-shanka-muted">{{ t("settings.appearance") }}</span>
              <div class="flex rounded-md border border-shanka-border bg-shanka-panel p-0.5">
                <button
                  v-for="themeOption in themes"
                  :key="themeOption"
                  class="flex size-7 items-center justify-center rounded transition"
                  :class="theme === themeOption ? 'bg-shanka-hover/10 text-shanka-primary' : 'text-shanka-muted hover:text-shanka-primary'"
                  :aria-label="themeLabels[themeOption]"
                  :title="themeLabels[themeOption]"
                  type="button"
                  @click="setTheme(themeOption)"
                >
                  <Sun v-if="themeOption === 'light'" class="size-3.5" aria-hidden="true" />
                  <Moon v-else class="size-3.5" aria-hidden="true" />
                </button>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <span class="text-xs text-shanka-muted">{{ t("settings.language") }}</span>
              <div class="flex rounded-md border border-shanka-border bg-shanka-panel p-0.5">
                <button
                  v-for="localeOption in locales"
                  :key="localeOption"
                  class="h-7 rounded px-2 text-xs transition"
                  :class="locale === localeOption ? 'bg-shanka-hover/10 text-shanka-primary' : 'text-shanka-muted hover:text-shanka-primary'"
                  type="button"
                  @click="setLocale(localeOption)"
                >
                  {{ localeLabels[localeOption] }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <form v-if="selectedTab === 'general'" class="mt-6 space-y-5" @submit.prevent="saveAppSettings">
          <label class="block">
            <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.field.apiKey") }}</span>
            <input
              v-model.trim="appSettings.api_key"
              class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
              :disabled="settingsLoading"
              placeholder="sk-..."
              type="password"
              @input="markSettingsDirty"
            />
          </label>

          <label class="block">
            <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.field.baseUrl") }}</span>
            <input
              v-model.trim="appSettings.base_url"
              class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
              :disabled="settingsLoading"
              placeholder="https://api.openai.com/v1"
              type="url"
              @input="markSettingsDirty"
            />
          </label>

          <label class="block">
            <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.field.model") }}</span>
            <input
              v-model.trim="appSettings.model"
              class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
              :disabled="settingsLoading"
              placeholder="gpt-4.1-mini"
              type="text"
              @input="markSettingsDirty"
            />
          </label>

          <label class="block">
            <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.field.timeoutMs") }}</span>
            <input
              v-model.number="appSettings.timeout_ms"
              class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
              :disabled="settingsLoading"
              max="120000"
              min="1000"
              step="1000"
              type="number"
              @input="markSettingsDirty"
            />
          </label>

          <div class="flex flex-wrap items-center justify-between gap-3">
            <p v-if="settingsLoading" class="text-xs text-shanka-muted">
              {{ t("settings.general.loading") }}
            </p>
            <p v-else-if="settingsStatus === 'error'" class="text-xs text-red-500 dark:text-red-400">
              {{ t("settings.general.errorPrefix") }}: {{ settingsErrorMessage }}
            </p>
            <p v-else-if="settingsStatus === 'saved'" class="text-xs text-shanka-success">
              {{ t("settings.general.saved") }}
            </p>
            <p v-else class="text-xs text-shanka-muted">
              {{ t("settings.general.mockFallback") }}
            </p>

            <button
              class="inline-flex h-9 items-center gap-2 rounded-md bg-shanka-primary px-3 text-sm text-shanka-canvas transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="!canSaveSettings || (!settingsDirty && settingsStatus === 'saved')"
              type="submit"
            >
              <Save class="size-4" aria-hidden="true" />
              <span>{{ settingsSaving ? t("settings.general.saving") : t("settings.general.save") }}</span>
            </button>
          </div>
        </form>

        <div v-else-if="selectedTab === 'personas'" class="mt-6">
          <div class="mb-2 text-sm text-shanka-secondary">{{ t("settings.field.activePersona") }}</div>
          <div class="divide-y divide-shanka-border rounded-md border border-shanka-border">
            <button
              v-for="(persona, index) in personas"
              :key="persona"
              class="flex h-11 w-full items-center justify-between px-3 text-sm text-shanka-secondary transition hover:bg-shanka-hover/5"
              type="button"
            >
              <span>{{ t(persona) }}</span>
              <span v-if="index === 0" class="text-xs text-shanka-success">
                {{ t("settings.status.active") }}
              </span>
            </button>
          </div>
        </div>

        <form v-else class="mt-6 space-y-5" @submit.prevent="saveHotkeys">
          <div class="space-y-2">
            <div class="text-sm text-shanka-secondary">{{ t("settings.hotkeys.safeMode") }}</div>
            <div class="flex gap-2">
              <button
                class="flex h-10 flex-1 items-center rounded-md border border-transparent bg-shanka-input px-3 text-left font-mono text-sm text-shanka-primary outline-none transition focus:border-shanka-focus disabled:cursor-not-allowed disabled:opacity-60"
                :disabled="hotkeysLoading"
                type="button"
                @click="startRecordingHotkey('safe_mode')"
              >
                {{ recordingHotkeyField === "safe_mode" ? t("settings.hotkeys.recording") : hotkeyConfig.safe_mode }}
              </button>
              <button
                class="inline-flex h-10 min-w-24 items-center justify-center gap-2 rounded-md border border-shanka-border px-3 text-sm text-shanka-secondary transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-60"
                :disabled="hotkeysLoading"
                type="button"
                @click="recordingHotkeyField === 'safe_mode' ? stopRecordingHotkey() : startRecordingHotkey('safe_mode')"
              >
                <Keyboard class="size-4" aria-hidden="true" />
                <span>{{ recordingHotkeyField === "safe_mode" ? t("settings.hotkeys.cancelRecord") : t("settings.hotkeys.record") }}</span>
              </button>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-sm text-shanka-secondary">{{ t("settings.hotkeys.magicMode") }}</div>
            <div class="flex gap-2">
              <button
                class="flex h-10 flex-1 items-center rounded-md border border-transparent bg-shanka-input px-3 text-left font-mono text-sm text-shanka-primary outline-none transition focus:border-shanka-focus disabled:cursor-not-allowed disabled:opacity-60"
                :disabled="hotkeysLoading"
                type="button"
                @click="startRecordingHotkey('magic_mode')"
              >
                {{ recordingHotkeyField === "magic_mode" ? t("settings.hotkeys.recording") : hotkeyConfig.magic_mode }}
              </button>
              <button
                class="inline-flex h-10 min-w-24 items-center justify-center gap-2 rounded-md border border-shanka-border px-3 text-sm text-shanka-secondary transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-60"
                :disabled="hotkeysLoading"
                type="button"
                @click="recordingHotkeyField === 'magic_mode' ? stopRecordingHotkey() : startRecordingHotkey('magic_mode')"
              >
                <Keyboard class="size-4" aria-hidden="true" />
                <span>{{ recordingHotkeyField === "magic_mode" ? t("settings.hotkeys.cancelRecord") : t("settings.hotkeys.record") }}</span>
              </button>
            </div>
          </div>

          <div class="flex flex-wrap items-center justify-between gap-3">
            <p v-if="hotkeysLoading" class="text-xs text-shanka-muted">
              {{ t("settings.hotkeys.loading") }}
            </p>
            <p v-else-if="hotkeyStatus === 'error'" class="text-xs text-red-500 dark:text-red-400">
              {{ t("settings.hotkeys.errorPrefix") }}: {{ hotkeyErrorMessage }}
            </p>
            <p v-else-if="hotkeyStatus === 'saved'" class="text-xs text-shanka-success">
              {{ t("settings.hotkeys.saved") }}
            </p>
            <p v-else class="text-xs text-shanka-muted">
              {{ t("settings.hotkeys.appliesImmediately") }}
            </p>

            <button
              class="inline-flex h-9 items-center gap-2 rounded-md bg-shanka-primary px-3 text-sm text-shanka-canvas transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="!canSaveHotkeys || (!hotkeysDirty && hotkeyStatus === 'saved')"
              type="submit"
            >
              <Save class="size-4" aria-hidden="true" />
              <span>{{ hotkeysSaving ? t("settings.hotkeys.saving") : t("settings.hotkeys.save") }}</span>
            </button>
          </div>
        </form>
      </div>
    </section>
  </section>
</template>
