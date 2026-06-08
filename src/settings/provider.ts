import type { TranslationKey } from "../i18n/messages";

export type ProviderSettings = {
  provider: string;
  base_url: string;
  model: string;
};

export type ProviderPreset = {
  id: string;
  label: string;
  base_url: string;
  model: string;
};

export const PROVIDER_PRESETS = [
  {
    id: "deepseek",
    label: "DeepSeek",
    base_url: "https://api.deepseek.com",
    model: "deepseek-v4-flash",
  },
  {
    id: "openai",
    label: "OpenAI",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4.1-mini",
  },
  {
    id: "openrouter",
    label: "OpenRouter",
    base_url: "https://openrouter.ai/api/v1",
    model: "openai/gpt-4.1-mini",
  },
  {
    id: "custom",
    label: "Custom",
    base_url: "",
    model: "",
  },
] as const satisfies readonly ProviderPreset[];

export function applyProviderPresetToSettings<TSettings extends ProviderSettings>(
  settings: TSettings,
  providerId: string,
): TSettings {
  const preset = PROVIDER_PRESETS.find((item) => item.id === providerId);

  if (!preset) {
    return {
      ...settings,
      provider: "custom",
    };
  }

  if (preset.id === "custom") {
    return {
      ...settings,
      provider: preset.id,
    };
  }

  return {
    ...settings,
    provider: preset.id,
    base_url: preset.base_url,
    model: preset.model,
  };
}

export function providerTestErrorKey(error: unknown): TranslationKey {
  const message = error instanceof Error ? error.message : String(error);

  if (message.includes("api_key, base_url, and model are required")) {
    return "settings.providerTest.missing";
  }
  if (message.includes("PROVIDER_TEST_AUTH")) {
    return "settings.providerTest.auth";
  }
  if (message.includes("PROVIDER_TEST_MODEL")) {
    return "settings.providerTest.model";
  }
  if (message.includes("PROVIDER_TEST_NETWORK")) {
    return "settings.providerTest.network";
  }
  if (message.includes("request timed out")) {
    return "settings.providerTest.timeout";
  }
  if (message.includes("system keychain") || message.includes("keychain")) {
    return "settings.providerTest.keychain";
  }

  return "settings.providerTest.remote";
}
