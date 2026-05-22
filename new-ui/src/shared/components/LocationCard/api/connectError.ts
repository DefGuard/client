import z from 'zod';

const connectErrorSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('postureCheckFailed'),
    message: z.string(),
  }),
  z.object({
    kind: z.literal('other'),
    message: z.string(),
  }),
]);

export type ConnectError = z.infer<typeof connectErrorSchema>;

export const parseConnectError = (err: unknown): ConnectError | null => {
  const result = connectErrorSchema.safeParse(err);

  return result.success ? result.data : null;
};
