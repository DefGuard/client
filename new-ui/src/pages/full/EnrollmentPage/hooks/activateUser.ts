import { edgeApi } from '../../../../shared/edge-api/api';
import { useEnrollmentStore } from './useEnrollmentStore';

/**
 * Activate the enrolling user by calling the proxy activation API.
 *
 * Reads proxy URL, cookie, skipPassword, and userPassword from the store via
 * {@linkcode useEnrollmentStore.getState} so every invocation uses the latest
 * values (no stale-closure risk after in-flight {@linkcode setState} updates).
 *
 * When `skipPassword` is true the `password` key is omitted from the request
 * body instead of sending `null` or an empty string.
 */
export const activateUser = async () => {
  const { proxyUrl, sessionCookie, skipPassword, userPassword } =
    useEnrollmentStore.getState();
  const body = skipPassword ? {} : { password: userPassword ?? '' };
  // biome-ignore lint/style/noNonNullAssertion: proxy and cookie are set in start()
  return edgeApi.activateUser(proxyUrl!, sessionCookie!, body);
};
