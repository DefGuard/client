import { Response } from '@tauri-apps/api/http';

import { DefguardInstance } from '../../../pages/client/types';

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
};

export type EnrollmentInstanceInfo = {
  id: string;
  name: string;
  url: string;
};

// FIXME: strong types
export type UseApi = {
  enrollment: {
    start: (data: EnrollmentStartRequest) => Promise<Response<EnrollmentStartResponse>>;
    activateUser: (data: ActivateUserRequest) => Promise<Response<EmptyApiResponse>>;
    createDevice: (data: CreateDeviceRequest) => Promise<Response<CreateDeviceResponse>>;
  };
  getAppInfo: () => Promise<Response<AppInfo>>;
};
