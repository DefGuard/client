import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import { ThemeSpacing } from '../../../../../shared/types';
import { patternValidIp, patternValidIpV6 } from '../../../../../shared/utils/patterns';
import { useTunnelWizardStore } from '../../hooks/useTunnelWizardStore';

const formSchema = z.object({
  name: z.string().trim().min(1, 'Field is required'),
  address: z.string().refine((value) => {
    if (value) {
      const ips = value.split(',').map((ip) => ip.trim());
      return ips.every((ip) => patternValidIp.test(ip) || patternValidIpV6.test(ip));
    }
    return false;
  }, 'Field is invalid'),
});

type FormFields = z.infer<typeof formSchema>;

export const GeneralInformationStep = () => {
  const navigate = useNavigate();
  const initData = useTunnelWizardStore((s) => s.tunnelData);

  const defaultValues = useMemo(
    (): FormFields => ({
      address: initData.address,
      name: initData.name,
    }),
    [initData.address, initData.name],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      useTunnelWizardStore.getState().next(value);
    },
  });

  return (
    <div id="general-info-step" className="step-content">
      <header>
        <h1>General information</h1>
        <SizedBox height={ThemeSpacing.Md} />
        <p>{`Upload your config file (optional) and we'll securely extract the connection settings for you. This is the fastest and recommended way to get started.`}</p>
      </header>
      <Divider spacing={ThemeSpacing.Xl2} />
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Tunnel name" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="address">
            {(field) => <field.FormInput required label="Address" />}
          </form.AppField>
        </form.AppForm>
      </form>
      <Controls>
        <Button
          text="Cancel"
          onClick={() => {
            navigate({ to: '/full/add', replace: true });
          }}
          variant={ButtonVariant.Secondary}
        />
        <div className="right">
          <Button
            text="Continue"
            variant={ButtonVariant.Primary}
            onClick={() => {
              form.handleSubmit();
            }}
          />
        </div>
      </Controls>
    </div>
  );
};
