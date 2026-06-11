import personasData from "./personas.json";

export interface PersonaDefinition {
  id: string;
  name: string;
  description: string;
  nameKey: string;
  descriptionKey: string;
  systemPrompt: string;
  builtIn: boolean;
  enabled: boolean;
}

export interface PersonaConfig {
  defaultSafePersonaId: string;
  items: PersonaDefinition[];
  deletedBuiltInPersonaIds: string[];
}

interface PersonaCatalog {
  defaultSafePersonaId: string;
  personas: PersonaDefinition[];
}

const catalog = personasData as PersonaCatalog;

export const DEFAULT_SAFE_PERSONA_ID = catalog.defaultSafePersonaId;

export const BUILT_IN_PERSONAS = catalog.personas;

export const ENABLED_PERSONAS = BUILT_IN_PERSONAS.filter((persona) => persona.enabled);

export type PersonaId = (typeof BUILT_IN_PERSONAS)[number]["id"];

export function resolvePersonaDefinition(personaId?: string) {
  return (
    ENABLED_PERSONAS.find((persona) => persona.id === personaId) ??
    ENABLED_PERSONAS.find((persona) => persona.id === DEFAULT_SAFE_PERSONA_ID) ??
    ENABLED_PERSONAS[0]
  );
}
