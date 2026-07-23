import type { LosslessNumber } from "lossless-json";

export type JsonPrimitive = null | boolean | string | number | LosslessNumber;
export type JsonValue =
  | JsonPrimitive
  | JsonValue[]
  | { [key: string]: JsonValue };

export type DocumentKind = "json" | "stringified" | "ndjson" | "extracted";

export interface JsonDocument {
  id: string;
  label: string;
  kind: DocumentKind;
  value: JsonValue;
  rawText: string;
  sourceStart: number;
  sourceEnd: number;
}

export interface ParseProblem {
  message: string;
  position?: number;
  line?: number;
  column?: number;
}

export interface AnalysisResult {
  inputText: string;
  documents: JsonDocument[];
  problem?: ParseProblem;
  elapsedMs: number;
}
