export interface TextSearchSegment {
  text: string;
  matchIndex?: number;
}

export interface TextSearchResult {
  segments: TextSearchSegment[];
  count: number;
  limited: boolean;
}

export function findTextMatches(
  text: string,
  query: string,
  limit = 2000,
): TextSearchResult {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle || !text) {
    return {
      segments: [{ text }],
      count: 0,
      limited: false,
    };
  }

  const searchable = text.toLocaleLowerCase();
  const segments: TextSearchSegment[] = [];
  let cursor = 0;
  let count = 0;

  while (cursor < text.length && count < limit) {
    const matchStart = searchable.indexOf(needle, cursor);
    if (matchStart < 0) break;

    if (matchStart > cursor) {
      segments.push({ text: text.slice(cursor, matchStart) });
    }

    const matchEnd = matchStart + needle.length;
    segments.push({
      text: text.slice(matchStart, matchEnd),
      matchIndex: count,
    });
    count += 1;
    cursor = matchEnd;
  }

  const limited =
    count === limit && searchable.indexOf(needle, cursor) >= 0;
  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor) });
  }

  return { segments, count, limited };
}
