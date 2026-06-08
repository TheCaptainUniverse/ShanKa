import { describe, expect, test } from "bun:test";
import {
  ERROR_CODES,
  HUD_STATUSES,
  REFINE_MODES,
  TAURI_EVENTS,
} from "../shared";

function expectUnique(values: readonly string[]) {
  expect(new Set(values).size).toBe(values.length);
}

describe("shared contracts", () => {
  test("keeps refine modes stable for Rust and frontend mapping", () => {
    expect(REFINE_MODES).toEqual(["safe", "magic"]);
    expectUnique(REFINE_MODES);
  });

  test("keeps HUD statuses unique and includes preview lifecycle states", () => {
    expectUnique(HUD_STATUSES);
    expect(HUD_STATUSES).toContain("refining");
    expect(HUD_STATUSES).toContain("preview");
    expect(HUD_STATUSES).toContain("undo_available");
    expect(HUD_STATUSES).toContain("saved_to_clipboard");
  });

  test("keeps error codes unique and covers platform fallbacks", () => {
    expectUnique(ERROR_CODES);
    expect(ERROR_CODES).toContain("NO_TEXT_SELECTED");
    expect(ERROR_CODES).toContain("PASTE_BLOCKED");
    expect(ERROR_CODES).toContain("PLATFORM_PERMISSION_REQUIRED");
  });

  test("keeps Tauri event names stable", () => {
    expect(TAURI_EVENTS).toEqual({
      hudUpdate: "hud:update",
      refineStarted: "refine:started",
      refineFinished: "refine:finished",
    });
  });
});
