import type { AddInstanceResult } from '../../../../shared/rust-api/types';

type AddInstanceFailure =
  | { kind: 'field'; field: 'url' | 'token' | 'name'; message: string }
  | { kind: 'snackbar'; message: string };

/** Deep links hide the URL and token fields, so their errors go to a snackbar instead. */
export const getAddInstanceFailure = (
  result: AddInstanceResult,
  credentialsHidden: boolean,
): AddInstanceFailure => {
  switch (result.errorKind) {
    case 'network':
      return credentialsHidden
        ? {
            kind: 'snackbar',
            message:
              'Unable to reach the instance. Check your connection or contact administrator.',
          }
        : { kind: 'field', field: 'url', message: 'Invalid URL.' };
    case 'unauthorized':
      return credentialsHidden
        ? {
            kind: 'snackbar',
            message:
              'Enrollment token is invalid or expired. Request a new link from your administrator.',
          }
        : { kind: 'field', field: 'token', message: 'Invalid token.' };
    case 'name_taken':
      return { kind: 'field', field: 'name', message: 'Name already used.' };
    default:
      return { kind: 'snackbar', message: 'Communication error, contact administrator.' };
  }
};
