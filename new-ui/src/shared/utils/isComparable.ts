export const isComparableWithStrictEquality = (val: unknown) => {
  const t = typeof val;
  return (
    t === 'number' ||
    t === 'string' ||
    t === 'boolean' ||
    t === 'undefined' ||
    val === null
  );
};
