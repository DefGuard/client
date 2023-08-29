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
  phone_number?: string;
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
};

export type AppInfo = {
  version: string;
};

export type UseApi = {
  enrollment: {
    start: (data: EnrollmentStartRequest) => Promise<EnrollmentStartResponse>;
    activateUser: (data: ActivateUserRequest) => Promise<EmptyApiResponse>;
    createDevice: (data: CreateDeviceRequest) => Promise<CreateDeviceResponse>;
  };
  getAppInfo: () => Promise<AppInfo>;
};
