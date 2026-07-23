import type { AnalysisResult } from "./json/types";

export interface ClipboardHistoryEntry {
  id: string;
  createdAt: number;
  preview: string;
  bytes: number;
  kind: string;
  valid: boolean;
  text: string;
  fingerprint: string;
}

export function textFingerprint(text: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return `${text.length.toString(36)}-${(hash >>> 0).toString(36)}`;
}

export function createHistoryEntry(
  text: string,
  analysis: AnalysisResult,
  previous?: ClipboardHistoryEntry,
): ClipboardHistoryEntry {
  const compactPreview = text.replace(/\s+/g, " ").trim();
  const fingerprint = textFingerprint(text);

  return {
    id: previous?.id ?? `${Date.now().toString(36)}-${fingerprint}`,
    createdAt: previous?.createdAt ?? Date.now(),
    preview: compactPreview.slice(0, 160) || "空文本",
    bytes: new TextEncoder().encode(text).length,
    kind: analysis.documents[0]?.kind ?? "invalid",
    valid: analysis.documents.length > 0,
    text,
    fingerprint,
  };
}

export function formatHistoryTime(timestamp: number): string {
  const elapsed = Date.now() - timestamp;
  if (elapsed < 60_000) return "刚刚";
  if (elapsed < 3_600_000) return `${Math.floor(elapsed / 60_000)} 分钟前`;
  if (elapsed < 86_400_000) return `${Math.floor(elapsed / 3_600_000)} 小时前`;

  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(timestamp);
}
