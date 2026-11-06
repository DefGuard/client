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
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { Snackbar } from '../../../shared/providers/snackbar/snackbar';
import {
  enrollmentAddInstance,
  enrollmentCreateDevice,
} from '../../../shared/rust-api/enrollment';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { useEnrollmentStore } from '../EnrollmentPage/hooks/useEnrollmentStore';
import { getAddInstanceFailure } from './utils/getAddInstanceFailure';

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
      const result = await enrollmentAddInstance(value);
      if (result.error ?? result.errorKind) {
        void error(
          `Failed to add instance (${result.errorKind ?? 'unknown'}): ${result.error ?? ''}`,
        );
        const failure = getAddInstanceFailure(result, hasInitialValues);
        if (failure.kind === 'field') {
          formApi.setErrorMap({
            onSubmit: { fields: { [failure.field]: failure.message } },
          });
        } else {
          Snackbar.error(failure.message);
        }
        return;
      }

      const { session_id, startResponse } = result;
      if (session_id) {
        const created = await enrollmentCreateDevice(session_id, value.name.trim());
        if (created.error) {
          void error(`Failed to create device: ${created.error}`);
          Snackbar.error('Failed to create device, contact administrator.');
          return;
        }
      }

      if (session_id && startResponse && !startResponse.user.enrolled) {
        useEnrollmentStore.getState().start(startResponse, session_id, undefined);
        navigate({ to: '/full/enrollment', replace: true });
        return;
      }
      navigate({ to: '/full/overview', replace: true });
    },
  });

  return (
    <FullPage id="add-instance-view" hideScrollContainer withControls>
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
