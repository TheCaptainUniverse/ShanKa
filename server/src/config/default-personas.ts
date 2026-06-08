import type { Persona } from "../services/persona-service";

export const DEFAULT_PERSONAS: Persona[] = [
  {
    id: "workplace-eq",
    name: "High-EQ Workplace",
    systemPrompt: "Rewrite plain or emotional wording into tactful workplace communication.",
  },
  {
    id: "academic-concise",
    name: "Academic Concise",
    systemPrompt: "Remove colloquial wording, compress length, and keep academic rigor.",
  },
  {
    id: "clean-correction",
    name: "Clean Correction",
    systemPrompt: "Correct typos, punctuation, and formatting without changing the author's voice.",
  },
];
