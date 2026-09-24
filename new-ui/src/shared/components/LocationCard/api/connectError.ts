import z from 'zod';

export const DEFAULT_CONNECTION_ERROR =
  'One or more external services are unavailable or unreachable. This may be caused by a network issue or a temporary service outage. Please try again later.';

const connectErrorSchema = z.object({
  kind: z.enum([
    'postureCheckFailed',
    'serviceUnavailable',
    'allTrafficConflict',
    'other',
  ]),
  message: z.string(),
});

export type ConnectError = z.infer<typeof connectErrorSchema>;

export const parseConnectError = (err: unknown): ConnectError | null => {
  const result = connectErrorSchema.safeParse(err);

  return result.success ? result.data : null;
};
