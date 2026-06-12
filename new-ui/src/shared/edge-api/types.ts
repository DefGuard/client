import type { MfaMethodValue } from '../rust-api/types';

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

export type AddInstanceRequest = { url: string; token: string; name: string };
export type AddInstanceResult = {
  startResponse?: EnrollmentStartResponse;
  proxyUrl?: string;
  cookie?: string;
  error?: string;
};

export type MfaSetupStartRequest = { method: MfaMethodValue };
export type MfaSetupStartResponse = { totp_secret?: string };

export type MfaSetupFinishRequest = { code: string; method: MfaMethodValue };
export type MfaSetupFinishResponse = { recovery_codes: string[] };

export type ActivateUserRequest = {
  password: string;
  phone_number: string;
};

export type ActivateUserResponse = {
  token: string;
};
