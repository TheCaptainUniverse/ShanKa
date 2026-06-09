import { ref } from "vue";
import { emit, listen } from "@tauri-apps/api/event";
import { TAURI_EVENTS } from "@shared";

export const THEMES = ["light", "dark"] as const;

export type Theme = (typeof THEMES)[number];
type ThemeChangedPayload = { theme: Theme };

export const DEFAULT_THEME: Theme = "dark";

const STORAGE_KEY = "shanka.theme";

const theme = ref<Theme>(readStoredTheme());
let themeSyncStarted = false;

applyTheme(theme.value);

export function useTheme() {
  startThemeSync();

  function setTheme(nextTheme: Theme) {
    setThemePreference(nextTheme, true);
  }

  return {
    setTheme,
    theme,
    themes: THEMES,
  };
}

function startThemeSync() {
  if (themeSyncStarted) {
    return;
  }
  themeSyncStarted = true;

  window.addEventListener("storage", (event) => {
    if (event.key === STORAGE_KEY && isTheme(event.newValue)) {
      setThemePreference(event.newValue, false);
    }
  });

  void listen<ThemeChangedPayload>(TAURI_EVENTS.themeChanged, (event) => {
    if (isTheme(event.payload.theme)) {
      setThemePreference(event.payload.theme, false);
    }
  }).catch((error) => {
    console.warn("[theme] failed to start theme sync", error);
  });
}

function setThemePreference(nextTheme: Theme, broadcast: boolean) {
  theme.value = nextTheme;
  localStorage.setItem(STORAGE_KEY, nextTheme);
  applyTheme(nextTheme);

  if (broadcast) {
    void emit(TAURI_EVENTS.themeChanged, { theme: nextTheme }).catch((error) => {
      console.warn("[theme] failed to broadcast theme change", error);
    });
  }
}

function applyTheme(nextTheme: Theme) {
  document.documentElement.dataset.theme = nextTheme;
}

function readStoredTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY);
  return isTheme(stored) ? stored : DEFAULT_THEME;
}

function isTheme(value: string | null): value is Theme {
  return THEMES.some((themeOption) => themeOption === value);
}
