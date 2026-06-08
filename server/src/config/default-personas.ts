import { BUILT_IN_PERSONAS } from "@shared";
import type { Persona } from "../services/persona-service";

export const DEFAULT_PERSONAS: Persona[] = BUILT_IN_PERSONAS.map((persona) => ({
  id: persona.id,
  name: persona.name,
  systemPrompt: persona.systemPrompt,
}));
