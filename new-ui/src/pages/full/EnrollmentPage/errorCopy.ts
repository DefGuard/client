export type EnrollmentErrorCopyEntry = { title: string; message: string };

export const EnrollmentErrorCopy = {
  mfa: {
    title: 'Verification Failed',
    message:
      'Something went wrong while verifying your identity with this authentication method. ' +
      'Please try again or choose a different MFA method.',
  },
  generic: {
    title: 'Something went wrong',
    message:
      "We couldn't reach an external service to complete this step. " +
      'This may be a network issue or a temporary service outage. Please try again.',
  },
} as const;
