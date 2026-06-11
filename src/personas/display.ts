import { BUILT_IN_PERSONAS, type PersonaDefinition } from "@shared";
import { DEFAULT_LOCALE, messages, type TranslationKey } from "@/i18n/messages";

type Translate = (key: TranslationKey) => string;
type PersonaTextField = "name" | "description";
type PersonaKeyField = "nameKey" | "descriptionKey";

const builtInPersonasById = new Map(BUILT_IN_PERSONAS.map((persona) => [persona.id, persona]));

export function personaDisplayName(persona: PersonaDefinition, t: Translate) {
  return personaDisplayField(persona, "name", "nameKey", t);
}

export function personaDisplayDescription(persona: PersonaDefinition, t: Translate) {
  return personaDisplayField(persona, "description", "descriptionKey", t);
}

export function personaStorageName(persona: PersonaDefinition) {
  return personaStorageField(persona, "name", "nameKey");
}

export function personaStorageDescription(persona: PersonaDefinition) {
  return personaStorageField(persona, "description", "descriptionKey");
}

function personaDisplayField(
  persona: PersonaDefinition,
  textField: PersonaTextField,
  keyField: PersonaKeyField,
  t: Translate,
) {
  const value = personaFieldValue(persona, textField);
  const key = persona[keyField].trim();

  if (isBuiltInPersonaField(persona, keyField)) {
    return translatePersonaKey(key, t);
  }

  return value || translatePersonaKey(key, t);
}

function personaStorageField(
  persona: PersonaDefinition,
  textField: PersonaTextField,
  keyField: PersonaKeyField,
) {
  if (isBuiltInPersonaField(persona, keyField)) {
    return builtInPersonaFieldValue(persona, textField);
  }

  return personaFieldValue(persona, textField);
}

function isBuiltInPersonaField(persona: PersonaDefinition, keyField: PersonaKeyField) {
  const builtInPersona = builtInPersonasById.get(persona.id);
  const key = persona[keyField].trim();

  return persona.builtIn && Boolean(builtInPersona) && key !== "" && key === builtInPersona?.[keyField].trim();
}

function builtInPersonaFieldValue(persona: PersonaDefinition, textField: PersonaTextField) {
  return personaFieldValue(builtInPersonasById.get(persona.id) ?? persona, textField);
}

function personaFieldValue(persona: PersonaDefinition, textField: PersonaTextField) {
  return persona[textField].trim();
}

function translatePersonaKey(key: string, t: Translate) {
  return isTranslationKey(key) ? t(key) : "";
}

function isTranslationKey(key: string): key is TranslationKey {
  return key in messages[DEFAULT_LOCALE];
}
