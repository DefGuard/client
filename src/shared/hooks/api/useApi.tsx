import { Body, fetch } from '@tauri-apps/api/http';

import { useEnrollmentStore } from '../../../pages/enrollment/hooks/store/useEnrollmentStore';
import { UseApi } from './types';

export const useApi = (): UseApi => {
  const [proxyUrl, cookie] = useEnrollmentStore((state) => [
    state.proxy_url,
    state.cookie,
  ]);

  const startEnrollment: UseApi['enrollment']['start'] = async (data) => {
    const response = await fetch(`${proxyUrl}/enrollment/start`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      },
      body: Body.json(data),
    });
    return response;
  };

  const activateUser: UseApi['enrollment']['activateUser'] = async (data) => {
    const response = await fetch(`${proxyUrl}/enrollment/activate_user`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      },
      body: Body.json(data),
    });

    return response;
  };

  const createDevice: UseApi['enrollment']['createDevice'] = async (data) => {
    const response = await fetch(`${proxyUrl}/enrollment/create_device`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      },
      body: Body.json(data),
    });

    return response;
  };

  const getAppInfo: UseApi['getAppInfo'] = async () => {
    const response = await fetch(`${proxyUrl}/info`, {
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
