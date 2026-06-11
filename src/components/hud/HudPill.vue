<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, ChevronDown, Copy, FileDiff, LoaderCircle, PenLine, RefreshCw, Replace, Search, Undo2, X } from "lucide-vue-next";
import { useI18n } from "@/i18n/useI18n";
import { personaDisplayName } from "@/personas/display";
import { buildTextDiff, diffStats, type DiffPart } from "@/hud/diff";
import {
  hudErrorMessageKey,
  hudMessageKey,
  previewActionCommand,
  shouldResetEditablePreviewText,
  type PreviewAction,
} from "@/hud/state";
import { useHud } from "@/composables/useHud";
import {
  BUILT_IN_PERSONAS,
  DEFAULT_SAFE_PERSONA_ID,
  TAURI_EVENTS,
  type HudUpdate,
  type PersonaConfig,
  type PersonaDefinition,
} from "@shared";

const { t } = useI18n();
const { currentHud, setHud } = useHud();
const hudWindow = getCurrentWindow();
const personaOptions = ref<PersonaDefinition[]>(BUILT_IN_PERSONAS.filter((persona) => persona.enabled));
const selectedPersonaId = ref(DEFAULT_SAFE_PERSONA_ID);
const personaSearch = ref("");
const personaSelectOpen = ref(false);
const busyAction = ref<PreviewAction | null>(null);
const editablePreviewText = ref("");
const previewView = ref<"diff" | "result">("diff");
const undoCopying = ref(false);
let unlistenHud: UnlistenFn | null = null;
let unlistenFocus: UnlistenFn | null = null;
let unlistenPersonas: UnlistenFn | null = null;

const message = computed(() => t(hudMessageKey(currentHud.value)));

const previewText = computed(() => currentHud.value.message ?? "");
const originalText = computed(() => currentHud.value.originalText ?? "");
const previewId = computed(() => currentHud.value.previewId ?? null);
const isRefining = computed(() => currentHud.value.status === "refining");
const isError = computed(() => currentHud.value.status === "error");
const isPreview = computed(() => currentHud.value.status === "preview" && previewText.value !== "");
const isUndoAvailable = computed(() => currentHud.value.status === "undo_available");
const previewErrorMessage = computed(() =>
  isPreview.value && currentHud.value.errorCode ? t(hudErrorMessageKey(currentHud.value.errorCode)) : "",
);
const diffParts = computed<DiffPart[]>(() => buildTextDiff(originalText.value, editablePreviewText.value));
const currentDiffStats = computed(() => diffStats(diffParts.value));
const hasDiff = computed(() => currentDiffStats.value.added > 0 || currentDiffStats.value.removed > 0);
const canShowDiff = computed(() => originalText.value !== "");
const selectedPersona = computed(
  () =>
    personaOptions.value.find((persona) => persona.id === selectedPersonaId.value) ??
    personaOptions.value[0] ??
    BUILT_IN_PERSONAS[0],
);
const filteredPersonas = computed(() => {
  const query = personaSearch.value.trim().toLocaleLowerCase();
  if (!query) {
    return personaOptions.value;
  }

  return personaOptions.value.filter((persona) => personaLabel(persona).toLocaleLowerCase().includes(query));
});

onMounted(() => {
  void loadPersonaConfig();

  void listen<HudUpdate>(TAURI_EVENTS.hudUpdate, (event) => {
    applyHudUpdate(event.payload);
  }).then((unlisten) => {
    unlistenHud = unlisten;
    void syncLatestHudState();
  });

  void listen<PersonaConfig>(TAURI_EVENTS.personasChanged, (event) => {
    applyPersonaConfig(event.payload);
  }).then((unlisten) => {
    unlistenPersonas = unlisten;
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
  unlistenPersonas?.();
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

async function loadPersonaConfig() {
  try {
    const config = await invoke<PersonaConfig>("get_persona_config");
    applyPersonaConfig(config);
  } catch (error) {
    console.warn("[hud] failed to load persona config", error);
  }
}

function applyPersonaConfig(config: PersonaConfig) {
  const enabledPersonas = config.items.filter((persona) => persona.enabled);
  personaOptions.value = enabledPersonas.length > 0 ? enabledPersonas : BUILT_IN_PERSONAS.filter((persona) => persona.enabled);

  if (!personaOptions.value.some((persona) => persona.id === selectedPersonaId.value)) {
    selectedPersonaId.value =
      personaOptions.value.find((persona) => persona.id === config.defaultSafePersonaId)?.id ??
      personaOptions.value[0]?.id ??
      DEFAULT_SAFE_PERSONA_ID;
  }
}

function applyHudUpdate(update: HudUpdate) {
  const previousPreviewId = previewId.value;
  setHud(update);
  busyAction.value = null;
  undoCopying.value = false;

  if (update.status === "preview") {
    if (shouldResetEditablePreviewText(update, previousPreviewId)) {
      editablePreviewText.value = update.message ?? "";
      previewView.value = update.originalText ? "diff" : "result";
    }
    void loadPersonaConfig();
  } else {
    editablePreviewText.value = "";
    previewView.value = "diff";
  }

  if (update.personaId) {
    selectedPersonaId.value = update.personaId;
  }
  if (update.status !== "preview") {
    personaSelectOpen.value = false;
  }
}

async function runPreviewAction(action: PreviewAction, personaId = selectedPersonaId.value) {
  if (!previewId.value || busyAction.value) {
    return;
  }

  busyAction.value = action;

  try {
    await invoke(previewActionCommand(action), {
      previewId: previewId.value,
      ...(action === "copy" || action === "replace" ? { editedText: editablePreviewText.value } : {}),
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

function personaLabel(persona: PersonaDefinition) {
  return personaDisplayName(persona, t);
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

async function copyUndoOriginal() {
  if (undoCopying.value) {
    return;
  }

  undoCopying.value = true;
  try {
    await invoke("copy_last_replacement_original");
  } catch (error) {
    console.warn("[hud] failed to copy previous original text", error);
    undoCopying.value = false;
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

function diffPartClass(kind: DiffPart["kind"]) {
  switch (kind) {
    case "added":
      return "bg-shanka-success/15 text-shanka-primary ring-1 ring-inset ring-shanka-success/25";
    case "removed":
      return "bg-shanka-danger/10 text-shanka-muted line-through decoration-shanka-danger/70 ring-1 ring-inset ring-shanka-danger/20";
    case "equal":
    default:
      return "text-shanka-secondary";
  }
}
</script>

<template>
  <div
    v-if="isPreview"
    class="relative flex h-[272px] w-[504px] flex-col overflow-hidden rounded-lg border border-shanka-border bg-shanka-panel text-shanka-primary shadow-2xl"
  >
    <div class="flex h-11 items-center justify-between gap-3 border-b border-shanka-border px-3">
      <div class="inline-flex h-8 rounded-md bg-shanka-input p-0.5 text-xs text-shanka-muted">
        <button
          class="inline-flex items-center gap-1.5 rounded px-2.5 transition-colors"
          :class="previewView === 'diff' ? 'bg-shanka-panel text-shanka-primary shadow-sm' : 'hover:text-shanka-primary'"
          :disabled="!canShowDiff"
          type="button"
          @click="previewView = 'diff'"
        >
          <FileDiff class="size-3.5" aria-hidden="true" />
          <span>{{ t("hud.preview.diff") }}</span>
        </button>
        <button
          class="inline-flex items-center gap-1.5 rounded px-2.5 transition-colors"
          :class="previewView === 'result' ? 'bg-shanka-panel text-shanka-primary shadow-sm' : 'hover:text-shanka-primary'"
          type="button"
          @click="previewView = 'result'"
        >
          <PenLine class="size-3.5" aria-hidden="true" />
          <span>{{ t("hud.preview.result") }}</span>
        </button>
      </div>

      <div class="flex min-w-0 items-center gap-2 text-[11px] text-shanka-muted">
        <span v-if="hasDiff" class="whitespace-nowrap text-shanka-success">+{{ currentDiffStats.added }}</span>
        <span v-if="hasDiff" class="whitespace-nowrap text-shanka-danger">-{{ currentDiffStats.removed }}</span>
        <span v-if="!hasDiff" class="truncate">{{ t("hud.preview.noChanges") }}</span>
      </div>
    </div>

    <div class="min-h-0 flex-1 px-3 py-3">
      <div
        v-if="previewView === 'diff'"
        class="h-full overflow-y-auto whitespace-pre-wrap break-words rounded-md bg-shanka-input/45 px-3 py-2 text-[13px] leading-6"
      >
        <template v-for="(part, index) in diffParts" :key="`${index}-${part.kind}`">
          <span
            v-if="part.text"
            class="rounded px-0.5 py-px"
            :class="diffPartClass(part.kind)"
          >{{ part.text }}</span>
        </template>
      </div>
      <textarea
        v-else
        v-model="editablePreviewText"
        class="h-full w-full resize-none bg-transparent text-[13px] leading-5 text-shanka-secondary outline-none placeholder:text-shanka-muted disabled:opacity-70"
        :disabled="busyAction !== null"
      />
    </div>

    <div
      v-if="previewErrorMessage"
      class="border-t border-shanka-border px-3 py-1 text-[11px] leading-4 text-shanka-danger"
    >
      {{ previewErrorMessage }}
    </div>

    <div class="flex h-11 items-center justify-between gap-2 border-t border-shanka-border px-2">
      <div class="relative w-44">
        <button
          class="flex h-8 w-full items-center justify-between gap-2 rounded-md border border-shanka-border bg-shanka-input px-2 text-left text-xs text-shanka-secondary transition-colors hover:text-shanka-primary"
          type="button"
          :aria-label="t('settings.field.activePersona')"
          @click="personaSelectOpen = !personaSelectOpen"
        >
          <span class="truncate">{{ personaLabel(selectedPersona) }}</span>
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
              <span class="truncate">{{ personaLabel(persona) }}</span>
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
    class="inline-flex h-9 max-w-[204px] items-center gap-2 rounded-md border border-shanka-border bg-shanka-panel px-3 text-xs font-medium text-shanka-secondary shadow-xl transition-colors"
  >
    <LoaderCircle v-if="isRefining" class="size-3.5 shrink-0 animate-spin text-shanka-primary" />
    <X v-else-if="isError" class="size-3.5 shrink-0 text-shanka-danger" />
    <Check v-else class="size-3.5 shrink-0 text-shanka-success" />
    <span class="truncate">{{ message }}</span>
    <button
      v-if="isUndoAvailable"
      class="-mr-1 inline-flex size-7 shrink-0 items-center justify-center rounded-md text-shanka-muted transition hover:bg-shanka-input hover:text-shanka-primary disabled:opacity-50"
      :title="t('hud.action.undo')"
      :aria-label="t('hud.action.undo')"
      :disabled="undoCopying"
      type="button"
      @click="copyUndoOriginal"
    >
      <LoaderCircle v-if="undoCopying" class="size-3.5 animate-spin" />
      <Undo2 v-else class="size-3.5" />
    </button>
  </div>
</template>
