import './style.scss';

import { useLoaderData, useSearch } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { FullPage } from '../FullPage/FullPage';

const formSchema = z.object({
  token: z.string().min(1, 'Required'),
  url: z.string().min(1, 'Required'),
  name: z.string().min(1, 'Required'),
});

type FormFields = z.infer<typeof formSchema>;

export const AddInstancePage = () => {
  const { deviceName } = useLoaderData({ from: '/full/add/instance' });
  const { token, url } = useSearch({ from: '/full/add/instance' });

  const defaultValues = useMemo((): FormFields => {
    return {
      token: token ?? '',
      url: url ?? '',
      name: deviceName,
    };
  }, [token, url, deviceName]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
  });

  return (
    <FullPage id="add-instance-view">
      <FullPageTitle title="Add instance" />
      <p className="page-description">{`To add an instance, provide the instance URL along with a valid provisioning token. These credentials are issued by your administrator and are required to initiate the setup.`}</p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          form.handleSubmit();
        }}
      >
        <p>Some inputs here i guess</p>
      </form>
    </FullPage>
  );
};
