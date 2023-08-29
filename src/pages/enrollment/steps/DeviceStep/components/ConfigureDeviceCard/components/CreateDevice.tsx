import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { ToggleOption } from '../../../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import { useApi } from '../../../../../../../shared/hooks/api/useApi';
import { generateWGKeys } from '../../../../../../../shared/utils/generateWGKeys';
import { useEnrollmentStore } from '../../../../../hooks/store/useEnrollmentStore';

enum ConfigurationType {
  AUTO,
  MANUAL,
}

type FormFields = {
  name: string;
  configType: ConfigurationType;
  public?: string;
};

export const CreateDevice = () => {
  const {
    enrollment: { createDevice },
  } = useApi();

  const [autoKeys] = useState(generateWGKeys());

  const { LL } = useI18nContext();

  const cardLL = LL.pages.enrollment.steps.deviceSetup.cards.device;

  const setEnrollment = useEnrollmentStore((state) => state.setState);

  const toggleOptions: ToggleOption<ConfigurationType>[] = useMemo(
    () => [
      {
        text: cardLL.create.form.fields.toggle.generate(),
        value: ConfigurationType.AUTO,
      },
      {
        text: cardLL.create.form.fields.toggle.own(),
        value: ConfigurationType.MANUAL,
      },
    ],
    [cardLL.create.form.fields.toggle],
  );

  const schema = useMemo(
    () =>
      z
        .object({
          name: z.string().trim().nonempty(LL.form.errors.required()),
          configType: z.number(),
          public: z.string().trim().optional(),
        })
        .superRefine((val, ctx) => {
          if (val.configType === ConfigurationType.MANUAL && val.public?.length === 0) {
            ctx.addIssue({
              code: z.ZodIssueCode.custom,
              message: LL.form.errors.required(),
              path: ['public'],
            });
          }
        }),
    [LL.form.errors],
  );

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues: {
      name: '',
      configType: ConfigurationType.AUTO,
      public: '',
    },
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const { isLoading, mutate } = useMutation({
    mutationFn: createDevice,
    onSuccess: (res) => {
      setEnrollment({
        deviceState: {
          device: {
            ...res.device,
            privateKey: autoKeys.privateKey,
          },
          configs: res.configs,
        },
      });
    },
    onError: (res) => {
      console.error(res);
    },
  });

  const {
    field: { value: configTypeValue },
  } = useController({ control, name: 'configType' });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    if (!isLoading) {
      if (values.configType === ConfigurationType.MANUAL && values.public) {
        mutate({
          name: values.name,
          pubkey: values.public,
        });
      }
      if (values.configType === ConfigurationType.AUTO) {
        mutate({
          name: values.name,
          pubkey: autoKeys.publicKey,
        });
      }
    }
  };

  return (
    <>
      <MessageBox message={cardLL.create.messageBox()} />
      <form
        data-testid="enrollment-device-form"
        onSubmit={handleSubmit(handleValidSubmit)}
      >
        <FormInput
          label={cardLL.create.form.fields.name.label()}
          controller={{ control, name: 'name' }}
          required
        />
        <FormToggle
          controller={{ control, name: 'configType' }}
          options={toggleOptions}
        />
        <FormInput
          label={cardLL.create.form.fields.public.label()}
          controller={{ control, name: 'public' }}
          disabled={configTypeValue === ConfigurationType.AUTO}
          required={configTypeValue === ConfigurationType.MANUAL}
        />
        <Button
          type="submit"
          text={cardLL.create.submit()}
          loading={isLoading}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
        />
      </form>
    </>
  );
};
