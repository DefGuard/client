import { fetch } from '@tauri-apps/plugin-http';

import { useEnrollmentStore } from '../../../pages/enrollment/hooks/store/useEnrollmentStore';
import type { UseApi } from '../../../shared/hooks/api/types';
import { MfaMethod } from '../../../shared/types';

export const useEnrollmentApi = (): UseApi => {
  const [proxyUrl, cookie] = useEnrollmentStore((state) => [
    state.proxy_url,
    state.cookie,
  ]);

  const registerCodeMfaStart: UseApi['enrollment']['registerCodeMfaStart'] = async (
    method,
  ) => {
    const response = await fetch(`${proxyUrl}/enrollment/register-mfa/code/start`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      } as Record<string, string>,
      body: JSON.stringify({
        method: method.valueOf(),
      }),
    });
    return await response.json();
  };

  const registerCodeMfaFinish: UseApi['enrollment']['registerCodeMfaFinish'] = async (
    data,
  ) => {
    const response = await fetch(`${proxyUrl}/enrollment/register-mfa/code/finish`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      } as Record<string, string>,
      body: JSON.stringify({ ...data, method: MfaMethod.TOTP.valueOf() }),
    });
    return await response.json();
  };

  const start: UseApi['enrollment']['start'] = async (data) => {
    const response = await fetch(`${proxyUrl}/enrollment/start`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      } as Record<string, string>,
      body: JSON.stringify(data),
    });
    return response;
  };

  const activateUser: UseApi['enrollment']['activateUser'] = async (data) => {
    const response = await fetch(`${proxyUrl}/enrollment/activate_user`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      } as Record<string, string>,
      body: JSON.stringify(data),
    });

    return response;
  };

  const createDevice: UseApi['enrollment']['createDevice'] = async (data) => {
    const response = await fetch(`${proxyUrl}/enrollment/create_device`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      } as Record<string, string>,
      body: JSON.stringify(data),
    });

    return response;
  };

  const getAppInfo: UseApi['getAppInfo'] = async () => {
    const response = await fetch(`${proxyUrl}/info`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        Cookie: cookie,
      } as Record<string, string>,
    });

    return response;
  };

  return {
    enrollment: {
      start,
      activateUser,
      createDevice,
      registerCodeMfaStart,
      registerCodeMfaFinish,
    },
    getAppInfo,
  };
};
