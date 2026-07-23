import { isLosslessNumber, stringify } from "lossless-json";
import type { JsonValue } from "./types";

export function formatJson(value: JsonValue, indentation = 2): string {
  return stringify(value, null, indentation) ?? "";
}

export function compactJson(value: JsonValue): string {
  return stringify(value) ?? "";
}

export function getJsonType(value: JsonValue): string {
  if (value === null) return "null";
  if (isLosslessNumber(value) || typeof value === "number") return "number";
  if (Array.isArray(value)) return "array";
  return typeof value;
}

export function isJsonContainer(
  value: JsonValue,
): value is JsonValue[] | Record<string, JsonValue> {
  return Array.isArray(value) || (typeof value === "object" && value !== null && !isLosslessNumber(value));
}

export function valuePreview(value: JsonValue): string {
  if (typeof value === "string") return JSON.stringify(value);
  if (isLosslessNumber(value)) return value.value;
  if (Array.isArray(value)) return `Array(${value.length})`;
  if (value !== null && typeof value === "object") {
    return `Object(${Object.keys(value).length})`;
  }
  return String(value);
}

export function countNodes(value: JsonValue): number {
  let count = 0;
  const stack: JsonValue[] = [value];

  while (stack.length > 0) {
    const current = stack.pop()!;
    count += 1;
    if (Array.isArray(current)) {
      stack.push(...current);
    } else if (
      current !== null &&
      typeof current === "object" &&
      !isLosslessNumber(current)
    ) {
      stack.push(...Object.values(current));
    }
  }

  return count;
}

export function formatBytes(text: string): string {
  const bytes = new TextEncoder().encode(text).length;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function pathToJsonPath(path: Array<string | number>): string {
  return path.reduce<string>((result, segment) => {
    if (typeof segment === "number") return `${result}[${segment}]`;
    if (/^[A-Za-z_$][\w$]*$/.test(segment)) return `${result}.${segment}`;
    return `${result}[${JSON.stringify(segment)}]`;
  }, "$");
}

export interface SearchMatches {
  direct: Set<string>;
  branches: Set<string>;
  count: number;
}

export function pathKey(path: Array<string | number>): string {
  return JSON.stringify(path);
}

export function findSearchMatches(
  value: JsonValue,
  query: string,
  limit = 2000,
): SearchMatches {
  const direct = new Set<string>();
  const branches = new Set<string>();
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) return { direct, branches, count: 0 };

  const stack: Array<{
    value: JsonValue;
    path: Array<string | number>;
    name: string;
  }> = [{ value, path: [], name: "$" }];

  while (stack.length > 0 && direct.size < limit) {
    const current = stack.pop()!;
    const searchable = `${current.name} ${isJsonContainer(current.value) ? "" : valuePreview(current.value)}`
      .toLocaleLowerCase();

    if (searchable.includes(normalized)) {
      direct.add(pathKey(current.path));
      for (let length = 0; length <= current.path.length; length += 1) {
        branches.add(pathKey(current.path.slice(0, length)));
      }
    }

    if (Array.isArray(current.value)) {
      for (let index = current.value.length - 1; index >= 0; index -= 1) {
        stack.push({
          value: current.value[index],
          path: [...current.path, index],
          name: String(index),
        });
      }
    } else if (
      current.value !== null &&
      typeof current.value === "object" &&
      !isLosslessNumber(current.value)
    ) {
      const entries = Object.entries(current.value);
      for (let index = entries.length - 1; index >= 0; index -= 1) {
        const [name, child] = entries[index];
        stack.push({
          value: child,
          path: [...current.path, name],
          name,
        });
      }
    }
  }

  return { direct, branches, count: direct.size };
}
