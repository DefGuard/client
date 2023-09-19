import axios, { AxiosResponse } from 'axios';

import { useEnrollmentStore } from '../../../pages/enrollment/hooks/store/useEnrollmentStore';
import { UseApi } from './types';

const unpackRequest = <T,>(res: AxiosResponse<T>): T => res.data;

export const useApi = (): UseApi => {
  const url = useEnrollmentStore((state) => state.proxy_url);

  const client = axios.create({
    baseURL: url,
  });

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
