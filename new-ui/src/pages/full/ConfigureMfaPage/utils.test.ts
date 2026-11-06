import { describe, expect, it } from 'vitest';
import { ConfigureFactorsSource, MfaMethod } from '../../../shared/rust-api/types';
import { isMfaFactorOfferable, showsVerificationPicker } from './utils';

describe('showsVerificationPicker', () => {
  it('shows for more than one method', () => {
    expect(showsVerificationPicker([MfaMethod.Totp, MfaMethod.Email])).toBe(true);
  });

  it('shows for OpenID alone', () => {
    expect(showsVerificationPicker([MfaMethod.Oidc])).toBe(true);
  });

  it('skips for a single code method', () => {
    expect(showsVerificationPicker([MfaMethod.Totp])).toBe(false);
  });
});

describe('isMfaFactorOfferable', () => {
  const setupMethods = [MfaMethod.Totp, MfaMethod.Fido2];

  it('offers another FIDO2 key from the Add page', () => {
    expect(
      isMfaFactorOfferable(
        MfaMethod.Fido2,
        [MfaMethod.Fido2],
        setupMethods,
        ConfigureFactorsSource.AddPage,
      ),
    ).toBe(true);
  });

  it.each([
    ConfigureFactorsSource.FullConnect,
    ConfigureFactorsSource.TrayConnect,
    ConfigureFactorsSource.FullMfaEdit,
    ConfigureFactorsSource.TrayMfaEdit,
    null,
  ])('treats a configured FIDO2 as configured from %s', (source) => {
    expect(
      isMfaFactorOfferable(MfaMethod.Fido2, [MfaMethod.Fido2], setupMethods, source),
    ).toBe(false);
  });

  it('offers an unconfigured FIDO2 from a location', () => {
    expect(
      isMfaFactorOfferable(
        MfaMethod.Fido2,
        [],
        setupMethods,
        ConfigureFactorsSource.FullConnect,
      ),
    ).toBe(true);
  });

  it('never offers a configured TOTP', () => {
    expect(
      isMfaFactorOfferable(
        MfaMethod.Totp,
        [MfaMethod.Totp],
        setupMethods,
        ConfigureFactorsSource.AddPage,
      ),
    ).toBe(false);
  });
});
