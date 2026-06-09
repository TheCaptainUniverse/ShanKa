import { describe, expect, test } from "bun:test";
import type { ErrorCode, HudUpdate } from "../shared";
import {
  hudErrorMessageKey,
  hudMessageKey,
  previewActionCommand,
  shouldResetEditablePreviewText,
} from "../src/hud/state";

describe("HUD message state", () => {
  test.each([
    [{ status: "refining" }, "hud.refining"],
    [{ status: "replaced" }, "hud.replaced"],
    [{ status: "undo_available" }, "hud.replaced"],
    [{ status: "saved_to_clipboard" }, "hud.savedToClipboard"],
    [{ status: "idle" }, "hud.refining"],
  ] as const)("maps %p to %p", (update, key) => {
    expect(hudMessageKey(update)).toBe(key);
  });

  test.each([
    ["NO_TEXT_SELECTED", "hud.error.noTextSelected"],
    ["NETWORK_TIMEOUT", "hud.error.networkTimeout"],
    ["API_CONFIG_MISSING", "hud.error.apiConfigMissing"],
    ["PROVIDER_RESPONSE_INVALID", "hud.error.providerResponseInvalid"],
    ["CLIPBOARD_ACCESS_FAILED", "hud.error.clipboardAccessFailed"],
    ["PASTE_BLOCKED", "hud.error.pasteBlocked"],
    ["PLATFORM_PERMISSION_REQUIRED", "hud.error.platformPermissionRequired"],
    ["API_ERROR", "hud.error.apiError"],
    [null, "hud.error.apiError"],
  ] as const)("maps error code %p to %p", (errorCode, key) => {
    expect(hudErrorMessageKey(errorCode as ErrorCode | null)).toBe(key);
    expect(hudMessageKey({ status: "error", errorCode: errorCode as ErrorCode | null })).toBe(key);
  });
});

describe("HUD preview actions", () => {
  test.each([
    ["copy", "copy_safe_preview"],
    ["replace", "replace_safe_preview"],
    ["regenerate", "regenerate_safe_preview"],
  ] as const)("maps %p action to %p command", (action, command) => {
    expect(previewActionCommand(action)).toBe(command);
  });

  test("resets editable text for a fresh preview", () => {
    const update: HudUpdate = {
      status: "preview",
      message: "next",
      errorCode: null,
      previewId: 2,
    };

    expect(shouldResetEditablePreviewText(update, 1)).toBe(true);
  });

  test("keeps edited text when current preview receives an inline error", () => {
    const update: HudUpdate = {
      status: "preview",
      message: "old preview text",
      errorCode: "API_ERROR",
      previewId: 2,
    };

    expect(shouldResetEditablePreviewText(update, 2)).toBe(false);
  });

  test("resets editable text when an errored preview belongs to a newer preview id", () => {
    const update: HudUpdate = {
      status: "preview",
      message: "new preview text",
      errorCode: "API_ERROR",
      previewId: 3,
    };

    expect(shouldResetEditablePreviewText(update, 2)).toBe(true);
  });
});
