import { error, info } from '@tauri-apps/plugin-log';
import { api } from '../../../../shared/rust-api/api';
import { refreshEnrollmentInstance } from '../../../../shared/rust-api/enrollment';
import { useEnrollmentStore } from './useEnrollmentStore';

/**
 * Activate the enrolling user by calling the enrollment Tauri command, then refresh the saved
 * instance from Core so MFA set up during enrollment is recognised without config polling, which
 * Core refuses without a business license. A failed refresh is only logged.
 *
 * Reads the session from the store via {@linkcode useEnrollmentStore.getState} so every
 * invocation uses the latest values (no stale-closure risk after in-flight
 * {@linkcode setState} updates).
 *
 * When `skipPassword` is true the `password` key is omitted from the request
 * body instead of sending `null` or an empty string.
 */
export const activateUser = async () => {
  const { sessionId, skipPassword, userPassword, startResponse } =
    useEnrollmentStore.getState();
  if (!sessionId) throw new Error('Enrollment session is missing');
  const password = skipPassword ? null : (userPassword ?? '');
  await api.enrollmentActivateUser(sessionId, password, null);
  void info('Enrollment activation succeeded; starting instance refresh');

  try {
    if (!startResponse) throw new Error('Enrollment start response is missing');
    await refreshEnrollmentInstance(sessionId, startResponse.instance.id);
  } catch (err) {
    void error(`Failed to refresh instance after enrollment activation: ${String(err)}`);
  }
};
