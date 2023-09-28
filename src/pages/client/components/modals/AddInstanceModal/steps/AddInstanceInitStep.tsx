import { zodResolver } from '@hookform/resolvers/zod';
import { Body, fetch, Response } from '@tauri-apps/api/http';
import dayjs from 'dayjs';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { EnrollmentStartResponse } from '../../../../../../shared/hooks/api/types';
import { routes } from '../../../../../../shared/routes';
import { useEnrollmentStore } from '../../../../../enrollment/hooks/store/useEnrollmentStore';
import { clientApi } from '../../../../clientAPI/clientApi';
import { useAddInstanceModal } from '../hooks/useAddInstanceModal';

type FormFields = {
  url: string;
  token: string;
};

const defaultValues: FormFields = {
  url: '',
  token: '',
};

export const AddInstanceModalInitStep = () => {
  const toaster = useToaster();
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const initEnrollment = useEnrollmentStore((state) => state.init);
  const isLoading = useAddInstanceModal((state) => state.loading);
  const closeModal = useAddInstanceModal((state) => state.close);
  const nextStep = useAddInstanceModal((state) => state.next);
  const setModalState = useAddInstanceModal((state) => state.setState);
  const schema = useMemo(
    () =>
      z.object({
        url: z
          .string()
          .trim()
          .nonempty(LL.form.errors.required())
          .url(LL.form.errors.invalid()),
        token: z.string().trim().nonempty(LL.form.errors.required()),
      }),
    [LL.form.errors],
  );
  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    const url = () => {
      const endpoint = '/api/v1/enrollment/start';
      let base: string;
      if (values.url[values.url.length - 1] === '/') {
        base = values.url.slice(0, -1);
      } else {
        base = values.url;
      }
      return base + endpoint;
    };

    const endpointUrl = url();

    const headers = {
      'Content-Type': 'application/json',
    };

    const data = {
      token: values.token,
    };

    setModalState({ loading: true });
    console.log(endpointUrl);

    fetch(endpointUrl, {
      method: 'POST',
      headers,
      body: Body.json(data),
    })
      .then(async (res: Response<EnrollmentStartResponse>) => {
        const authCookie = res.headers['set-cookie'];
        if (!res.ok) {
          toaster.error(LL.pages.client.modals.addInstanceModal.messages.error());
          setModalState({ loading: false });
          return;
        }
        const r = res.data;
        // get client registered instances
        const clientInstances = await clientApi.getInstances();
        const instance = clientInstances.find((i) => i.uuid === r.instance.id);
        let proxy_api_url = values.url;
        if (proxy_api_url[proxy_api_url.length - 1] === '/') {
          proxy_api_url = proxy_api_url.slice(0, -1);
        }
        console.log(proxy_api_url);
        proxy_api_url = proxy_api_url + '/api/v1';
        setModalState({ loading: false });

        if (instance) {
          // update already registered instance instead
          headers['Cookie'] = authCookie;
          const instanceInfo = await fetch(`${proxy_api_url}/enrollment/network_info`, {
            method: 'POST',
            headers,
            body: Body.json({
              pubkey: instance.pubkey,
            }),
          });
          return;
        }
        // register new instance
        // is user in need of full enrollment ?
        if (r.user.is_active) {
          //no, only create new device for desktop client
          nextStep({
            proxyUrl: proxy_api_url,
            cookie: authCookie,
          });
        } else {
          // yes, enroll the user
          const sessionEnd = dayjs.unix(r.deadline_timestamp).utc().local().format();
          const sessionStart = dayjs().local().format();
          initEnrollment({
            userInfo: r.user,
            adminInfo: r.admin,
            endContent: r.final_page_content,
            proxy_url: proxy_api_url,
            sessionEnd,
            sessionStart,
            cookie: authCookie,
          });
          closeModal();
          navigate(routes.enrollment, { replace: true });
        }
      })
      .catch((e) => {
        toaster.error(LL.pages.client.modals.addInstanceModal.messages.error());
        setModalState({ loading: false });
        console.error(e);
      });
  };

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput
        controller={{ control, name: 'url' }}
        label={LL.pages.client.modals.addInstanceModal.form.fields.url.label()}
      />
      <FormInput
        controller={{ control, name: 'token' }}
        label={LL.pages.client.modals.addInstanceModal.form.fields.token.label()}
      />
      <div className="controls">
        <Button
          className="close"
          onClick={closeModal}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.cancel()}
        />
        <Button
          type="submit"
          className="submit"
          loading={isLoading}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.submit()}
        />
      </div>
    </form>
  );
};
