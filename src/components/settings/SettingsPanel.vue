<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Copy, Keyboard, Pencil, Plus, Power, Save, Star, Sun, Moon, Trash2, X } from "lucide-vue-next";
import { useI18n } from "@/i18n/useI18n";
import type { Locale, TranslationKey } from "@/i18n/messages";
import { useTheme } from "@/theme/useTheme";
import type { Theme } from "@/theme/useTheme";
import { BUILT_IN_PERSONAS, DEFAULT_SAFE_PERSONA_ID, type PersonaConfig, type PersonaDefinition } from "@shared";

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
type PersonaStatus = "idle" | "saved" | "error";
type PersonaDraftMode = "create" | "edit";
type PersonaDraft = {
  mode: PersonaDraftMode;
  originalId: string | null;
  item: PersonaDefinition;
};

const { locale, locales, setLocale, t } = useI18n();
const { setTheme, theme, themes } = useTheme();

const navItems = [
  { id: "general", label: "settings.nav.general" },
  { id: "personas", label: "settings.nav.personas" },
  { id: "hotkeys", label: "settings.nav.hotkeys" },
] as const satisfies readonly { id: SettingsTab; label: TranslationKey }[];

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
const personaConfig = ref<PersonaConfig>({
  defaultSafePersonaId: DEFAULT_SAFE_PERSONA_ID,
  items: [...BUILT_IN_PERSONAS],
});
const personasLoading = ref(false);
const personasSaving = ref(false);
const personasDirty = ref(false);
const personaStatus = ref<PersonaStatus>("idle");
const personaErrorKey = ref<TranslationKey | null>(null);
const personaDraft = ref<PersonaDraft | null>(null);

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
const personaErrorMessage = computed(() => (personaErrorKey.value ? t(personaErrorKey.value) : ""));
const enabledPersonaCount = computed(() => personaConfig.value.items.filter((persona) => persona.enabled).length);
const canSavePersonas = computed(() => !personasLoading.value && !personasSaving.value && personasDirty.value);
const canSubmitPersonaDraft = computed(() => {
  const draft = personaDraft.value;
  return Boolean(draft?.item.name.trim() && draft.item.systemPrompt.trim());
});

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
  void loadPersonas();
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

async function loadPersonas() {
  personasLoading.value = true;
  personaErrorKey.value = null;

  try {
    personaConfig.value = await invoke<PersonaConfig>("get_persona_config");
    personasDirty.value = false;
    personaStatus.value = "idle";
    personaDraft.value = null;
  } catch (error) {
    personaStatus.value = "error";
    personaErrorKey.value = formatPersonaError(error);
  } finally {
    personasLoading.value = false;
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

async function savePersonas() {
  if (!canSavePersonas.value) {
    return;
  }

  personasSaving.value = true;
  personaErrorKey.value = null;

  try {
    personaConfig.value = await invoke<PersonaConfig>("save_persona_config", {
      personas: sanitizedPersonaConfig(),
    });
    personasDirty.value = false;
    personaStatus.value = "saved";
    personaDraft.value = null;
  } catch (error) {
    personaStatus.value = "error";
    personaErrorKey.value = formatPersonaError(error);
  } finally {
    personasSaving.value = false;
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

function markPersonasDirty() {
  personasDirty.value = true;
  if (personaStatus.value === "saved") {
    personaStatus.value = "idle";
  }
}

function startCreatePersona() {
  personaDraft.value = {
    mode: "create",
    originalId: null,
    item: createCustomPersona(),
  };
  personaStatus.value = "idle";
  personaErrorKey.value = null;
}

function startEditPersona(persona: PersonaDefinition) {
  if (persona.builtIn) {
    return;
  }

  personaDraft.value = {
    mode: "edit",
    originalId: persona.id,
    item: clonePersona(persona),
  };
  personaStatus.value = "idle";
  personaErrorKey.value = null;
}

function copyPersona(persona: PersonaDefinition) {
  personaDraft.value = {
    mode: "create",
    originalId: null,
    item: {
      ...clonePersona(persona),
      id: createPersonaId(persona.name),
      name: `${personaName(persona)} ${t("settings.personas.copySuffix")}`,
      nameKey: "",
      descriptionKey: "",
      builtIn: false,
      enabled: true,
    },
  };
  personaStatus.value = "idle";
  personaErrorKey.value = null;
}

function cancelPersonaDraft() {
  personaDraft.value = null;
}

function submitPersonaDraft() {
  const draft = personaDraft.value;
  if (!draft || !canSubmitPersonaDraft.value) {
    return;
  }

  const item = {
    ...draft.item,
    name: draft.item.name.trim(),
    description: draft.item.description?.trim() ?? "",
    systemPrompt: draft.item.systemPrompt.trim(),
    nameKey: "",
    descriptionKey: "",
    builtIn: false,
    enabled: true,
  };

  if (draft.mode === "edit" && draft.originalId) {
    personaConfig.value.items = personaConfig.value.items.map((persona) =>
      persona.id === draft.originalId ? item : persona,
    );
  } else {
    personaConfig.value.items = [...personaConfig.value.items, item];
  }

  personaDraft.value = null;
  markPersonasDirty();
}

function setDefaultPersona(personaId: string) {
  if (!personaConfig.value.items.some((persona) => persona.id === personaId && persona.enabled)) {
    return;
  }

  personaConfig.value.defaultSafePersonaId = personaId;
  markPersonasDirty();
}

function togglePersona(persona: PersonaDefinition) {
  if (persona.enabled && enabledPersonaCount.value <= 1) {
    personaStatus.value = "error";
    personaErrorKey.value = "settings.personas.needEnabled";
    return;
  }

  personaConfig.value.items = personaConfig.value.items.map((item) =>
    item.id === persona.id ? { ...item, enabled: !item.enabled } : item,
  );

  if (personaConfig.value.defaultSafePersonaId === persona.id && persona.enabled) {
    personaConfig.value.defaultSafePersonaId = nextEnabledPersonaId(persona.id);
  }

  markPersonasDirty();
}

function deletePersona(persona: PersonaDefinition) {
  if (persona.builtIn) {
    return;
  }

  personaConfig.value.items = personaConfig.value.items.filter((item) => item.id !== persona.id);
  if (personaConfig.value.defaultSafePersonaId === persona.id) {
    personaConfig.value.defaultSafePersonaId = nextEnabledPersonaId(persona.id);
  }

  if (personaDraft.value?.originalId === persona.id) {
    personaDraft.value = null;
  }

  markPersonasDirty();
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

function sanitizedPersonaConfig(): PersonaConfig {
  const items = personaConfig.value.items.map((persona) => ({
    ...persona,
    id: persona.id.trim(),
    name: persona.name.trim(),
    description: persona.description?.trim() ?? "",
    nameKey: persona.nameKey.trim(),
    descriptionKey: persona.descriptionKey.trim(),
    systemPrompt: persona.systemPrompt.trim(),
  }));
  const defaultSafePersonaId =
    items.find((persona) => persona.enabled && persona.id === personaConfig.value.defaultSafePersonaId)?.id ??
    items.find((persona) => persona.enabled)?.id ??
    DEFAULT_SAFE_PERSONA_ID;

  return {
    defaultSafePersonaId,
    items,
  };
}

function createCustomPersona(): PersonaDefinition {
  return {
    id: createPersonaId(),
    name: "",
    description: "",
    nameKey: "",
    descriptionKey: "",
    systemPrompt: "",
    builtIn: false,
    enabled: true,
  };
}

function clonePersona(persona: PersonaDefinition): PersonaDefinition {
  return {
    ...persona,
    description: persona.description ?? "",
  };
}

function createPersonaId(seed = "persona") {
  const slug =
    seed
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "persona";
  let id = `custom-${slug}`;
  let index = 2;
  const usedIds = new Set(personaConfig.value.items.map((persona) => persona.id));

  while (usedIds.has(id)) {
    id = `custom-${slug}-${index}`;
    index += 1;
  }

  return id;
}

function nextEnabledPersonaId(excludedPersonaId: string) {
  return (
    personaConfig.value.items.find((persona) => persona.enabled && persona.id !== excludedPersonaId)?.id ??
    DEFAULT_SAFE_PERSONA_ID
  );
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

function formatPersonaError(error: unknown): TranslationKey {
  const message = error instanceof Error ? error.message : String(error);

  if (message.includes("at least one enabled persona")) {
    return "settings.personas.needEnabled";
  }

  return "settings.personas.unknownError";
}

function personaName(persona: PersonaDefinition) {
  return persona.nameKey ? t(persona.nameKey as TranslationKey) : persona.name;
}

function personaDescription(persona: PersonaDefinition) {
  return persona.descriptionKey ? t(persona.descriptionKey as TranslationKey) : (persona.description ?? "");
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

        <div v-else-if="selectedTab === 'personas'" class="mt-6 space-y-5">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="text-sm text-shanka-secondary">{{ t("settings.field.activePersona") }}</div>
              <p class="mt-1 text-xs text-shanka-muted">{{ t("settings.personas.defaultHint") }}</p>
            </div>
            <button
              class="inline-flex h-9 items-center gap-2 rounded-md border border-shanka-border px-3 text-sm text-shanka-secondary transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-60"
              :disabled="personasLoading"
              type="button"
              @click="startCreatePersona"
            >
              <Plus class="size-4" aria-hidden="true" />
              <span>{{ t("settings.personas.add") }}</span>
            </button>
          </div>

          <form
            v-if="personaDraft"
            class="space-y-4 rounded-md border border-shanka-border p-3"
            @submit.prevent="submitPersonaDraft"
          >
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium text-shanka-primary">
                {{
                  personaDraft.mode === "edit"
                    ? t("settings.personas.editTitle")
                    : t("settings.personas.createTitle")
                }}
              </div>
              <button
                class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary"
                :title="t('settings.personas.cancel')"
                :aria-label="t('settings.personas.cancel')"
                type="button"
                @click="cancelPersonaDraft"
              >
                <X class="size-4" aria-hidden="true" />
              </button>
            </div>

            <label class="block">
              <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.personas.name") }}</span>
              <input
                v-model.trim="personaDraft.item.name"
                class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
                type="text"
              />
            </label>

            <label class="block">
              <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.personas.description") }}</span>
              <input
                v-model.trim="personaDraft.item.description"
                class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
                type="text"
              />
            </label>

            <label class="block">
              <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.personas.systemPrompt") }}</span>
              <textarea
                v-model.trim="personaDraft.item.systemPrompt"
                class="h-28 w-full resize-none rounded-md border border-transparent bg-shanka-input px-3 py-2 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
              />
            </label>

            <div class="flex justify-end">
              <button
                class="inline-flex h-9 items-center gap-2 rounded-md bg-shanka-primary px-3 text-sm text-shanka-canvas transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="!canSubmitPersonaDraft"
                type="submit"
              >
                <Save class="size-4" aria-hidden="true" />
                <span>
                  {{
                    personaDraft.mode === "edit"
                      ? t("settings.personas.update")
                      : t("settings.personas.create")
                  }}
                </span>
              </button>
            </div>
          </form>

          <p v-if="personasLoading" class="text-xs text-shanka-muted">
            {{ t("settings.personas.loading") }}
          </p>

          <div v-else class="divide-y divide-shanka-border rounded-md border border-shanka-border">
            <div
              v-for="persona in personaConfig.items"
              :key="persona.id"
              class="flex min-h-16 items-center justify-between gap-4 px-3 py-2 text-sm text-shanka-secondary"
            >
              <div class="min-w-0">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="truncate text-shanka-primary">{{ personaName(persona) }}</span>
                  <span
                    v-if="persona.id === personaConfig.defaultSafePersonaId"
                    class="rounded border border-shanka-success/40 px-1.5 py-0.5 text-[11px] text-shanka-success"
                  >
                    {{ t("settings.personas.defaultBadge") }}
                  </span>
                  <span class="rounded border border-shanka-border px-1.5 py-0.5 text-[11px] text-shanka-muted">
                    {{ persona.builtIn ? t("settings.personas.builtIn") : t("settings.personas.custom") }}
                  </span>
                  <span
                    v-if="!persona.enabled"
                    class="rounded border border-shanka-border px-1.5 py-0.5 text-[11px] text-shanka-muted"
                  >
                    {{ t("settings.personas.disabled") }}
                  </span>
                </div>
                <div class="mt-1 line-clamp-2 text-xs text-shanka-muted">
                  {{ personaDescription(persona) }}
                </div>
              </div>

              <div class="flex shrink-0 items-center gap-1">
                <button
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-40"
                  :title="t('settings.personas.setDefault')"
                  :aria-label="t('settings.personas.setDefault')"
                  :disabled="!persona.enabled || persona.id === personaConfig.defaultSafePersonaId"
                  type="button"
                  @click="setDefaultPersona(persona.id)"
                >
                  <Star class="size-4" aria-hidden="true" />
                </button>
                <button
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-40"
                  :title="persona.enabled ? t('settings.personas.disable') : t('settings.personas.enable')"
                  :aria-label="persona.enabled ? t('settings.personas.disable') : t('settings.personas.enable')"
                  :disabled="persona.enabled && enabledPersonaCount <= 1"
                  type="button"
                  @click="togglePersona(persona)"
                >
                  <Power class="size-4" aria-hidden="true" />
                </button>
                <button
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary"
                  :title="t('settings.personas.copy')"
                  :aria-label="t('settings.personas.copy')"
                  type="button"
                  @click="copyPersona(persona)"
                >
                  <Copy class="size-4" aria-hidden="true" />
                </button>
                <button
                  v-if="!persona.builtIn"
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary"
                  :title="t('settings.personas.edit')"
                  :aria-label="t('settings.personas.edit')"
                  type="button"
                  @click="startEditPersona(persona)"
                >
                  <Pencil class="size-4" aria-hidden="true" />
                </button>
                <button
                  v-if="!persona.builtIn"
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-danger"
                  :title="t('settings.personas.delete')"
                  :aria-label="t('settings.personas.delete')"
                  type="button"
                  @click="deletePersona(persona)"
                >
                  <Trash2 class="size-4" aria-hidden="true" />
                </button>
              </div>
            </div>
          </div>

          <div class="flex flex-wrap items-center justify-between gap-3">
            <p v-if="personaStatus === 'error'" class="text-xs text-red-500 dark:text-red-400">
              {{ t("settings.personas.errorPrefix") }}: {{ personaErrorMessage }}
            </p>
            <p v-else-if="personaStatus === 'saved'" class="text-xs text-shanka-success">
              {{ t("settings.personas.saved") }}
            </p>
            <p v-else class="text-xs text-shanka-muted">
              {{ t("settings.personas.saveHint") }}
            </p>

            <button
              class="inline-flex h-9 items-center gap-2 rounded-md bg-shanka-primary px-3 text-sm text-shanka-canvas transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="!canSavePersonas"
              type="button"
              @click="savePersonas"
            >
              <Save class="size-4" aria-hidden="true" />
              <span>{{ personasSaving ? t("settings.personas.saving") : t("settings.personas.save") }}</span>
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
