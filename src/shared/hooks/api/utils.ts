// biome-ignore lint/suspicious/noExplicitAny: intended
export const removeNulls = (obj: any) => {
  return JSON.parse(JSON.stringify(obj), (_, value) => {
    if (value == null) return undefined;
    return value;
  });
};
