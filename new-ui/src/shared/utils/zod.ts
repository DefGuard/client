import type z from 'zod';

export const createZodIssue = (
  message: string,
  path: PropertyKey[],
): z.core.$ZodIssueCustom => ({
  code: 'custom',
  message,
  path,
});
