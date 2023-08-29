import axios, { AxiosResponse } from 'axios';

import { UseApi } from './types';

const envBaseUrl = import.meta.env.VITE_API_BASE_URL;

const unpackRequest = <T,>(res: AxiosResponse<T>): T => res.data;

const client = axios.create({
  baseURL: envBaseUrl && String(envBaseUrl).length > 0 ? envBaseUrl : '/api/v1',
});

client.defaults.headers.common['Content-Type'] = 'application/json';

export const useApi = (): UseApi => {
  const startEnrollment: UseApi['enrollment']['start'] = (data) =>
    client.post('/enrollment/start', data).then(unpackRequest);

  const activateUser: UseApi['enrollment']['activateUser'] = (data) =>
    client.post('/enrollment/activate_user', data).then(unpackRequest);

  const createDevice: UseApi['enrollment']['createDevice'] = (data) =>
    client.post('/enrollment/create_device', data).then(unpackRequest);

  const getAppInfo: UseApi['getAppInfo'] = () => client.get('/info').then(unpackRequest);

  return {
    enrollment: {
      start: startEnrollment,
      activateUser,
      createDevice,
    },
    getAppInfo,
  };
};
