import { DEFAULT_PERSONAS } from "../config/default-personas";

export interface Persona {
  id: string;
  name: string;
  systemPrompt: string;
}

export async function getActivePersona(personaId?: string): Promise<Persona> {
  return DEFAULT_PERSONAS.find((persona) => persona.id === personaId) ?? DEFAULT_PERSONAS[0];
}
