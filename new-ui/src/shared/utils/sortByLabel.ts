const collator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: 'base',
});

export const sortByLabel = <T>(items: readonly T[], selector: (item: T) => string): T[] =>
  [...items].sort((a, b) => collator.compare(selector(a), selector(b)));
