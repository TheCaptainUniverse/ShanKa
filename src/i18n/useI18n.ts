import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_LOCALE, LOCALES, messages } from "./messages";
import type { Locale, TranslationKey } from "./messages";

const STORAGE_KEY = "shanka.locale";

const locale = ref<Locale>(readStoredLocale());

export function useI18n() {
  function t(key: TranslationKey) {
    return messages[locale.value][key];
  }

  function setLocale(nextLocale: Locale) {
    locale.value = nextLocale;
    localStorage.setItem(STORAGE_KEY, nextLocale);
    void syncTrayLocale(nextLocale);
  }

  return {
    locale,
    locales: LOCALES,
    setLocale,
    t,
  };
}

function readStoredLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  return isLocale(stored) ? stored : DEFAULT_LOCALE;
}

function isLocale(value: string | null): value is Locale {
  return LOCALES.some((localeOption) => localeOption === value);
}

async function syncTrayLocale(nextLocale: Locale) {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    return;
  }

  try {
    await invoke("update_tray_locale", { locale: nextLocale });
  } catch (error) {
    console.warn("[i18n] failed to update tray locale", error);
  }
}

void syncTrayLocale(locale.value);
