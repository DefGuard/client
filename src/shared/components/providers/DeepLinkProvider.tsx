import { invoke } from '@tauri-apps/api/core';
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
import { debug, error } from '@tauri-apps/plugin-log';
import dayjs from 'dayjs';
import { type PropsWithChildren, useCallback, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import z, { string } from 'zod';
import { clientApi } from '../../../pages/client/clientAPI/clientApi';
import { useClientStore } from '../../../pages/client/hooks/useClientStore';
import { AddInstanceFormStep } from '../../../pages/client/pages/ClientAddInstancePage/hooks/types';
import { useAddInstanceStore } from '../../../pages/client/pages/ClientAddInstancePage/hooks/useAddInstanceStore';
import { WireguardInstanceType } from '../../../pages/client/types';
import { useEnrollmentStore } from '../../../pages/enrollment/hooks/store/useEnrollmentStore';
import { useEnrollmentApi } from '../../../pages/enrollment/hooks/useEnrollmentApi';
import type { EnrollmentStartResponse } from '../../hooks/api/types';
import { routes } from '../../routes';

enum DeepLink {
  AddInstance = 'addinstance',
}

const linkStorageKey = 'lastSuccessfullyHandledDeepLink';

const storeLink = (value: string) => {
  sessionStorage.setItem(linkStorageKey, value);
};

const readStoreLink = (): string | null => {
  return sessionStorage.getItem(linkStorageKey);
};

const addInstanceLinkSchema = z.object({
  token: string().trim().min(1),
  url: string().trim().min(1).url(),
});

const AddInstanceLink = z.object({
  link: z.literal(DeepLink.AddInstance),
  data: addInstanceLinkSchema,
});

const validLinkPayload = z.discriminatedUnion('link', [AddInstanceLink]);

type LinkPayload = z.infer<typeof validLinkPayload>;

const linkIntoPayload = (link: URL | null): LinkPayload | null => {
  if (link == null) return null;

  const searchData = Object.fromEntries(new URLSearchParams(link.search));
  const linkKey = [link.hostname, link.pathname]
    .map((l) => l.trim().replaceAll('/', ''))
    .filter((l) => l !== '')[0] as string;
  const payload = {
    link: linkKey,
    data: searchData,
  };
  const result = validLinkPayload.safeParse(payload);
  if (result.success) {
    return result.data;
  } else {
    error(`Link ${link} was rejected due to schema validation.`);
  }
  return null;
};

const prepareProxyUrl = (value: string) => {
  let proxyUrl = value;
  if (proxyUrl[proxyUrl.length - 1] === '/') {
    proxyUrl = proxyUrl.slice(0, -1);
  }
  proxyUrl = `${proxyUrl}/api/v1`;
  return proxyUrl;
};

export const DeepLinkProvider = ({ children }: PropsWithChildren) => {
  const mounted = useRef(false);

  const {
    enrollment: { start, networkInfo },
  } = useEnrollmentApi();

  const setEnrollmentState = useEnrollmentStore((s) => s.init);
  const setAddInstanceState = useAddInstanceStore((s) => s.setState);
  const setClientState = useClientStore((s) => s.setState);

  const navigate = useNavigate();

  // biome-ignore lint/correctness/useExhaustiveDependencies: should init once
  const handleValidLink = useCallback(async (payload: LinkPayload, rawLink?: string) => {
    const { data, link } = payload;
    switch (link) {
      case DeepLink.AddInstance:
        await start({
          token: data.token,
          proxyUrl: prepareProxyUrl(data.url),
        }).then(async (response) => {
          if (response.ok) {
            const authCookie = response.headers
              .getSetCookie()
              .find((cookie) => cookie.startsWith('defguard_proxy='));
            if (authCookie === undefined) {
              error('Failed to open deep link, auth cookie missing from proxy response.');
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
                  type: WireguardInstanceType.DEFGUARD_INSTANCE,
                  id: existingInstance.id,
                },
              });
              if (rawLink) {
                storeLink(rawLink);
              }
              debug(`Updated ${existingInstance.name} via deep link`);
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
              `Add instance from deep link failed! Proxy enrollment start request failed! status: ${response.status}`,
            );
          }
        });
        break;
    }
    if (rawLink) {
      storeLink(rawLink);
    }
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: only on mount
  useEffect(() => {
    if (!mounted.current) {
      mounted.current = true;

      let unlisten: (() => void) | undefined;
      (async () => {
        const start = await getCurrent();
        if (start != null) {
          const lastLink = readStoreLink();
          // if the link is exact as last successfully executed link
          // this is only necessary bcs in dev mode window is hot reloaded causing the startup link to be handled multiple times over.
          if (lastLink != null && lastLink === start[0]) {
            return;
          }
          const payload = linkIntoPayload(new URL(start[0]));
          if (payload != null) {
            try {
              handleValidLink(payload, start[0]);
            } catch (e) {
              error(
                `Failed to handle valid deep link ${payload.link}!\n${JSON.stringify(e)}`,
              );
            }
          }
        }
        unlisten = await onOpenUrl((urls) => {
          if (urls?.length) {
            const link = urls[0];
            const payload = linkIntoPayload(new URL(link));
            if (payload != null) {
              try {
                handleValidLink(payload);
              } catch (e) {
                error(`Failed to handle valid deep link ${payload?.link} action!`);
                error(JSON.stringify(e));
              }
            }
          }
        });
      })();
      return () => {
        unlisten?.();
      };
    }
  }, []);

  return <>{children}</>;
};
