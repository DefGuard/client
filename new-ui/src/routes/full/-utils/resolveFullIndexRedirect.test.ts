import { describe, expect, it } from 'vitest';
import { resolveFullIndexRedirect } from './resolveFullIndexRedirect';

describe('resolveFullIndexRedirect', () => {
  it('adds an empty client when no provisioning config exists', () => {
    expect(
      resolveFullIndexRedirect({
        instanceCount: 0,
        tunnelCount: 0,
        provisioningConfig: null,
      }),
    ).toEqual({ to: '/full/add' });
  });

  it('opens the prefilled add-instance form for an empty client with config', () => {
    expect(
      resolveFullIndexRedirect({
        instanceCount: 0,
        tunnelCount: 0,
        provisioningConfig: {
          enrollment_url: 'https://enroll.example',
          enrollment_token: 'provisioning-token',
        },
      }),
    ).toEqual({
      to: '/full/add/instance',
      search: {
        url: 'https://enroll.example',
        token: 'provisioning-token',
      },
    });
  });

  it('opens the prefilled form even if tunnels exist', () => {
    expect(
      resolveFullIndexRedirect({
        instanceCount: 0,
        tunnelCount: 1,
        provisioningConfig: {
          enrollment_url: 'https://enroll.example',
          enrollment_token: 'provisioning-token',
        },
      }),
    ).toEqual({
      to: '/full/add/instance',
      search: {
        url: 'https://enroll.example',
        token: 'provisioning-token',
      },
    });
  });

  it('shows overview when an instance exists despite config', () => {
    expect(
      resolveFullIndexRedirect({
        instanceCount: 1,
        tunnelCount: 0,
        provisioningConfig: {
          enrollment_url: 'https://enroll.example',
          enrollment_token: 'provisioning-token',
        },
      }),
    ).toEqual({ to: '/full/overview' });
  });

  it('shows overview for an empty client with tunnels and no config', () => {
    expect(
      resolveFullIndexRedirect({
        instanceCount: 0,
        tunnelCount: 1,
        provisioningConfig: null,
      }),
    ).toEqual({ to: '/full/overview' });
  });
});
