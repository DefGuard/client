export const formatRequestBody = <T>(value: T): T => {
  if (typeof value === 'string') return value.trim() as T;
  if (Array.isArray(value)) return value.map(formatRequestBody) as T;
  if (value !== null && typeof value === 'object' && !(value instanceof Date)) {
    return Object.fromEntries(
      Object.entries(value).map(([k, v]) => [k, formatRequestBody(v)]),
    ) as T;
  }
  return value;
};
