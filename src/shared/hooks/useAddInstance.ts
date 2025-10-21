/**
 * Hook which handles adding an instance in the background and triggering enrollment process (if necessary)
 * in automated scenarios e.g. deep-link, client provisioning etc.
 */

import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '@tauri-apps/plugin-log';
import dayjs from 'dayjs';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { clientApi } from '../../pages/client/clientAPI/clientApi';
import { useClientStore } from '../../pages/client/hooks/useClientStore';
import { AddInstanceFormStep } from '../../pages/client/pages/ClientAddInstancePage/hooks/types';
import { useAddInstanceStore } from '../../pages/client/pages/ClientAddInstancePage/hooks/useAddInstanceStore';
import { type AddInstancePayload, ClientConnectionType } from '../../pages/client/types';
import { useEnrollmentStore } from '../../pages/enrollment/hooks/store/useEnrollmentStore';
import { useEnrollmentApi } from '../../pages/enrollment/hooks/useEnrollmentApi';
import { storeLink } from '../components/providers/DeepLinkProvider';
import type { EnrollmentStartResponse } from '../hooks/api/types';
import { routes } from '../routes';

const prepareProxyUrl = (value: string) => {
  let proxyUrl = value;
  if (proxyUrl[proxyUrl.length - 1] === '/') {
    proxyUrl = proxyUrl.slice(0, -1);
  }
  proxyUrl = `${proxyUrl}/api/v1`;
  return proxyUrl;
};

export default function useAddInstance() {
  const [loading, setLoading] = useState(false);

  const setEnrollmentState = useEnrollmentStore((s) => s.init);
  const setAddInstanceState = useAddInstanceStore((s) => s.setState);
  const setClientState = useClientStore((s) => s.setState);

  const navigate = useNavigate();

  const {
    enrollment: { start, networkInfo },
  } = useEnrollmentApi();

  const handleAddInstance = async (payload: AddInstancePayload, rawLink?: string) => {
    setLoading(true);

    await start({
      token: payload.token,
      proxyUrl: prepareProxyUrl(payload.url),
    }).then(async (response) => {
      if (response.ok) {
        const authCookie = response.headers
          .getSetCookie()
          .find((cookie) => cookie.startsWith('defguard_proxy='));
        if (authCookie === undefined) {
          error(
            'Failed to automatically add new instance, auth cookie missing from proxy response.',
          );
          return;
        }
        const respData = (await response.json()) as EnrollmentStartResponse;
        const instances = await clientApi.getInstances();
        const proxy_api_url = prepareProxyUrl(
          respData.instance.proxy_url ?? respData.instance.url,
        );
        const existingInstance = instances.find(
          (instance) => instance.uuid === respData.instance.id,
        );
        if (existingInstance) {
          // update existing instance instead
          const networkInfoResp = await networkInfo(
            {
              pubkey: existingInstance.pubkey,
            },
            proxy_api_url,
            authCookie,
          );
          await invoke<void>('update_instance', {
            instanceId: existingInstance.id,
            response: networkInfoResp,
          });
          setClientState({
            selectedInstance: {
              type: ClientConnectionType.LOCATION,
              id: existingInstance.id,
            },
          });
          if (rawLink) {
            storeLink(rawLink);
          }
          debug(`Automatically updated ${existingInstance.name}`);
          navigate(routes.client.base, { replace: true });
          return;
        }
        if (!respData.user.enrolled) {
          // user needs full enrollment
          const sessionEnd = dayjs
            .unix(respData.deadline_timestamp)
            .utc()
            .local()
            .format();
          const sessionStart = dayjs().local().format();
          // set enrollment
          setEnrollmentState({
            enrollmentSettings: respData.settings,
            proxy_url: proxy_api_url,
            userInfo: respData.user,
            adminInfo: respData.admin,
            endContent: respData.final_page_content,
            cookie: authCookie,
            sessionEnd,
            sessionStart,
          });
          navigate('/enrollment', { replace: true });
        } else {
          // only needs to register this device
          setAddInstanceState({
            step: AddInstanceFormStep.DEVICE,
            response: {
              cookie: authCookie,
              device_names: respData.user.device_names,
              url: proxy_api_url,
            },
          });
          navigate('/client/add-instance', { replace: true });
        }
      } else {
        error(
          `Adding instance automatically failed. Proxy enrollment start request failed with status: ${response.status}`,
        );
      }
    });
  };

  return { handleAddInstance, loading, error };
}
