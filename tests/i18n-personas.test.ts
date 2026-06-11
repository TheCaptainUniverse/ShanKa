import { describe, expect, test } from "bun:test";
import {
  BUILT_IN_PERSONAS,
  DEFAULT_SAFE_PERSONA_ID,
  ENABLED_PERSONAS,
  type PersonaDefinition,
  resolvePersonaDefinition,
} from "../shared";
import { personaDisplayName, personaStorageName } from "../src/personas/display";
import { DEFAULT_LOCALE, LOCALES, messages, type TranslationKey } from "../src/i18n/messages";

function keysFor(locale: (typeof LOCALES)[number]) {
  return Object.keys(messages[locale]).sort();
}

function zh(key: TranslationKey) {
  return messages["zh-CN"][key];
}

describe("i18n catalog", () => {
  test("defaults to Chinese", () => {
    expect(DEFAULT_LOCALE).toBe("zh-CN");
  });

  test("keeps translation keys aligned between supported locales", () => {
    const defaultKeys = keysFor(DEFAULT_LOCALE);

    for (const locale of LOCALES) {
      expect(keysFor(locale)).toEqual(defaultKeys);
    }
  });

  test("contains non-empty translations for every key", () => {
    for (const locale of LOCALES) {
      for (const [key, value] of Object.entries(messages[locale])) {
        expect(value.trim(), `${locale}:${key}`).not.toBe("");
      }
    }
  });
});

describe("persona catalog", () => {
  test("keeps built-in persona ids stable", () => {
    expect(BUILT_IN_PERSONAS.map((persona) => persona.id)).toEqual([
      "workplace-eq",
      "academic-concise",
      "clean-correction",
      "translation-zh",
      "vibecoding-requirements",
    ]);
  });

  test("includes developer and translation helper personas", () => {
    expect(BUILT_IN_PERSONAS.find((persona) => persona.id === "translation-zh")?.systemPrompt).toContain(
      "Simplified Chinese",
    );
    expect(BUILT_IN_PERSONAS.find((persona) => persona.id === "vibecoding-requirements")?.systemPrompt).toContain(
      "vibecoding-ready requirement",
    );
  });

  test("uses an enabled built-in persona as the Safe Mode default", () => {
    expect(DEFAULT_SAFE_PERSONA_ID).toBe("clean-correction");
    expect(ENABLED_PERSONAS.some((persona) => persona.id === DEFAULT_SAFE_PERSONA_ID)).toBe(true);
    expect(resolvePersonaDefinition().id).toBe(DEFAULT_SAFE_PERSONA_ID);
    expect(resolvePersonaDefinition("missing-persona").id).toBe(DEFAULT_SAFE_PERSONA_ID);
  });

  test("has localized labels for every built-in persona", () => {
    for (const persona of BUILT_IN_PERSONAS) {
      expect(persona.builtIn).toBe(true);
      expect(persona.systemPrompt.trim()).not.toBe("");

      for (const locale of LOCALES) {
        expect(messages[locale][persona.nameKey], `${locale}:${persona.nameKey}`).toBeDefined();
        expect(
          messages[locale][persona.descriptionKey],
          `${locale}:${persona.descriptionKey}`,
        ).toBeDefined();
      }
    }
  });

  test("uses Chinese as the built-in persona catalog default", () => {
    for (const persona of BUILT_IN_PERSONAS) {
      expect(persona.name).toBe(messages[DEFAULT_LOCALE][persona.nameKey]);
      expect(persona.description).toBe(messages[DEFAULT_LOCALE][persona.descriptionKey]);
    }
  });

  test("localizes built-in personas with legacy English config names", () => {
    const legacyNames: Record<string, string> = {
      "translation-zh": "Chinese Translation Assistant",
      "vibecoding-requirements": "Vibecoding Requirement Refiner",
    };

    for (const [personaId, legacyName] of Object.entries(legacyNames)) {
      const persona = BUILT_IN_PERSONAS.find((item) => item.id === personaId) as PersonaDefinition;
      const legacyPersona = { ...persona, name: legacyName };

      expect(personaDisplayName(legacyPersona, zh)).toBe(messages["zh-CN"][persona.nameKey]);
      expect(personaStorageName(legacyPersona)).toBe(messages["zh-CN"][persona.nameKey]);
    }
  });
});
