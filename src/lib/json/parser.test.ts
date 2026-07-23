import { describe, expect, it } from "vitest";
import { formatJson } from "./format";
import { analyzeJson } from "./parser";

describe("analyzeJson", () => {
  it("parses standard JSON", () => {
    const result = analyzeJson('{"ok":true,"items":[1,2]}');
    expect(result.documents).toHaveLength(1);
    expect(result.documents[0].kind).toBe("json");
  });

  it("preserves integers larger than Number.MAX_SAFE_INTEGER", () => {
    const result = analyzeJson('{"orderId":9223372036854775807}');
    expect(formatJson(result.documents[0].value)).toContain(
      "9223372036854775807",
    );
  });

  it("decodes stringified JSON", () => {
    const result = analyzeJson('"{\\"code\\":0,\\"data\\":{\\"id\\":1}}"');
    expect(result.documents[0].kind).toBe("stringified");
    expect(result.documents[0].label).toContain("解码 1 层");
  });

  it("parses NDJSON into an array", () => {
    const result = analyzeJson('{"id":1}\n{"id":2}\n');
    expect(result.documents[0].kind).toBe("ndjson");
    expect(Array.isArray(result.documents[0].value)).toBe(true);
  });

  it("extracts multiple JSON fragments from logs", () => {
    const result = analyzeJson(
      'INFO request={"id":1} response={"code":0,"data":{"ok":true}}',
    );
    expect(result.documents).toHaveLength(2);
    expect(result.documents.every((document) => document.kind === "extracted")).toBe(
      true,
    );
  });

  it("returns a useful problem for invalid input", () => {
    const result = analyzeJson('{"broken": }');
    expect(result.documents).toHaveLength(0);
    expect(result.problem?.message).toBeTruthy();
  });

  it("rejects an empty clipboard", () => {
    const result = analyzeJson(" \n\t ");
    expect(result.problem?.message).toContain("没有文本");
  });
});
