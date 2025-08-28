import type { DefguardInstance } from '../../../pages/client/types';
import type { MfaMethod } from '../../types';

export type EmptyApiResponse = Record<string, never>;

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

export type EnrollmentStartRequest = {
  token: string;
};

export type EnrollmentStartResponse = {
  admin: AdminInfo;
  user: UserInfo;
  deadline_timestamp: number;
  final_page_content: string;
  vpn_setup_optional: boolean;
  instance: EnrollmentInstanceInfo;
};

export type ActivateUserRequest = {
  phone_number: string;
  password: string;
};

export type ActivateUserResponse = {
  token: string;
};

export type CreateDeviceRequest = {
  name: string;
  pubkey: string;
};

export type Device = {
  id: number;
  name: string;
  pubkey: string;
  // stored by frontend only
  privateKey?: string;
  user_id: number;
  // timestamp
  created_at: number;
};

export type DeviceConfig = {
  network_id: number;
  network_name: string;
  config: string;
};

export type CreateDeviceResponse = {
  device: Device;
  configs: DeviceConfig[];
  instance: DefguardInstance;
};

export type AppInfo = {
  version: string;
};

export type EnrollmentAdminInfo = {
  name: string;
  email: string;
  phone_number?: string;
};

export type EnrollmentInitialUserInfo = {
  first_name: string;
  last_name: string;
  login: string;
  email: string;
  phone_number?: string;
  is_active: boolean;
  enrolled: boolean;
};

export type EnrollmentInstanceInfo = {
  id: string;
  name: string;
  url: string;
};

export type NewApplicationVersionInfo = {
  version: string;
  release_date: string;
  release_notes_url: string;
  update_url: string;
};

export type RegisterCodeMfaFinishRequest = {
  code: string;
  method: MfaMethod;
};

export type RegisterCodeMfaStartResponse = {
  totp_secret?: string;
};

export type RegisterCodeMfaFinishResponse = {
  recovery_codes: string[];
};

// FIXME: strong types
export type UseApi = {
  enrollment: {
    start: (data: EnrollmentStartRequest) => Promise<Response>;
    activateUser: (data: ActivateUserRequest) => Promise<Response>;
    createDevice: (data: CreateDeviceRequest) => Promise<Response>;
    registerCodeMfaStart: (method: MfaMethod) => Promise<RegisterCodeMfaStartResponse>;
    registerCodeMfaFinish: (
      data: RegisterCodeMfaFinishRequest,
    ) => Promise<RegisterCodeMfaFinishResponse>;
  };
  getAppInfo: () => Promise<Response>;
};

export type EnrollmentError = {
  error: string;
};
