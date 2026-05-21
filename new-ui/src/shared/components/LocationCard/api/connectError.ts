export type ConnectError =
  | {
      kind: 'postureCheckFailed';
      message: string;
    }
  | {
      kind: 'other';
      message: string;
    };

export const isPostureCheckFailedConnectError = (
  err: unknown,
): err is Extract<ConnectError, { kind: 'postureCheckFailed' }> =>
  typeof err === 'object' &&
  err !== null &&
  'kind' in err &&
  'message' in err &&
  err.kind === 'postureCheckFailed' &&
  typeof err.message === 'string';
