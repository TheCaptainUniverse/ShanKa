import { describe, expect, test } from "bun:test";
import {
  applyProviderPresetToSettings,
  PROVIDER_PRESETS,
  providerTestErrorKey,
} from "../src/settings/provider";

const baseSettings = {
  provider: "custom",
  api_key: "",
  api_key_ref: "",
  base_url: "https://example.test/v1",
  model: "custom-model",
  timeout_ms: 8000,
  debug_logging: false,
  history_enabled: true,
};

describe("provider presets", () => {
  test("keeps provider ids unique", () => {
    const ids = PROVIDER_PRESETS.map((preset) => preset.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  test("keeps DeepSeek preset aligned with JSON mode provider behavior", () => {
    const deepseek = PROVIDER_PRESETS.find((preset) => preset.id === "deepseek");
    expect(deepseek).toEqual({
      id: "deepseek",
      label: "DeepSeek",
      base_url: "https://api.deepseek.com",
      model: "deepseek-v4-flash",
    });
  });

  test("applies known provider preset endpoint and model", () => {
    const next = applyProviderPresetToSettings(baseSettings, "openrouter");

    expect(next).toEqual({
      ...baseSettings,
      provider: "openrouter",
      base_url: "https://openrouter.ai/api/v1",
      model: "openai/gpt-4.1-mini",
    });
  });

  test("custom preset does not erase manually entered endpoint or model", () => {
    const next = applyProviderPresetToSettings(baseSettings, "custom");

    expect(next).toEqual({
      ...baseSettings,
      provider: "custom",
    });
  });

  test("unknown provider falls back to custom without overwriting endpoint or model", () => {
    const next = applyProviderPresetToSettings(baseSettings, "unknown-provider");

    expect(next).toEqual({
      ...baseSettings,
      provider: "custom",
    });
  });
});

describe("provider test error mapping", () => {
  test.each([
    ["api_key, base_url, and model are required", "settings.providerTest.missing"],
    ["PROVIDER_TEST_AUTH: unauthorized", "settings.providerTest.auth"],
    ["PROVIDER_TEST_MODEL: model not found", "settings.providerTest.model"],
    ["PROVIDER_TEST_NETWORK: dns failed", "settings.providerTest.network"],
    ["request timed out after 5000ms", "settings.providerTest.timeout"],
    ["system keychain is unavailable", "settings.providerTest.keychain"],
    ["provider returned HTTP 500", "settings.providerTest.remote"],
  ] as const)("maps %p to %p", (message, key) => {
    expect(providerTestErrorKey(new Error(message))).toBe(key);
  });
});
