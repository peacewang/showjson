import { parse } from "lossless-json";
import type {
  AnalysisResult,
  JsonDocument,
  JsonValue,
  ParseProblem,
} from "./types";

const MAX_INPUT_BYTES = 50 * 1024 * 1024;
const MAX_EXTRACTED_DOCUMENTS = 20;
const MAX_STRINGIFY_DEPTH = 3;

function strictParse(text: string): JsonValue {
  return parse(text) as JsonValue;
}

function trimInput(text: string): { text: string; offset: number } {
  const withoutBom = text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  const leading = withoutBom.match(/^\s*/)?.[0].length ?? 0;
  return {
    text: withoutBom.trim(),
    offset: leading + (withoutBom.length === text.length ? 0 : 1),
  };
}

function looksLikeNestedJson(value: string): boolean {
  const trimmed = value.trim();
  return (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]")) ||
    (trimmed.startsWith('"') && trimmed.endsWith('"'))
  );
}

function parseWithStringLayers(text: string): {
  value: JsonValue;
  layers: number;
} {
  let value = strictParse(text);
  let layers = 0;

  while (
    typeof value === "string" &&
    layers < MAX_STRINGIFY_DEPTH &&
    looksLikeNestedJson(value)
  ) {
    try {
      value = strictParse(value.trim());
      layers += 1;
    } catch {
      break;
    }
  }

  return { value, layers };
}

function positionToProblem(
  error: unknown,
  text: string,
  fallbackMessage = "不是有效的 JSON",
): ParseProblem {
  const originalMessage = error instanceof Error ? error.message : String(error);
  const match = originalMessage.match(
    /(?:position|character|char)\s+(\d+)/i,
  );
  const position = match ? Number(match[1]) : undefined;
  let line: number | undefined;
  let column: number | undefined;

  if (position !== undefined && Number.isFinite(position)) {
    const before = text.slice(0, position);
    const lines = before.split(/\r\n|\r|\n/);
    line = lines.length;
    column = (lines.at(-1)?.length ?? 0) + 1;
  }

  return {
    message: originalMessage || fallbackMessage,
    position,
    line,
    column,
  };
}

function parseNdjson(text: string): JsonDocument | undefined {
  const lines = text.split(/\r\n|\r|\n/);
  const nonEmpty = lines
    .map((line, index) => ({ text: line.trim(), index }))
    .filter((line) => line.text.length > 0);

  if (nonEmpty.length < 2) return undefined;

  const values: JsonValue[] = [];
  for (const line of nonEmpty) {
    try {
      values.push(strictParse(line.text));
    } catch {
      return undefined;
    }
  }

  return {
    id: "ndjson",
    label: `JSON Lines · ${values.length} 条`,
    kind: "ndjson",
    value: values,
    rawText: text,
    sourceStart: 0,
    sourceEnd: text.length,
  };
}

interface Candidate {
  start: number;
  end: number;
  rawText: string;
  value: JsonValue;
}

function extractCandidates(text: string): Candidate[] {
  const stack: Array<{ char: "{" | "["; start: number }> = [];
  const candidates: Candidate[] = [];
  let inString = false;
  let escaped = false;

  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];

    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === '"') {
        inString = false;
      }
      continue;
    }

    if (char === '"') {
      inString = true;
      continue;
    }

    if (char === "{" || char === "[") {
      stack.push({ char, start: index });
      continue;
    }

    if (char !== "}" && char !== "]") continue;

    const expected = char === "}" ? "{" : "[";
    const top = stack.at(-1);
    if (!top || top.char !== expected) {
      stack.length = 0;
      continue;
    }

    stack.pop();
    const rawText = text.slice(top.start, index + 1);
    try {
      const value = strictParse(rawText);
      if (typeof value === "object" && value !== null) {
        candidates.push({
          start: top.start,
          end: index + 1,
          rawText,
          value,
        });
      }
    } catch {
      // A balanced bracket pair is only a candidate; invalid pairs are ignored.
    }
  }

  const sorted = candidates.sort(
    (left, right) =>
      left.start - right.start || right.end - right.start - (left.end - left.start),
  );
  const outermost: Candidate[] = [];

  for (const candidate of sorted) {
    const contained = outermost.some(
      (existing) =>
        candidate.start >= existing.start && candidate.end <= existing.end,
    );
    if (!contained) outermost.push(candidate);
    if (outermost.length >= MAX_EXTRACTED_DOCUMENTS) break;
  }

  return outermost;
}

function lineAt(text: string, position: number): number {
  return text.slice(0, position).split(/\r\n|\r|\n/).length;
}

export function analyzeJson(inputText: string): AnalysisResult {
  const startedAt = performance.now();
  const bytes = new TextEncoder().encode(inputText).length;

  if (bytes > MAX_INPUT_BYTES) {
    return {
      inputText,
      documents: [],
      problem: {
        message: "当前版本单次最多处理 50 MB 文本，请缩小输入后重试。",
      },
      elapsedMs: performance.now() - startedAt,
    };
  }

  const normalized = trimInput(inputText);
  if (!normalized.text) {
    return {
      inputText,
      documents: [],
      problem: { message: "剪贴板中没有文本。" },
      elapsedMs: performance.now() - startedAt,
    };
  }

  try {
    const parsed = parseWithStringLayers(normalized.text);
    const document: JsonDocument = {
      id: "document-1",
      label: parsed.layers > 0 ? `转义 JSON · 解码 ${parsed.layers} 层` : "JSON",
      kind: parsed.layers > 0 ? "stringified" : "json",
      value: parsed.value,
      rawText: normalized.text,
      sourceStart: normalized.offset,
      sourceEnd: normalized.offset + normalized.text.length,
    };

    return {
      inputText,
      documents: [document],
      elapsedMs: performance.now() - startedAt,
    };
  } catch (directError) {
    const ndjson = parseNdjson(normalized.text);
    if (ndjson) {
      return {
        inputText,
        documents: [ndjson],
        elapsedMs: performance.now() - startedAt,
      };
    }

    const candidates = extractCandidates(inputText);
    if (candidates.length > 0) {
      return {
        inputText,
        documents: candidates.map((candidate, index) => ({
          id: `fragment-${index + 1}`,
          label: `JSON 片段 ${index + 1} · 第 ${lineAt(inputText, candidate.start)} 行`,
          kind: "extracted",
          value: candidate.value,
          rawText: candidate.rawText,
          sourceStart: candidate.start,
          sourceEnd: candidate.end,
        })),
        elapsedMs: performance.now() - startedAt,
      };
    }

    return {
      inputText,
      documents: [],
      problem: positionToProblem(directError, normalized.text),
      elapsedMs: performance.now() - startedAt,
    };
  }
}
