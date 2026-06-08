import { ref } from "vue";

export const THEMES = ["light", "dark"] as const;

export type Theme = (typeof THEMES)[number];

export const DEFAULT_THEME: Theme = "dark";

const STORAGE_KEY = "shanka.theme";

const theme = ref<Theme>(readStoredTheme());

applyTheme(theme.value);

export function useTheme() {
  function setTheme(nextTheme: Theme) {
    theme.value = nextTheme;
    localStorage.setItem(STORAGE_KEY, nextTheme);
    applyTheme(nextTheme);
  }

  return {
    setTheme,
    theme,
    themes: THEMES,
  };
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
