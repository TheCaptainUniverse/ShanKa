import { DEFAULT_PERSONAS } from "../config/default-personas";

export function getInitialPersonaRows(now = new Date()) {
  return DEFAULT_PERSONAS.map((persona, index) => ({
    id: persona.id,
    name: persona.name,
    systemPrompt: persona.systemPrompt,
    isBuiltIn: true,
    isActive: index === 0,
    sortOrder: index,
    createdAt: now,
    updatedAt: now,
  }));
}
