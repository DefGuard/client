/** Build the Edge URL that starts an external-OIDC MFA step in the system browser.
 *
 *  `step_attempt_id` stays snake_case because it is Edge's query-parameter name, not an object
 *  property. It is omitted for a legacy session that has no attempt to bind to, which is the shape
 *  a pre-2.2 Edge still accepts. The raw token is passed through untouched: Edge composes the
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
