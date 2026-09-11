export const EnrollmentStep = {
  Welcome: 'welcome',
  Password: 'password',
  MfaChoice: 'mfa-choice',
  MfaConfiguration: 'mfa-configuration',
  RecoveryCodes: 'recovery-codes',
  Finish: 'finish',
} as const;

export type EnrollmentStepValue = (typeof EnrollmentStep)[keyof typeof EnrollmentStep];
