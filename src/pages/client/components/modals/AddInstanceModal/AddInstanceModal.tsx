import { zodResolver } from '@hookform/resolvers/zod';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useToaster } from '../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { EnrollmentStartResponse } from '../../../../../shared/hooks/api/types';
import { routes } from '../../../../../shared/routes';
import { useEnrollmentStore } from '../../../../enrollment/hooks/store/useEnrollmentStore';
import { useAddInstanceModal } from './hooks/useAddInstanceModal';

export const AddInstanceModal = () => {
  const { LL } = useI18nContext();
  const [isOpen, loading, reset, close] = useAddInstanceModal((state) => [
    state.isOpen,
    state.loading,
    state.reset,
    state.close,
  ]);

  return (
    <ModalWithTitle
      title={LL.pages.client.modals.addInstanceModal.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      disableClose={loading}
      backdrop
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

type FormFields = {
  url: string;
  token: string;
};

const defaultValues: FormFields = {
  url: '',
  token: '',
};

const ModalContent = () => {
  const toaster = useToaster();
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const initEnrollment = useEnrollmentStore((state) => state.init);
  const isLoading = useAddInstanceModal((state) => state.loading);
  const closeModal = useAddInstanceModal((state) => state.close);
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
      if (import.meta.env.DEV) {
        return endpoint;
      }
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
      'Access-Control-Allow-Origin': '*',
    };

    const data = JSON.stringify({
      token: values.token,
    });

    setModalState({ loading: true });

    fetch(endpointUrl, {
      method: 'POST',
      headers,
      body: data,
      mode: 'cors',
    })
      .then((res) => {
        if (!res.ok) {
          toaster.error(LL.pages.client.modals.addInstanceModal.messages.error());
          setModalState({ loading: false });
          return;
        }
        res.json().then((r: EnrollmentStartResponse) => {
          setModalState({ loading: false });
          if (r.user.is_active) {
          } else {
            let proxy_url = import.meta.env.DEV ? '/api/v1' : values.url;
            if (proxy_url[proxy_url.length - 1] === '/') {
              proxy_url = proxy_url.slice(0, -1);
            }
            const sessionEnd = dayjs.unix(r.deadline_timestamp).utc().local().format();
            const sessionStart = dayjs().local().format();
            initEnrollment({
              userInfo: r.user,
              adminInfo: r.admin,
              endContent: r.final_page_content,
              proxy_url: proxy_url,
              sessionEnd,
              sessionStart,
            });
            closeModal();
            navigate(routes.enrollment, { replace: true });
          }
        });
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
