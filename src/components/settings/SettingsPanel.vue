<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import {
  ChevronDown,
  CloudUpload,
  Copy,
  ExternalLink,
  HardDrive,
  KeyRound,
  Keyboard,
  LoaderCircle,
  Pencil,
  Plus,
  Power,
  RefreshCw,
  Save,
  ShieldCheck,
  Sparkles,
  Star,
  Sun,
  Moon,
  Trash2,
  X,
  Zap,
} from "lucide-vue-next";
import { useI18n } from "@/i18n/useI18n";
import type { Locale, TranslationKey } from "@/i18n/messages";
import {
  applyProviderPresetToSettings,
  PROVIDER_PRESETS as providerPresets,
  providerTestErrorKey as mapProviderTestErrorKey,
} from "@/settings/provider";
import {
  personaDisplayDescription,
  personaDisplayName,
  personaStorageDescription,
  personaStorageName,
} from "@/personas/display";
import { useTheme } from "@/theme/useTheme";
import type { Theme } from "@/theme/useTheme";
import {
  BUILT_IN_PERSONAS,
  DEFAULT_SAFE_PERSONA_ID,
  TAURI_EVENTS,
  type PersonaConfig,
  type PersonaDefinition,
} from "@shared";

type SettingsTab = "general" | "personas" | "history" | "hotkeys" | "about";
type HotkeyField = "safe_mode" | "magic_mode";
type HotkeyConfig = {
  safe_mode: string;
  magic_mode: string;
};
type HotkeyStatus = "idle" | "saved" | "error";
type AppSettingsConfig = {
  provider: string;
  api_key: string;
  api_key_ref: string;
  base_url: string;
  model: string;
  timeout_ms: number;
  debug_logging: boolean;
  history_enabled: boolean;
  launch_at_login: boolean;
};
type AppSettingsStatus = "idle" | "saved" | "error";
type ProviderTestStatus = "idle" | "success" | "error";
type PersonaStatus = "idle" | "saved" | "error";
type PlatformCapability = {
  status: "ok" | "warning" | "blocked" | "unknown";
  messageKey: string;
};
type PlatformStatus = {
  os: string;
  accessibility: PlatformCapability;
  globalHotkey: PlatformCapability;
  clipboard: PlatformCapability;
  inputSimulation: PlatformCapability;
  linuxSession?: string | null;
  notes: string[];
  settingsActionAvailable: boolean;
};
type RewriteHistoryItem = {
  id: number;
  mode: "safe" | "magic" | string;
  originalText: string;
  resultText: string;
  personaId?: string | null;
  action: "copied" | "replaced" | "saved_to_clipboard" | string;
  replaced: boolean;
  createdAtMs: number;
};
type PersonaDraftMode = "create" | "edit";
type PersonaDraft = {
  mode: PersonaDraftMode;
  originalId: string | null;
  item: PersonaDefinition;
};
type GeneratedPersonaDraft = {
  description: string;
  systemPrompt: string;
};
type PersonaDraftMessageTone = "error" | "success";

const { locale, locales, setLocale, t } = useI18n();
const { setTheme, theme, themes } = useTheme();

const navItems = [
  { id: "general", label: "settings.nav.general" },
  { id: "personas", label: "settings.nav.personas" },
  { id: "history", label: "settings.nav.history" },
  { id: "hotkeys", label: "settings.nav.hotkeys" },
  { id: "about", label: "settings.nav.about" },
] as const satisfies readonly { id: SettingsTab; label: TranslationKey }[];

const selectedTab = ref<SettingsTab>("general");
const appSettings = ref<AppSettingsConfig>({
  provider: "openai",
  api_key: "",
  api_key_ref: "",
  base_url: "",
  model: "",
  timeout_ms: 8000,
  debug_logging: false,
  history_enabled: true,
  launch_at_login: false,
});
const settingsLoading = ref(false);
const settingsSaving = ref(false);
const settingsDirty = ref(false);
const settingsStatus = ref<AppSettingsStatus>("idle");
const settingsErrorKey = ref<TranslationKey | null>(null);
const platformStatus = ref<PlatformStatus | null>(null);
const platformLoading = ref(false);
const platformErrorKey = ref<TranslationKey | null>(null);
const providerTesting = ref(false);
const providerTestStatus = ref<ProviderTestStatus>("idle");
const providerTestErrorKey = ref<TranslationKey | null>(null);
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
  deletedBuiltInPersonaIds: [],
});
const personasLoading = ref(false);
const personasSaving = ref(false);
const personasDirty = ref(false);
const personaStatus = ref<PersonaStatus>("idle");
const personaErrorKey = ref<TranslationKey | null>(null);
const personaDraft = ref<PersonaDraft | null>(null);
const personaDraftGenerating = ref(false);
const personaDraftMessageKey = ref<TranslationKey | null>(null);
const personaDraftMessageTone = ref<PersonaDraftMessageTone>("success");
const personaSystemPromptOpen = ref(false);
const historyItems = ref<RewriteHistoryItem[]>([]);
const historyLoading = ref(false);
const historyStatus = ref<"idle" | "copied" | "cleared" | "error">("idle");
const historyErrorKey = ref<TranslationKey | null>(null);
const appVersion = ref("");
const savedProviderSettingsSignature = ref("");
const testedProviderSettingsSignature = ref("");

const localeLabels = computed<Record<Locale, string>>(() => ({
  "zh-CN": t("settings.locale.zh"),
  "en-US": t("settings.locale.en"),
}));

const themeLabels = computed<Record<Theme, string>>(() => ({
  light: t("settings.theme.light"),
  dark: t("settings.theme.dark"),
}));

const settingsErrorMessage = computed(() => (settingsErrorKey.value ? t(settingsErrorKey.value) : ""));
const platformErrorMessage = computed(() => (platformErrorKey.value ? t(platformErrorKey.value) : ""));
const providerTestErrorMessage = computed(() => (providerTestErrorKey.value ? t(providerTestErrorKey.value) : ""));
const hotkeyErrorMessage = computed(() => (hotkeyErrorKey.value ? t(hotkeyErrorKey.value) : ""));
const personaErrorMessage = computed(() => (personaErrorKey.value ? t(personaErrorKey.value) : ""));
const historyErrorMessage = computed(() => (historyErrorKey.value ? t(historyErrorKey.value) : ""));
const enabledPersonaCount = computed(() => personaConfig.value.items.filter((persona) => persona.enabled).length);
const canSavePersonas = computed(() => !personasLoading.value && !personasSaving.value && personasDirty.value);
const canSubmitPersonaDraft = computed(
  () => Boolean(personaDraft.value) && !personaDraftGenerating.value && !personasSaving.value,
);
const personaDraftMessage = computed(() => (personaDraftMessageKey.value ? t(personaDraftMessageKey.value) : ""));
const personaDraftMessageClass = computed(() =>
  personaDraftMessageTone.value === "error" ? "text-red-500 dark:text-red-400" : "text-shanka-success",
);
const currentProviderSettingsSignature = computed(() => providerSettingsSignature(appSettings.value));
const providerSettingsDirty = computed(
  () => currentProviderSettingsSignature.value !== savedProviderSettingsSignature.value,
);
const currentProviderSettingsTested = computed(
  () =>
    providerTestStatus.value === "success" &&
    testedProviderSettingsSignature.value === currentProviderSettingsSignature.value,
);
const aiSettingsConfigured = computed(
  () =>
    (appSettings.value.api_key.trim() !== "" || appSettings.value.api_key_ref.trim() !== "") &&
    appSettings.value.base_url.trim() !== "" &&
    appSettings.value.model.trim() !== "",
);
const aiSettingsAvailableForGeneration = computed(
  () => aiSettingsConfigured.value && !providerSettingsDirty.value,
);

const canSaveSettings = computed(
  () =>
    !settingsLoading.value &&
    !settingsSaving.value &&
    appSettings.value.base_url.trim() !== "" &&
    (!providerSettingsDirty.value || currentProviderSettingsTested.value),
);
const canTestProvider = computed(
  () =>
    !settingsLoading.value &&
    !providerTesting.value &&
    (appSettings.value.api_key.trim() !== "" || appSettings.value.api_key_ref.trim() !== "") &&
    appSettings.value.base_url.trim() !== "" &&
    appSettings.value.model.trim() !== "",
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
  void loadAppVersion();
  void loadAppSettings();
  void loadHotkeys();
  void loadPersonas();
  void loadPlatformStatus();
  void loadHistory();
});

async function loadAppVersion() {
  try {
    appVersion.value = await invoke<string>("app_version");
  } catch (error) {
    console.warn("[settings] failed to load app version", error);
  }
}

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
    const loadedSettings = await invoke<Partial<AppSettingsConfig>>("get_app_settings");
    appSettings.value = {
      ...appSettings.value,
      ...loadedSettings,
      api_key: "",
      api_key_ref: loadedSettings.api_key_ref ?? "",
    };
    savedProviderSettingsSignature.value = providerSettingsSignature(appSettings.value);
    testedProviderSettingsSignature.value = "";
    settingsDirty.value = false;
    settingsStatus.value = "idle";
  } catch (error) {
    settingsStatus.value = "error";
    settingsErrorKey.value = formatSettingsError(error);
  } finally {
    settingsLoading.value = false;
  }
}

async function loadPlatformStatus() {
  platformLoading.value = true;
  platformErrorKey.value = null;

  try {
    platformStatus.value = await invoke<PlatformStatus>("get_platform_status");
  } catch (error) {
    console.warn("[settings] failed to load platform status", error);
    platformErrorKey.value = "settings.platform.loadFailed";
  } finally {
    platformLoading.value = false;
  }
}

async function openPlatformPermissionSettings() {
  try {
    await invoke("open_platform_permission_settings");
    void loadPlatformStatus();
  } catch (error) {
    console.warn("[settings] failed to open platform permission settings", error);
    platformErrorKey.value = "settings.platform.openFailed";
  }
}

async function loadPersonas() {
  personasLoading.value = true;
  personaErrorKey.value = null;

  try {
    personaConfig.value = normalizePersonaConfig(await invoke<PersonaConfig>("get_persona_config"));
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

async function loadHistory() {
  historyLoading.value = true;
  historyErrorKey.value = null;

  try {
    historyItems.value = await invoke<RewriteHistoryItem[]>("get_rewrite_history");
    historyStatus.value = "idle";
  } catch (error) {
    console.warn("[settings] failed to load rewrite history", error);
    historyStatus.value = "error";
    historyErrorKey.value = "settings.history.loadFailed";
  } finally {
    historyLoading.value = false;
  }
}

async function saveAppSettings() {
  if (providerSettingsDirty.value && !currentProviderSettingsTested.value) {
    settingsStatus.value = "error";
    settingsErrorKey.value = "settings.general.providerTestRequired";
    return;
  }

  if (!canSaveSettings.value) {
    return;
  }

  settingsSaving.value = true;
  settingsErrorKey.value = null;

  try {
    const savedSettings = await invoke<Partial<AppSettingsConfig>>("save_app_settings", {
      settings: appSettingsPayload(),
    });
    appSettings.value = {
      ...appSettings.value,
      ...savedSettings,
      api_key: "",
      api_key_ref: savedSettings.api_key_ref ?? appSettings.value.api_key_ref,
    };
    savedProviderSettingsSignature.value = providerSettingsSignature(appSettings.value);
    settingsDirty.value = false;
    settingsStatus.value = "saved";
  } catch (error) {
    settingsStatus.value = "error";
    settingsErrorKey.value = formatSettingsError(error);
  } finally {
    settingsSaving.value = false;
  }
}

async function testProviderConnection() {
  if (!canTestProvider.value) {
    providerTestStatus.value = "error";
    providerTestErrorKey.value = "settings.providerTest.missing";
    return;
  }

  providerTesting.value = true;
  providerTestErrorKey.value = null;
  providerTestStatus.value = "idle";

  try {
    await invoke("test_provider_connection", {
      settings: appSettingsPayload(),
    });
    providerTestStatus.value = "success";
    testedProviderSettingsSignature.value = currentProviderSettingsSignature.value;
  } catch (error) {
    providerTestStatus.value = "error";
    providerTestErrorKey.value = formatProviderTestError(error);
  } finally {
    providerTesting.value = false;
  }
}

async function savePersonas() {
  if (!canSavePersonas.value) {
    return;
  }
  if (hasIncompletePersonaFields(personaConfig.value.items)) {
    personaStatus.value = "error";
    personaErrorKey.value = "settings.personas.fieldsRequired";
    return;
  }

  personasSaving.value = true;
  personaErrorKey.value = null;

  try {
    personaConfig.value = normalizePersonaConfig(await invoke<PersonaConfig>("save_persona_config", {
      personas: sanitizedPersonaConfig(),
    }));
    personasDirty.value = false;
    personaStatus.value = "saved";
    personaDraft.value = null;
    void emit(TAURI_EVENTS.personasChanged, personaConfig.value).catch((eventError) => {
      console.warn("[settings] failed to broadcast persona config change", eventError);
    });
  } catch (error) {
    personaStatus.value = "error";
    personaErrorKey.value = formatPersonaError(error);
  } finally {
    personasSaving.value = false;
  }
}

async function copyHistoryResult(historyId: number) {
  historyErrorKey.value = null;

  try {
    await invoke("copy_history_result", { historyId });
    historyStatus.value = "copied";
  } catch (error) {
    console.warn("[settings] failed to copy rewrite history result", error);
    historyStatus.value = "error";
    historyErrorKey.value = "settings.history.copyFailed";
  }
}

async function clearHistory() {
  historyErrorKey.value = null;

  try {
    await invoke("clear_rewrite_history");
    historyItems.value = [];
    historyStatus.value = "cleared";
  } catch (error) {
    console.warn("[settings] failed to clear rewrite history", error);
    historyStatus.value = "error";
    historyErrorKey.value = "settings.history.clearFailed";
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

function markSettingsDirty(resetProviderTest: boolean | Event = true) {
  const shouldResetProviderTest = resetProviderTest !== false;

  settingsDirty.value = true;
  if (shouldResetProviderTest) {
    providerTestStatus.value = "idle";
    providerTestErrorKey.value = null;
    testedProviderSettingsSignature.value = "";
  }
  if (settingsStatus.value === "saved") {
    settingsStatus.value = "idle";
  }
}

function appSettingsPayload(): AppSettingsConfig {
  return {
    provider: appSettings.value.provider.trim() || "custom",
    api_key: appSettings.value.api_key.trim(),
    api_key_ref: appSettings.value.api_key_ref.trim(),
    base_url: appSettings.value.base_url.trim(),
    model: appSettings.value.model.trim(),
    timeout_ms: Math.round(appSettings.value.timeout_ms),
    debug_logging: appSettings.value.debug_logging,
    history_enabled: appSettings.value.history_enabled,
    launch_at_login: appSettings.value.launch_at_login,
  };
}

function providerSettingsSignature(settings: AppSettingsConfig) {
  return [
    settings.provider.trim() || "custom",
    settings.api_key.trim(),
    settings.api_key_ref.trim(),
    settings.base_url.trim(),
    settings.model.trim(),
    String(Math.round(settings.timeout_ms)),
  ].join("\n");
}

function applyProviderPreset(providerId: string) {
  appSettings.value = applyProviderPresetToSettings(appSettings.value, providerId);
  markSettingsDirty();
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
  resetPersonaDraftState();
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
  resetPersonaDraftState();
}

function copyPersona(persona: PersonaDefinition) {
  const name = personaName(persona);

  personaDraft.value = {
    mode: "create",
    originalId: null,
    item: {
      ...clonePersona(persona),
      id: createPersonaId(name),
      name: `${name} ${t("settings.personas.copySuffix")}`,
      nameKey: "",
      descriptionKey: "",
      builtIn: false,
      enabled: true,
    },
  };
  resetPersonaDraftState();
}

async function generatePersonaDraft() {
  const draft = personaDraft.value;
  if (!draft || personaDraftGenerating.value) {
    return;
  }

  const name = draft.item.name.trim();
  if (!name) {
    showPersonaDraftMessage("settings.personas.aiNameRequired", "error");
    return;
  }
  if (!aiSettingsAvailableForGeneration.value) {
    showPersonaDraftMessage("settings.personas.aiSettingsRequired", "error");
    openAiSettingsForPersonaGeneration();
    return;
  }

  if (draft.item.description.trim() || draft.item.systemPrompt.trim()) {
    const shouldOverwrite = window.confirm(t("settings.personas.aiOverwriteConfirm"));
    if (!shouldOverwrite) {
      return;
    }
  }

  personaDraftGenerating.value = true;
  personaDraftMessageKey.value = null;
  personaStatus.value = "idle";
  personaErrorKey.value = null;

  try {
    const generated = await invoke<GeneratedPersonaDraft>("generate_persona_draft", {
      name,
      locale: locale.value,
    });

    if (personaDraft.value !== draft) {
      return;
    }

    draft.item.description = generated.description.trim();
    draft.item.systemPrompt = generated.systemPrompt.trim();
    showPersonaDraftMessage("settings.personas.aiGenerated", "success");
  } catch (error) {
    console.warn("[settings] failed to generate persona draft", error);
    if (personaDraft.value === draft) {
      showPersonaDraftMessage("settings.personas.aiGenerateFailed", "error");
    }
  } finally {
    if (personaDraft.value === draft) {
      personaDraftGenerating.value = false;
    }
  }
}

function openAiSettingsForPersonaGeneration() {
  selectedTab.value = "general";
  settingsStatus.value = "error";
  settingsErrorKey.value = "settings.general.aiRequiredForPersona";
  providerTestStatus.value = "error";
  providerTestErrorKey.value = "settings.providerTest.missing";
}

function resetPersonaDraftState() {
  personaStatus.value = "idle";
  personaErrorKey.value = null;
  personaDraftGenerating.value = false;
  personaDraftMessageKey.value = null;
  personaDraftMessageTone.value = "success";
  personaSystemPromptOpen.value = false;
}

function cancelPersonaDraft() {
  personaDraft.value = null;
  resetPersonaDraftState();
}

function submitPersonaDraft() {
  const draft = personaDraft.value;
  if (!draft || personaDraftGenerating.value) {
    return;
  }
  if (!validatePersonaDraft(draft)) {
    return;
  }

  const item = {
    ...draft.item,
    name: draft.item.name.trim(),
    description: draft.item.description.trim(),
    systemPrompt: draft.item.systemPrompt.trim(),
    nameKey: draft.item.builtIn ? draft.item.nameKey.trim() : "",
    descriptionKey: draft.item.builtIn ? draft.item.descriptionKey.trim() : "",
    builtIn: draft.item.builtIn,
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
  resetPersonaDraftState();
  markPersonasDirty();
  void savePersonas();
}

function validatePersonaDraft(draft: PersonaDraft) {
  if (
    !draft.item.name.trim() ||
    !draft.item.description.trim() ||
    !draft.item.systemPrompt.trim()
  ) {
    showPersonaDraftMessage("settings.personas.fieldsRequired", "error");
    return false;
  }

  return true;
}

function showPersonaDraftMessage(key: TranslationKey, tone: PersonaDraftMessageTone) {
  personaDraftMessageKey.value = key;
  personaDraftMessageTone.value = tone;
}

function clearPersonaDraftMessage() {
  personaDraftMessageKey.value = null;
}

function onPersonaSystemPromptToggle(event: Event) {
  personaSystemPromptOpen.value = (event.currentTarget as HTMLDetailsElement).open;
}

function setDefaultPersona(personaId: string) {
  if (!personaConfig.value.items.some((persona) => persona.id === personaId && persona.enabled)) {
    return;
  }

  personaConfig.value.defaultSafePersonaId = personaId;
  markPersonasDirty();
  void savePersonas();
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
  void savePersonas();
}

function deletePersona(persona: PersonaDefinition) {
  personaConfig.value.items = personaConfig.value.items.filter((item) => item.id !== persona.id);
  if (persona.builtIn) {
    personaConfig.value.deletedBuiltInPersonaIds = [
      ...new Set([...personaConfig.value.deletedBuiltInPersonaIds, persona.id]),
    ];
  }
  if (personaConfig.value.defaultSafePersonaId === persona.id) {
    personaConfig.value.defaultSafePersonaId = nextEnabledPersonaId(persona.id);
  }

  if (personaDraft.value?.originalId === persona.id) {
    personaDraft.value = null;
  }

  markPersonasDirty();
  void savePersonas();
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

function platformMessage(messageKey: string) {
  return t(messageKey as TranslationKey);
}

function platformStatusClass(status: PlatformCapability["status"]) {
  switch (status) {
    case "ok":
      return "text-shanka-success";
    case "blocked":
      return "text-shanka-danger";
    case "warning":
    case "unknown":
    default:
      return "text-shanka-muted";
  }
}

function historyModeLabel(mode: RewriteHistoryItem["mode"]) {
  return mode === "magic" ? t("settings.history.mode.magic") : t("settings.history.mode.safe");
}

function historyActionLabel(action: RewriteHistoryItem["action"]) {
  switch (action) {
    case "replaced":
      return t("settings.history.action.replaced");
    case "copied":
      return t("settings.history.action.copied");
    case "saved_to_clipboard":
      return t("settings.history.action.savedToClipboard");
    default:
      return t("settings.history.action.saved");
  }
}

function historyTimestamp(createdAtMs: number) {
  return new Intl.DateTimeFormat(locale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(createdAtMs));
}

function sanitizedPersonaConfig(): PersonaConfig {
  const items = personaConfig.value.items.map((persona) => ({
    ...persona,
    id: persona.id.trim(),
    name: personaStorageName(persona),
    description: personaStorageDescription(persona),
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
    deletedBuiltInPersonaIds: [
      ...new Set(personaConfig.value.deletedBuiltInPersonaIds.map((id) => id.trim()).filter(Boolean)),
    ],
  };
}

function normalizePersonaConfig(config: PersonaConfig): PersonaConfig {
  return {
    defaultSafePersonaId: config.defaultSafePersonaId,
    items: config.items,
    deletedBuiltInPersonaIds: config.deletedBuiltInPersonaIds ?? [],
  };
}

function hasIncompletePersonaFields(items: PersonaDefinition[]) {
  return items.some((persona) => !personaRequiredFieldsComplete(persona));
}

function personaRequiredFieldsComplete(persona: PersonaDefinition) {
  const hasName = Boolean(persona.name.trim() || persona.nameKey.trim());
  const hasDescription = Boolean((persona.description ?? "").trim() || persona.descriptionKey.trim());
  const hasSystemPrompt = Boolean(persona.systemPrompt.trim());

  return hasName && hasDescription && hasSystemPrompt;
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
  if (message.includes("system keychain") || message.includes("keychain")) {
    return "settings.general.keychainError";
  }
  if (message.includes("launch-at-login")) {
    return "settings.general.autostartError";
  }

  return "settings.general.unknownError";
}

function formatProviderTestError(error: unknown): TranslationKey {
  return mapProviderTestErrorKey(error);
}

function formatPersonaError(error: unknown): TranslationKey {
  const message = error instanceof Error ? error.message : String(error);

  if (message.includes("at least one enabled persona")) {
    return "settings.personas.needEnabled";
  }
  if (message.includes("name, description, and system prompt")) {
    return "settings.personas.fieldsRequired";
  }

  return "settings.personas.unknownError";
}

function personaName(persona: PersonaDefinition) {
  return personaDisplayName(persona, t);
}

function personaDescription(persona: PersonaDefinition) {
  return personaDisplayDescription(persona, t);
}
</script>

<template>
  <section class="mx-auto grid min-h-screen max-w-5xl grid-cols-[220px_1fr]">
    <aside class="sticky top-0 h-screen self-start overflow-y-auto border-r border-shanka-border px-3 py-5">
      <div class="mb-5 flex items-center gap-2 px-2 text-sm font-medium text-shanka-primary">
        <img class="size-7 shrink-0 object-contain" src="/pure_logo.png" alt="" aria-hidden="true" />
        <span>{{ t("app.name") }}</span>
      </div>
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
          <section class="rounded-md border border-shanka-border px-3 py-3">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div class="flex items-center gap-2 text-sm font-medium text-shanka-primary">
                <ShieldCheck class="size-4" aria-hidden="true" />
                <span>{{ t("settings.platform.title") }}</span>
              </div>
              <div class="flex items-center gap-1">
                <button
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-50"
                  :title="t('settings.platform.refresh')"
                  :aria-label="t('settings.platform.refresh')"
                  :disabled="platformLoading"
                  type="button"
                  @click="loadPlatformStatus"
                >
                  <RefreshCw class="size-4" :class="platformLoading ? 'animate-spin' : ''" aria-hidden="true" />
                </button>
                <button
                  v-if="platformStatus?.settingsActionAvailable"
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-40"
                  :title="t('settings.platform.openSettings')"
                  :aria-label="t('settings.platform.openSettings')"
                  type="button"
                  @click="openPlatformPermissionSettings"
                >
                  <ExternalLink class="size-4" aria-hidden="true" />
                </button>
              </div>
            </div>

            <p v-if="platformErrorKey" class="mt-2 text-xs text-red-500 dark:text-red-400">
              {{ platformErrorMessage }}
            </p>
            <p v-else-if="platformLoading" class="mt-2 text-xs text-shanka-muted">
              {{ t("settings.platform.loading") }}
            </p>
            <div v-else-if="platformStatus" class="mt-3 grid gap-2 text-xs">
              <div class="flex items-center justify-between gap-3">
                <span class="text-shanka-muted">{{ t("settings.platform.accessibility") }}</span>
                <span :class="platformStatusClass(platformStatus.accessibility.status)">
                  {{ platformMessage(platformStatus.accessibility.messageKey) }}
                </span>
              </div>
              <div class="flex items-center justify-between gap-3">
                <span class="text-shanka-muted">{{ t("settings.platform.globalHotkey") }}</span>
                <span :class="platformStatusClass(platformStatus.globalHotkey.status)">
                  {{ platformMessage(platformStatus.globalHotkey.messageKey) }}
                </span>
              </div>
              <div class="flex items-center justify-between gap-3">
                <span class="text-shanka-muted">{{ t("settings.platform.clipboard") }}</span>
                <span :class="platformStatusClass(platformStatus.clipboard.status)">
                  {{ platformMessage(platformStatus.clipboard.messageKey) }}
                </span>
              </div>
              <div class="flex items-center justify-between gap-3">
                <span class="text-shanka-muted">{{ t("settings.platform.inputSimulation") }}</span>
                <span :class="platformStatusClass(platformStatus.inputSimulation.status)">
                  {{ platformMessage(platformStatus.inputSimulation.messageKey) }}
                </span>
              </div>
              <div v-if="platformStatus.linuxSession" class="flex items-center justify-between gap-3">
                <span class="text-shanka-muted">{{ t("settings.platform.linuxSession") }}</span>
                <span class="text-shanka-primary">{{ platformStatus.linuxSession }}</span>
              </div>
              <p v-for="note in platformStatus.notes" :key="note" class="text-shanka-muted">
                {{ platformMessage(note) }}
              </p>
            </div>
          </section>

          <label class="block">
            <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.field.provider") }}</span>
            <select
              v-model="appSettings.provider"
              class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
              :disabled="settingsLoading"
              @change="applyProviderPreset(appSettings.provider)"
            >
              <option v-for="preset in providerPresets" :key="preset.id" :value="preset.id">
                {{ preset.id === "custom" ? t("settings.provider.custom") : preset.label }}
              </option>
            </select>
          </label>

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
            <span class="mt-1 block text-xs text-shanka-muted">{{ t("settings.general.apiKeyKeychainHint") }}</span>
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

          <label class="flex items-center justify-between gap-4 rounded-md border border-shanka-border px-3 py-2">
            <span>
              <span class="block text-sm text-shanka-secondary">{{ t("settings.field.debugLogging") }}</span>
              <span class="mt-1 block text-xs text-shanka-muted">{{ t("settings.general.debugLoggingHint") }}</span>
            </span>
            <input
              v-model="appSettings.debug_logging"
              class="size-4 accent-shanka-primary"
              :disabled="settingsLoading"
              type="checkbox"
              @change="markSettingsDirty(false)"
            />
          </label>

          <label class="flex items-center justify-between gap-4 rounded-md border border-shanka-border px-3 py-2">
            <span>
              <span class="block text-sm text-shanka-secondary">{{ t("settings.field.historyEnabled") }}</span>
              <span class="mt-1 block text-xs text-shanka-muted">{{ t("settings.general.historyHint") }}</span>
            </span>
            <input
              v-model="appSettings.history_enabled"
              class="size-4 accent-shanka-primary"
              :disabled="settingsLoading"
              type="checkbox"
              @change="markSettingsDirty(false)"
            />
          </label>

          <label class="flex items-center justify-between gap-4 rounded-md border border-shanka-border px-3 py-2">
            <span>
              <span class="block text-sm text-shanka-secondary">{{ t("settings.field.launchAtLogin") }}</span>
              <span class="mt-1 block text-xs text-shanka-muted">{{ t("settings.general.launchAtLoginHint") }}</span>
            </span>
            <input
              v-model="appSettings.launch_at_login"
              class="size-4 accent-shanka-primary"
              :disabled="settingsLoading"
              type="checkbox"
              @change="markSettingsDirty(false)"
            />
          </label>

          <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-shanka-border px-3 py-2">
            <p v-if="providerTesting" class="text-xs text-shanka-muted">
              {{ t("settings.providerTest.testing") }}
            </p>
            <p v-else-if="providerTestStatus === 'success'" class="text-xs text-shanka-success">
              {{ t("settings.providerTest.success") }}
            </p>
            <p v-else-if="providerTestStatus === 'error'" class="text-xs text-red-500 dark:text-red-400">
              {{ providerTestErrorMessage }}
            </p>
            <p v-else class="text-xs text-shanka-muted">
              {{ t("settings.providerTest.hint") }}
            </p>

            <button
              class="inline-flex h-8 items-center gap-2 rounded-md border border-shanka-border px-3 text-xs text-shanka-secondary transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="!canTestProvider"
              type="button"
              @click="testProviderConnection"
            >
              <Zap class="size-3.5" aria-hidden="true" />
              <span>{{ t("settings.providerTest.action") }}</span>
            </button>
          </div>

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
            <p v-else-if="providerSettingsDirty && !currentProviderSettingsTested" class="text-xs text-shanka-muted">
              {{ t("settings.general.providerTestRequiredHint") }}
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
              :disabled="personasLoading || personasSaving"
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
              <div class="flex gap-2">
                <input
                  v-model.trim="personaDraft.item.name"
                  class="h-10 min-w-0 flex-1 rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
                  type="text"
                  @input="clearPersonaDraftMessage"
                />
                <button
                  class="inline-flex size-10 shrink-0 items-center justify-center rounded-md border border-shanka-border text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-50"
                  :title="t('settings.personas.aiGenerate')"
                  :aria-label="t('settings.personas.aiGenerate')"
                  :disabled="personaDraftGenerating"
                  type="button"
                  @click="generatePersonaDraft"
                >
                  <LoaderCircle v-if="personaDraftGenerating" class="size-4 animate-spin" aria-hidden="true" />
                  <Sparkles v-else class="size-4" aria-hidden="true" />
                </button>
              </div>
            </label>

            <label class="block">
              <span class="mb-2 block text-sm text-shanka-secondary">{{ t("settings.personas.description") }}</span>
              <input
                v-model.trim="personaDraft.item.description"
                class="h-10 w-full rounded-md border border-transparent bg-shanka-input px-3 text-sm text-shanka-primary outline-none transition focus:border-shanka-focus"
                type="text"
                @input="clearPersonaDraftMessage"
              />
            </label>

            <details
              class="group rounded-md border border-shanka-border"
              :open="personaSystemPromptOpen"
              @toggle="onPersonaSystemPromptToggle"
            >
              <summary class="flex h-10 cursor-pointer list-none items-center justify-between gap-3 px-3 text-sm text-shanka-secondary [&::-webkit-details-marker]:hidden">
                <span>{{ t("settings.personas.systemPrompt") }}</span>
                <ChevronDown class="size-4 text-shanka-muted transition group-open:rotate-180" aria-hidden="true" />
              </summary>
              <textarea
                v-model.trim="personaDraft.item.systemPrompt"
                class="h-28 w-full resize-none border-t border-shanka-border bg-transparent px-3 py-2 text-sm text-shanka-primary outline-none transition focus:bg-shanka-input/50"
                @input="clearPersonaDraftMessage"
              />
            </details>

            <p v-if="personaDraftMessage" class="text-xs" :class="personaDraftMessageClass">
              {{ personaDraftMessage }}
            </p>

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
                  :disabled="personasSaving || !persona.enabled || persona.id === personaConfig.defaultSafePersonaId"
                  type="button"
                  @click="setDefaultPersona(persona.id)"
                >
                  <Star class="size-4" aria-hidden="true" />
                </button>
                <button
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-40"
                  :title="persona.enabled ? t('settings.personas.disable') : t('settings.personas.enable')"
                  :aria-label="persona.enabled ? t('settings.personas.disable') : t('settings.personas.enable')"
                  :disabled="personasSaving || (persona.enabled && enabledPersonaCount <= 1)"
                  type="button"
                  @click="togglePersona(persona)"
                >
                  <Power class="size-4" aria-hidden="true" />
                </button>
                <button
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-40"
                  :title="t('settings.personas.copy')"
                  :aria-label="t('settings.personas.copy')"
                  :disabled="personasSaving"
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
                  :disabled="personasSaving"
                  type="button"
                  @click="startEditPersona(persona)"
                >
                  <Pencil class="size-4" aria-hidden="true" />
                </button>
                <button
                  class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-danger disabled:cursor-not-allowed disabled:opacity-40"
                  :title="t('settings.personas.delete')"
                  :aria-label="t('settings.personas.delete')"
                  :disabled="personasSaving"
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

        <div v-else-if="selectedTab === 'history'" class="mt-6 space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="text-sm text-shanka-secondary">{{ t("settings.history.title") }}</div>
              <p class="mt-1 text-xs text-shanka-muted">{{ t("settings.history.hint") }}</p>
            </div>
            <div class="flex items-center gap-1">
              <button
                class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary disabled:cursor-not-allowed disabled:opacity-50"
                :title="t('settings.history.refresh')"
                :aria-label="t('settings.history.refresh')"
                :disabled="historyLoading"
                type="button"
                @click="loadHistory"
              >
                <RefreshCw class="size-4" :class="historyLoading ? 'animate-spin' : ''" aria-hidden="true" />
              </button>
              <button
                class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-danger disabled:cursor-not-allowed disabled:opacity-50"
                :title="t('settings.history.clear')"
                :aria-label="t('settings.history.clear')"
                :disabled="historyLoading || historyItems.length === 0"
                type="button"
                @click="clearHistory"
              >
                <Trash2 class="size-4" aria-hidden="true" />
              </button>
            </div>
          </div>

          <p v-if="historyLoading" class="text-xs text-shanka-muted">
            {{ t("settings.history.loading") }}
          </p>
          <p v-else-if="historyStatus === 'error'" class="text-xs text-red-500 dark:text-red-400">
            {{ historyErrorMessage }}
          </p>
          <p v-else-if="historyStatus === 'copied'" class="text-xs text-shanka-success">
            {{ t("settings.history.copied") }}
          </p>
          <p v-else-if="historyStatus === 'cleared'" class="text-xs text-shanka-success">
            {{ t("settings.history.cleared") }}
          </p>

          <div v-if="!historyLoading && historyItems.length === 0" class="rounded-md border border-shanka-border px-3 py-6 text-center text-xs text-shanka-muted">
            {{ t("settings.history.empty") }}
          </div>

          <div v-else class="divide-y divide-shanka-border rounded-md border border-shanka-border">
            <div v-for="item in historyItems" :key="item.id" class="px-3 py-3">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="flex flex-wrap items-center gap-2 text-xs">
                    <span class="rounded border border-shanka-border px-1.5 py-0.5 text-shanka-primary">
                      {{ historyModeLabel(item.mode) }}
                    </span>
                    <span class="text-shanka-muted">{{ historyActionLabel(item.action) }}</span>
                    <span class="text-shanka-muted">{{ historyTimestamp(item.createdAtMs) }}</span>
                  </div>
                  <p class="mt-2 line-clamp-2 text-xs text-shanka-muted">
                    {{ item.originalText }}
                  </p>
                  <p class="mt-2 line-clamp-3 whitespace-pre-wrap text-sm text-shanka-secondary">
                    {{ item.resultText }}
                  </p>
                </div>
                <button
                  class="inline-flex size-8 shrink-0 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-hover/5 hover:text-shanka-primary"
                  :title="t('settings.history.copyResult')"
                  :aria-label="t('settings.history.copyResult')"
                  type="button"
                  @click="copyHistoryResult(item.id)"
                >
                  <Copy class="size-4" aria-hidden="true" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <form v-else-if="selectedTab === 'hotkeys'" class="mt-6 space-y-5" @submit.prevent="saveHotkeys">
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

        <div v-else-if="selectedTab === 'about'" class="mt-6 space-y-5">
          <section class="rounded-md border border-shanka-border px-3 py-3">
            <div class="text-sm font-medium text-shanka-primary">{{ t("settings.about.title") }}</div>
            <p class="mt-2 text-sm leading-6 text-shanka-secondary">{{ t("settings.about.description") }}</p>
            <div class="mt-4 grid gap-2 text-sm">
              <div class="flex items-center justify-between gap-4">
                <span class="text-shanka-muted">{{ t("settings.about.version") }}</span>
                <span class="text-shanka-primary">{{ appVersion || "0.1.0" }}</span>
              </div>
              <div class="flex items-start justify-between gap-4">
                <span class="text-shanka-muted">{{ t("settings.about.safeMode") }}</span>
                <span class="max-w-sm text-right text-shanka-secondary">{{ t("settings.about.safeModeDescription") }}</span>
              </div>
              <div class="flex items-start justify-between gap-4">
                <span class="text-shanka-muted">{{ t("settings.about.magicMode") }}</span>
                <span class="max-w-sm text-right text-shanka-secondary">{{ t("settings.about.magicModeDescription") }}</span>
              </div>
            </div>
          </section>

          <section class="rounded-md border border-shanka-border px-3 py-3">
            <div class="text-sm font-medium text-shanka-primary">{{ t("settings.about.privacyTitle") }}</div>
            <p class="mt-2 text-sm leading-6 text-shanka-secondary">{{ t("settings.about.privacy") }}</p>
            <div class="mt-4 grid gap-2 sm:grid-cols-3">
              <div class="rounded-md border border-shanka-border bg-shanka-input/40 p-3">
                <HardDrive class="size-4 text-shanka-primary" aria-hidden="true" />
                <div class="mt-2 text-xs font-medium text-shanka-primary">{{ t("settings.about.privacyLocalTitle") }}</div>
                <p class="mt-1 text-xs leading-5 text-shanka-muted">{{ t("settings.about.privacyLocalDescription") }}</p>
              </div>
              <div class="rounded-md border border-shanka-border bg-shanka-input/40 p-3">
                <KeyRound class="size-4 text-shanka-primary" aria-hidden="true" />
                <div class="mt-2 text-xs font-medium text-shanka-primary">{{ t("settings.about.privacyKeychainTitle") }}</div>
                <p class="mt-1 text-xs leading-5 text-shanka-muted">{{ t("settings.about.privacyKeychainDescription") }}</p>
              </div>
              <div class="rounded-md border border-shanka-border bg-shanka-input/40 p-3">
                <CloudUpload class="size-4 text-shanka-primary" aria-hidden="true" />
                <div class="mt-2 text-xs font-medium text-shanka-primary">{{ t("settings.about.privacySendTitle") }}</div>
                <p class="mt-1 text-xs leading-5 text-shanka-muted">{{ t("settings.about.privacySendDescription") }}</p>
              </div>
            </div>
          </section>
        </div>
      </div>
    </section>
  </section>
</template>
