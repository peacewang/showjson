import { describe, expect, it } from "vitest";
import { findTextMatches } from "./text-search";

describe("pretty text search", () => {
  it("finds case-insensitive non-overlapping matches", () => {
    const result = findTextMatches('{"Name":"showjson","name":"JSON"}', "name");

    expect(result.count).toBe(2);
    expect(
      result.segments.filter((segment) => segment.matchIndex !== undefined),
    ).toEqual([
      { text: "Name", matchIndex: 0 },
      { text: "name", matchIndex: 1 },
    ]);
  });

  it("returns the original text for an empty query", () => {
    expect(findTextMatches("example", " ")).toEqual({
      segments: [{ text: "example" }],
      count: 0,
      limited: false,
    });
  });

  it("caps the number of highlighted matches", () => {
    const result = findTextMatches("a a a", "a", 2);

    expect(result.count).toBe(2);
    expect(result.limited).toBe(true);
  });
});
