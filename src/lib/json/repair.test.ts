import { describe, expect, it } from "vitest";
import { analyzeJson } from "./parser";
import { repairJson } from "./repair";

describe("repairJson", () => {
  it("repairs common JSON syntax errors only when explicitly called", () => {
    const invalid = "{name: 'ShowJSON', enabled: true,}";
    expect(analyzeJson(invalid).documents).toHaveLength(0);

    const repaired = repairJson(invalid);
    expect(analyzeJson(repaired).documents).toHaveLength(1);
    expect(repaired).toContain('"name": "ShowJSON"');
  });

  it("throws when text cannot be repaired", () => {
    expect(() => repairJson("\\uZZZZ")).toThrow();
  });
});
