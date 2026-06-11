import type { PersonaId } from "./personas";

export const REFINE_MODES = ["safe", "magic"] as const;

export type RefineMode = (typeof REFINE_MODES)[number];

export const HUD_STATUSES = [
  "idle",
  "refining",
  "preview",
  "ready",
  "replaced",
  "undo_available",
  "error",
  "saved_to_clipboard",
] as const;

export type HudStatus = (typeof HUD_STATUSES)[number];

export const ERROR_CODES = [
  "NO_TEXT_SELECTED",
  "NETWORK_TIMEOUT",
  "API_ERROR",
  "API_CONFIG_MISSING",
  "PROVIDER_RESPONSE_INVALID",
  "PASTE_BLOCKED",
  "CLIPBOARD_ACCESS_FAILED",
  "PLATFORM_PERMISSION_REQUIRED",
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];

export interface RefineRequest {
  text: string;
  mode: RefineMode;
  personaId?: PersonaId;
}

export interface RefineResponse {
  ok: boolean;
  text?: string;
  durationMs: number;
  errorCode?: ErrorCode;
  message?: string;
}

export interface HudUpdate {
  status: HudStatus;
  message?: string;
  originalText?: string | null;
  errorCode?: ErrorCode | null;
  previewId?: number | null;
  personaId?: string | null;
}
