/** Build the Edge URL that starts an external-OIDC MFA step in the system browser.
 *
 *  `step_attempt_id` is snake_case because it is Edge's query-parameter name, not an object
 *  property, and is omitted for a legacy session. Pass the raw token: Edge composes the
 *  composite AuthInfo state, so a composite value must never be built here. */
export const buildOpenIdMfaUrl = (
  proxyUrl: string,
  token: string,
  stepAttemptId: string | null,
): URL => {
  const url = new URL('openid/mfa', proxyUrl.endsWith('/') ? proxyUrl : `${proxyUrl}/`);
  url.searchParams.set('token', token);
  if (stepAttemptId) {
    url.searchParams.set('step_attempt_id', stepAttemptId);
  }
  return url;
};
