import type { TranslationKey } from "../i18n/messages";
import type { ErrorCode, HudUpdate } from "../../shared";

export type PreviewAction = "copy" | "replace" | "regenerate";

export function hudMessageKey(update: Pick<HudUpdate, "status" | "errorCode">): TranslationKey {
  switch (update.status) {
    case "refining":
      return "hud.refining";
    case "replaced":
    case "undo_available":
      return "hud.replaced";
    case "error":
      return hudErrorMessageKey(update.errorCode);
    case "saved_to_clipboard":
      return "hud.savedToClipboard";
    case "ready":
    case "idle":
    default:
      return "hud.refining";
  }
}

export function hudErrorMessageKey(errorCode?: ErrorCode | null): TranslationKey {
  switch (errorCode) {
    case "NO_TEXT_SELECTED":
      return "hud.error.noTextSelected";
    case "NETWORK_TIMEOUT":
      return "hud.error.networkTimeout";
    case "API_CONFIG_MISSING":
      return "hud.error.apiConfigMissing";
    case "CLIPBOARD_ACCESS_FAILED":
      return "hud.error.clipboardAccessFailed";
    case "PASTE_BLOCKED":
      return "hud.error.pasteBlocked";
    case "PLATFORM_PERMISSION_REQUIRED":
      return "hud.error.platformPermissionRequired";
    case "API_ERROR":
    default:
      return "hud.error.apiError";
  }
}

export function previewActionCommand(action: PreviewAction) {
  switch (action) {
    case "copy":
      return "copy_safe_preview";
    case "replace":
      return "replace_safe_preview";
    case "regenerate":
      return "regenerate_safe_preview";
  }
}

export function shouldResetEditablePreviewText(
  update: HudUpdate,
  previousPreviewId: number | null,
) {
  return update.status === "preview" && (!update.errorCode || update.previewId !== previousPreviewId);
}
