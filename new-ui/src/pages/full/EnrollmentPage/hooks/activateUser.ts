import { error } from '@tauri-apps/plugin-log';
import { api } from '../../../../shared/rust-api/api';
import { refreshEnrollmentInstance } from '../../../../shared/rust-api/enrollment';
import { useEnrollmentStore } from './useEnrollmentStore';

/**
 * Activate the enrolling user by calling the enrollment Tauri command.
 *
 * Reads sessionId, skipPassword, and userPassword from the store via
 * {@linkcode useEnrollmentStore.getState} so every invocation uses the latest
 * values (no stale-closure risk after in-flight {@linkcode setState} updates).
 *
 * When `skipPassword` is true the `password` key is omitted from the request
 * body instead of sending `null` or an empty string.
 */
export const activateUser = async () => {
  const { sessionId, skipPassword, userPassword, startResponse } =
    useEnrollmentStore.getState();
  const password = skipPassword ? null : (userPassword ?? '');
  // biome-ignore lint/style/noNonNullAssertion: sessionId is set in start(), called before this
  await api.enrollmentActivateUser(sessionId!, password, null);

  try {
    if (!startResponse) throw new Error('Enrollment start response is missing');
    // biome-ignore lint/style/noNonNullAssertion: sessionId is set in start(), called before this
    await refreshEnrollmentInstance(sessionId!, startResponse.instance.id);
  } catch (err) {
    void error(`Failed to refresh instance after enrollment activation: ${String(err)}`);
  }
};
