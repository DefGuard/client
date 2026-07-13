import './style.scss';

import { useLoaderData, useNavigate, useSearch } from '@tanstack/react-router';
import { error } from '@tauri-apps/plugin-log';
import { Fragment, useMemo } from 'react';
import z from 'zod';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { edgeApi } from '../../../shared/edge-api/api';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { Snackbar } from '../../../shared/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { useEnrollmentStore } from '../EnrollmentPage/hooks/useEnrollmentStore';

const formSchema = z.object({
  token: z.string().min(1, 'Required'),
  url: z.string().min(1, 'Required'),
  name: z.string().min(1, 'Required'),
});

type FormFields = z.infer<typeof formSchema>;

export const AddInstancePage = () => {
  const navigate = useNavigate();
  const { deviceName } = useLoaderData({ from: '/full/_default/add/instance' });
  const searchValues = useSearch({ from: '/full/_default/add/instance' });

  const hasInitialValues = isPresent(searchValues.token) && isPresent(searchValues.url);

  const defaultValues = useMemo((): FormFields => {
    return {
      token: searchValues.token ?? '',
      url: searchValues.url ?? '',
      name: deviceName,
    };
  }, [searchValues, deviceName]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      const result = await edgeApi.addInstance(value);
      if (result.error ?? result.errorKind) {
        if (result.errorKind === 'network') {
          formApi.setErrorMap({
            onSubmit: { fields: { url: 'Invalid URL.' } },
          });
          return;
        }
        if (result.errorKind === 'unauthorized') {
          formApi.setErrorMap({
            onSubmit: { fields: { token: 'Invalid token.' } },
          });
          return;
        }
        if (result.error?.toLowerCase().includes('device name')) {
          formApi.setErrorMap({
            onSubmit: { fields: { name: 'Name already used.' } },
          });
          return;
        }
        void error(`Failed to add instance: ${result.error ?? 'unknown error'}`);
        Snackbar.error('Communication error, contact administrator.');
        return;
      }
      if (result.session_id) {
        await edgeApi.createDevice(result.session_id, value.name.trim());
      }
      if (
        result.startResponse &&
        !result.startResponse.user.enrolled &&
        result.session_id
      ) {
        useEnrollmentStore
          .getState()
          .start(result.startResponse, result.session_id, undefined);
        navigate({
          to: '/full/enrollment',
          replace: true,
        });
      } else {
        navigate({
          to: '/full/overview',
          replace: true,
        });
      }
    },
  });

  return (
    <FullPage id="add-instance-view">
      <FullPageTitle title="Add instance" />
      <p className="page-description">{`To add an instance, provide the instance URL along with a valid provisioning token. These credentials are issued by your administrator and are required to initiate the setup.`}</p>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          {!hasInitialValues && (
            <Fragment>
              <form.AppField name="url">
                {(field) => <field.FormInput label="URL" required />}
              </form.AppField>
              <SizedBox height={ThemeSpacing.Xl} />
              <form.AppField name="token">
                {(field) => <field.FormInput label="Token" required />}
              </form.AppField>
              <SizedBox height={ThemeSpacing.Xl} />
            </Fragment>
          )}
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Device name" />}
          </form.AppField>
          <Controls>
            <Button
              text="Back"
              variant={ButtonVariant.Secondary}
              onClick={() => {
                navigate({ to: '/full/add' });
              }}
            />
            <div className="right">
              <form.Subscribe selector={(s) => s.isSubmitting}>
                {(isSubmitting) => (
                  <Button
                    text="Add Instance"
                    loading={isSubmitting}
                    variant={ButtonVariant.Primary}
                    onClick={() => {
                      form.handleSubmit();
                    }}
                  />
                )}
              </form.Subscribe>
            </div>
          </Controls>
        </form.AppForm>
      </form>
    </FullPage>
  );
};
