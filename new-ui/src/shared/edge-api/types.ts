import type { EnrollmentStartResult, MfaMethodValue } from '../rust-api/types';

export type EnrollmentInstanceInfo = {
  id: string;
  name: string;
  url: string;
  proxy_url?: string;
  username: string;
  openid_display_name?: string;
};

export type AdminInfo = {
  name: string;
  email: string;
  phone_number?: string;
};

export type UserInfo = {
  first_name: string;
  last_name: string;
  login: string;
  email: string;
  is_active: boolean;
  phone_number: string;
  device_names: string[];
  enrolled: boolean;
  password_management_disabled: boolean;
};

export type EnrollmentSettings = {
  admin_device_management: boolean;
  mfa_required: boolean;
  only_client_activation: boolean;
  smtp_configured: boolean;
  vpn_setup_optional: boolean;
};

export type EnrollmentStartResponse = {
  admin: AdminInfo;
  user: UserInfo;
  instance: EnrollmentInstanceInfo;
  deadline_timestamp: number;
  final_page_content: string;
  vpn_setup_optional: boolean;
  settings: EnrollmentSettings;
};

export type EdgeRequestHeaders = {
  'defguard-client-version': string;
  'defguard-client-platform': string;
};

/** `network`: the request could not be sent, most likely a bad URL.
 *  `unauthorized`: the server responded 401, the token is invalid.
 *  `server`: any other failure response. */
export type EnrollmentErrorKind = 'network' | 'unauthorized' | 'server';

export type AddInstanceRequest = { url: string; token: string; name: string };
export type AddInstanceResult = {
  startResponse?: EnrollmentStartResult;
  session_id?: string;
  error?: string;
  errorKind?: EnrollmentErrorKind;
};

export type UpdateInstanceRequest = { instanceId: number; url: string; token: string };
export type UpdateInstanceResult = {
  error?: string;
  errorKind?: EnrollmentErrorKind;
};

export type MfaSetupStartRequest = { method: MfaMethodValue };
export type MfaSetupStartResponse = { totp_secret?: string };

export type MfaSetupFinishRequest = { code: string; method: MfaMethodValue };
export type MfaSetupFinishResponse = { recovery_codes: string[] };

export type ActivateUserRequest = {
  password?: string;
  phone_number: string;
};

export type ActivateUserResponse = {
  token: string;
};
