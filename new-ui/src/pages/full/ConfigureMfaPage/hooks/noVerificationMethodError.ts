/** The session opened, but nothing it offers to verify with can run on this instance. */
export class NoVerificationMethodError extends Error {
  constructor() {
    super('No MFA method available to verify the configuration session');
    this.name = 'NoVerificationMethodError';
  }
}
