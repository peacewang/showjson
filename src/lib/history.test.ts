import { describe, expect, it } from "vitest";
import { createHistoryEntry, textFingerprint } from "./history";
import { analyzeJson } from "./json/parser";

describe("clipboard history", () => {
  it("creates metadata for valid and invalid clipboard text", () => {
    const validText = '{"id":9223372036854775807}';
    const valid = createHistoryEntry(validText, analyzeJson(validText));
    expect(valid.valid).toBe(true);
    expect(valid.kind).toBe("json");
    expect(valid.bytes).toBeGreaterThan(0);

    const invalidText = '{"id": }';
    const invalid = createHistoryEntry(invalidText, analyzeJson(invalidText));
    expect(invalid.valid).toBe(false);
    expect(invalid.kind).toBe("invalid");
  });

  it("uses a stable content fingerprint for deduplication", () => {
    expect(textFingerprint('{"same":true}')).toBe(
      textFingerprint('{"same":true}'),
    );
    expect(textFingerprint('{"same":true}')).not.toBe(
      textFingerprint('{"same":false}'),
    );
  });
});
