import { describe, expect, test } from "bun:test";
import { buildTextDiff, diffStats } from "../src/hud/diff";

describe("HUD diff", () => {
  test("marks added and removed text while preserving equal text", () => {
    const parts = buildTextDiff("Hello world.", "Hello brave world!");

    expect(parts).toEqual([
      { kind: "equal", text: "Hello " },
      { kind: "added", text: "brave " },
      { kind: "equal", text: "world" },
      { kind: "removed", text: "." },
      { kind: "added", text: "!" },
    ]);
  });

  test("handles Chinese text at character granularity", () => {
    const parts = buildTextDiff("专业技能", "核心专业技能");

    expect(parts).toEqual([
      { kind: "added", text: "核心" },
      { kind: "equal", text: "专业技能" },
    ]);
  });

  test("summarizes visible change chunks", () => {
    const parts = buildTextDiff("A B C", "A X C D");

    expect(diffStats(parts)).toEqual({
      added: 2,
      removed: 1,
    });
  });
});
