import { Body, fetch } from '@tauri-apps/api/http';

import { useEnrollmentStore } from '../../../pages/enrollment/hooks/store/useEnrollmentStore';
import {
  ActivateUserResponse,
  AppInfo,
  CreateDeviceResponse,
  EnrollmentStartResponse,
  UseApi,
} from '../../../shared/hooks/api/types';

export const useEnrollmentApi = (): UseApi => {
  const [proxyUrl, cookie] = useEnrollmentStore((state) => [
    state.proxy_url,
    state.cookie,
  ]);

  const startEnrollment: UseApi['enrollment']['start'] = async (data) => {
    const response = await fetch<EnrollmentStartResponse>(
      `${proxyUrl}/enrollment/start`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Cookie: cookie,
        },
        body: Body.json(data),
      },
    );
    return response;
  };

  const activateUser: UseApi['enrollment']['activateUser'] = async (data) => {
    const response = await fetch<ActivateUserResponse>(
      `${proxyUrl}/enrollment/activate_user`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Cookie: cookie,
        },
        body: Body.json(data),
      },
    );

    return response;
  };

  const createDevice: UseApi['enrollment']['createDevice'] = async (data) => {
    const response = await fetch<CreateDeviceResponse>(
      `${proxyUrl}/enrollment/create_device`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Cookie: cookie,
        },
        body: Body.json(data),
      },
    );

    return response;
  };

  const getAppInfo: UseApi['getAppInfo'] = async () => {
    const response = await fetch<AppInfo>(`${proxyUrl}/info`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      },
    });

    return response;
  };

  return {
    enrollment: {
      start: startEnrollment,
      activateUser,
      createDevice,
    },
    getAppInfo,
  };
};
