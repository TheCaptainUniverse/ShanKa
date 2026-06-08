<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, ChevronDown, Copy, LoaderCircle, RefreshCw, Replace, Search, X } from "lucide-vue-next";
import { useI18n } from "@/i18n/useI18n";
import { useHud } from "@/composables/useHud";
import { TAURI_EVENTS, type ErrorCode, type HudUpdate } from "@shared";
import type { TranslationKey } from "@/i18n/messages";

type PreviewAction = "copy" | "replace" | "regenerate";
type PersonaOption = {
  id: string;
  label: TranslationKey;
};

const { t } = useI18n();
const { currentHud, setHud } = useHud();
const hudWindow = getCurrentWindow();
const personaOptions = [
  { id: "workplace-eq", label: "persona.workplaceEq.name" },
  { id: "academic-concise", label: "persona.academicConcise.name" },
  { id: "clean-correction", label: "persona.cleanCorrection.name" },
] as const satisfies readonly PersonaOption[];
const selectedPersonaId = ref("clean-correction");
const personaSearch = ref("");
const personaSelectOpen = ref(false);
const busyAction = ref<PreviewAction | null>(null);
let unlistenHud: UnlistenFn | null = null;
let unlistenFocus: UnlistenFn | null = null;

const message = computed(() => {
  switch (currentHud.value.status) {
    case "refining":
      return t("hud.refining");
    case "replaced":
      return t("hud.replaced");
    case "error":
      return errorMessage(currentHud.value.errorCode);
    case "saved_to_clipboard":
      return t("hud.savedToClipboard");
    case "ready":
    case "idle":
    default:
      return t("hud.refining");
  }
});

const previewText = computed(() => currentHud.value.message?.trim() ?? "");
const previewId = computed(() => currentHud.value.previewId ?? null);
const isRefining = computed(() => currentHud.value.status === "refining");
const isError = computed(() => currentHud.value.status === "error");
const isPreview = computed(() => currentHud.value.status === "preview" && previewText.value !== "");
const selectedPersona = computed(
  () => personaOptions.find((persona) => persona.id === selectedPersonaId.value) ?? personaOptions[2],
);
const filteredPersonas = computed(() => {
  const query = personaSearch.value.trim().toLocaleLowerCase();
  if (!query) {
    return personaOptions;
  }

  return personaOptions.filter((persona) => t(persona.label).toLocaleLowerCase().includes(query));
});

onMounted(() => {
  void listen<HudUpdate>(TAURI_EVENTS.hudUpdate, (event) => {
    applyHudUpdate(event.payload);
  }).then((unlisten) => {
    unlistenHud = unlisten;
    void syncLatestHudState();
  });

  void hudWindow.onFocusChanged(({ payload: focused }) => {
    if (!focused) {
      void dismissPreview();
    }
  }).then((unlisten) => {
    unlistenFocus = unlisten;
  });

  window.addEventListener("blur", handleWindowBlur);
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  unlistenHud?.();
  unlistenFocus?.();
  window.removeEventListener("blur", handleWindowBlur);
  window.removeEventListener("keydown", handleKeydown);
});

async function syncLatestHudState() {
  try {
    applyHudUpdate(await invoke<HudUpdate>("get_hud_state"));
  } catch (error) {
    console.warn("[hud] failed to sync latest HUD state", error);
  }
}

function applyHudUpdate(update: HudUpdate) {
  setHud(update);
  busyAction.value = null;

  if (update.personaId) {
    selectedPersonaId.value = update.personaId;
  }
  if (update.status !== "preview") {
    personaSelectOpen.value = false;
  }
}

function errorMessage(errorCode?: ErrorCode | null) {
  switch (errorCode) {
    case "NO_TEXT_SELECTED":
      return t("hud.error.noTextSelected");
    case "NETWORK_TIMEOUT":
      return t("hud.error.networkTimeout");
    case "API_CONFIG_MISSING":
      return t("hud.error.apiConfigMissing");
    case "CLIPBOARD_ACCESS_FAILED":
      return t("hud.error.clipboardAccessFailed");
    case "PASTE_BLOCKED":
      return t("hud.error.pasteBlocked");
    case "API_ERROR":
    default:
      return t("hud.error.apiError");
  }
}

async function runPreviewAction(action: PreviewAction, personaId = selectedPersonaId.value) {
  if (!previewId.value || busyAction.value) {
    return;
  }

  busyAction.value = action;

  try {
    await invoke(commandForAction(action), {
      previewId: previewId.value,
      ...(action === "regenerate" ? { personaId } : {}),
    });

    if (action === "regenerate") {
      selectedPersonaId.value = personaId;
    }
  } catch (error) {
    console.warn(`[hud] failed to ${action} Safe Mode preview`, error);
    busyAction.value = null;
  }
}

function commandForAction(action: PreviewAction) {
  switch (action) {
    case "copy":
      return "copy_safe_preview";
    case "replace":
      return "replace_safe_preview";
    case "regenerate":
      return "regenerate_safe_preview";
  }
}

function selectPersona(personaId: string) {
  if (busyAction.value) {
    return;
  }

  selectedPersonaId.value = personaId;
  personaSelectOpen.value = false;
  personaSearch.value = "";
  void runPreviewAction("regenerate", personaId);
}

async function dismissPreview() {
  if (!previewId.value || busyAction.value === "replace") {
    return;
  }

  const dismissedPreviewId = previewId.value;
  setHud({ status: "idle" });
  personaSelectOpen.value = false;
  busyAction.value = null;

  try {
    await invoke("dismiss_safe_preview", { previewId: dismissedPreviewId });
  } catch (error) {
    console.warn("[hud] failed to dismiss Safe Mode preview", error);
  }
}

function handleWindowBlur() {
  void dismissPreview();
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    void dismissPreview();
  }
}
</script>

<template>
  <div
    v-if="isPreview"
    class="relative flex h-[212px] w-[404px] flex-col overflow-hidden rounded-lg border border-shanka-border bg-shanka-panel text-shanka-primary shadow-2xl"
  >
    <div class="min-h-0 flex-1 overflow-y-auto px-3 py-3 text-[13px] leading-5 text-shanka-secondary">
      <p class="whitespace-pre-wrap break-words">{{ previewText }}</p>
    </div>

    <div class="flex h-11 items-center justify-between gap-2 border-t border-shanka-border px-2">
      <div class="relative w-44">
        <button
          class="flex h-8 w-full items-center justify-between gap-2 rounded-md border border-shanka-border bg-shanka-input px-2 text-left text-xs text-shanka-secondary transition-colors hover:text-shanka-primary"
          type="button"
          :aria-label="t('settings.field.activePersona')"
          @click="personaSelectOpen = !personaSelectOpen"
        >
          <span class="truncate">{{ t(selectedPersona.label) }}</span>
          <ChevronDown class="size-3.5 shrink-0 text-shanka-muted" />
        </button>

        <div
          v-if="personaSelectOpen"
          class="absolute bottom-9 left-0 z-10 h-32 w-56 overflow-hidden rounded-md border border-shanka-border bg-shanka-panel shadow-2xl"
        >
          <label class="flex h-8 items-center gap-1.5 border-b border-shanka-border px-2 text-shanka-muted">
            <Search class="size-3.5 shrink-0" />
            <input
              v-model="personaSearch"
              class="h-full min-w-0 flex-1 bg-transparent text-xs text-shanka-primary outline-none placeholder:text-shanka-muted"
              :placeholder="t('hud.persona.search')"
              type="search"
            />
          </label>

          <div class="h-24 overflow-y-auto py-1">
            <button
              v-for="persona in filteredPersonas"
              :key="persona.id"
              class="flex h-8 w-full items-center px-2 text-left text-xs transition-colors hover:bg-shanka-input"
              :class="persona.id === selectedPersonaId ? 'text-shanka-primary' : 'text-shanka-muted'"
              type="button"
              @click="selectPersona(persona.id)"
            >
              <span class="truncate">{{ t(persona.label) }}</span>
            </button>
            <div v-if="filteredPersonas.length === 0" class="px-2 py-2 text-xs text-shanka-muted">
              {{ t("hud.persona.noResults") }}
            </div>
          </div>
        </div>
      </div>

      <div class="flex items-center justify-end gap-1">
      <button
        class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition-colors hover:bg-shanka-input hover:text-shanka-primary disabled:cursor-default disabled:opacity-50"
        type="button"
        :title="t('hud.action.copy')"
        :aria-label="t('hud.action.copy')"
        :disabled="busyAction !== null"
        @click="runPreviewAction('copy')"
      >
        <LoaderCircle v-if="busyAction === 'copy'" class="size-4 animate-spin" />
        <Copy v-else class="size-4" />
      </button>

      <button
        class="inline-flex size-8 items-center justify-center rounded-md text-shanka-muted transition-colors hover:bg-shanka-input hover:text-shanka-primary disabled:cursor-default disabled:opacity-50"
        type="button"
        :title="t('hud.action.regenerate')"
        :aria-label="t('hud.action.regenerate')"
        :disabled="busyAction !== null"
        @click="runPreviewAction('regenerate')"
      >
        <LoaderCircle v-if="busyAction === 'regenerate'" class="size-4 animate-spin" />
        <RefreshCw v-else class="size-4" />
      </button>

      <button
        class="inline-flex size-8 items-center justify-center rounded-md bg-shanka-primary text-shanka-canvas transition-colors hover:opacity-90 disabled:cursor-default disabled:opacity-50"
        type="button"
        :title="t('hud.action.replace')"
        :aria-label="t('hud.action.replace')"
        :disabled="busyAction !== null"
        @click="runPreviewAction('replace')"
      >
        <LoaderCircle v-if="busyAction === 'replace'" class="size-4 animate-spin" />
        <Replace v-else class="size-4" />
      </button>
      </div>
    </div>
  </div>

  <div
    v-else
    class="inline-flex h-9 max-w-[164px] items-center gap-2 rounded-md border border-shanka-border bg-shanka-panel px-3 text-xs font-medium text-shanka-secondary shadow-xl transition-colors"
  >
    <LoaderCircle v-if="isRefining" class="size-3.5 shrink-0 animate-spin text-shanka-primary" />
    <X v-else-if="isError" class="size-3.5 shrink-0 text-shanka-danger" />
    <Check v-else class="size-3.5 shrink-0 text-shanka-success" />
    <span class="truncate">{{ message }}</span>
  </div>
</template>
