/** Builds the browser URL for an external OIDC MFA step. Legacy sessions omit `step_attempt_id`; Edge combines the token and attempt ID. */
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
