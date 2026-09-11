// Compares dotted numeric versions, ignoring pre-release/build suffixes.
export const isVersionGreater = (version: string, than: string): boolean => {
  const parse = (v: string) =>
    v
      .trim()
      .replace(/^v/, '')
      .split(/[-+]/)[0]
      .split('.')
      .map((part) => Number.parseInt(part, 10) || 0);

  const a = parse(version);
  const b = parse(than);
  const len = Math.max(a.length, b.length);
  for (let i = 0; i < len; i++) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff > 0;
  }
  return false;
};
