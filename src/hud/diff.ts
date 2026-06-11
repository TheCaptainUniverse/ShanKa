export type DiffPart = {
  kind: "equal" | "added" | "removed";
  text: string;
};

export function buildTextDiff(originalText: string, revisedText: string): DiffPart[] {
  const originalTokens = tokenizeForDiff(originalText);
  const revisedTokens = tokenizeForDiff(revisedText);
  const rows = originalTokens.length + 1;
  const columns = revisedTokens.length + 1;
  const table = new Uint16Array(rows * columns);

  for (let row = originalTokens.length - 1; row >= 0; row -= 1) {
    for (let column = revisedTokens.length - 1; column >= 0; column -= 1) {
      const index = row * columns + column;
      table[index] =
        originalTokens[row] === revisedTokens[column]
          ? table[(row + 1) * columns + column + 1] + 1
          : Math.max(table[(row + 1) * columns + column], table[row * columns + column + 1]);
    }
  }

  const parts: DiffPart[] = [];
  let originalIndex = 0;
  let revisedIndex = 0;

  while (originalIndex < originalTokens.length && revisedIndex < revisedTokens.length) {
    if (originalTokens[originalIndex] === revisedTokens[revisedIndex]) {
      pushDiffPart(parts, "equal", originalTokens[originalIndex]);
      originalIndex += 1;
      revisedIndex += 1;
    } else if (
      table[(originalIndex + 1) * columns + revisedIndex] >=
      table[originalIndex * columns + revisedIndex + 1]
    ) {
      pushDiffPart(parts, "removed", originalTokens[originalIndex]);
      originalIndex += 1;
    } else {
      pushDiffPart(parts, "added", revisedTokens[revisedIndex]);
      revisedIndex += 1;
    }
  }

  while (originalIndex < originalTokens.length) {
    pushDiffPart(parts, "removed", originalTokens[originalIndex]);
    originalIndex += 1;
  }

  while (revisedIndex < revisedTokens.length) {
    pushDiffPart(parts, "added", revisedTokens[revisedIndex]);
    revisedIndex += 1;
  }

  return parts;
}

export function diffStats(parts: DiffPart[]) {
  return parts.reduce(
    (stats, part) => {
      if (part.kind === "added") {
        stats.added += part.text.trim() ? 1 : 0;
      }
      if (part.kind === "removed") {
        stats.removed += part.text.trim() ? 1 : 0;
      }
      return stats;
    },
    { added: 0, removed: 0 },
  );
}

function tokenizeForDiff(text: string) {
  const tokens = text.match(/\r\n|\n|\s+|[\p{Script=Han}]|[\p{L}\p{N}_]+|[^\s\p{L}\p{N}_]/gu);
  return tokens ?? [];
}

function pushDiffPart(parts: DiffPart[], kind: DiffPart["kind"], text: string) {
  if (!text) {
    return;
  }

  const lastPart = parts[parts.length - 1];
  if (lastPart?.kind === kind) {
    lastPart.text += text;
    return;
  }

  parts.push({ kind, text });
}
