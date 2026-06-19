import './style.scss';
import { useMemo } from 'react';
import z from 'zod';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { TooltipButton } from '../../../../../shared/components/TooltipButton/TooltipButton';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import { ThemeSpacing } from '../../../../../shared/types';
import { generateWGKeys } from '../../../../../shared/utils/generateWGKeys';
import { patternValidWireguardKey } from '../../../../../shared/utils/patterns';
import { useTunnelWizardStore } from '../../hooks/useTunnelWizardStore';

const formSchema = z.object({
  prvkey: z
    .string()
    .refine((v) => patternValidWireguardKey.test(v), 'Invalid WireGuard key'),
  pubkey: z
    .string()
    .refine((v) => patternValidWireguardKey.test(v), 'Invalid WireGuard key'),
});

type FormFields = z.infer<typeof formSchema>;

export const KeysStep = () => {
  const initData = useTunnelWizardStore((s) => s.tunnelData);

  const defaultValues = useMemo(
    (): FormFields => ({
      prvkey: initData.prvkey,
      pubkey: initData.pubkey,
    }),
    [initData.prvkey, initData.pubkey],
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
    <div id="keys-step" className="step-content">
      <header>
        <h1>Keys</h1>
        <SizedBox height={ThemeSpacing.Md} />
        <p>{`Upload your config file (optional) and we'll securely extract the connection settings for you. This is the fastest and recommended way to get started.`}</p>
      </header>
      <SizedBox height={ThemeSpacing.Xl2} />
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="prvkey">
            {(field) => <field.FormInput required label="Private key" type="password" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="pubkey">
            {(field) => <field.FormInput required label="Public key" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <div className="actions">
            <TooltipButton
              buttonProps={{
                text: 'Generate keys',
                iconLeft: 'refresh',
                variant: ButtonVariant.Secondary,
                onClick: () => {
                  const pair = generateWGKeys();
                  form.setFieldValue('prvkey', pair.privateKey);
                  form.setFieldValue('pubkey', pair.publicKey);
                },
              }}
              tooltipText="New keys set"
            />
          </div>
        </form.AppForm>
      </form>
      <Controls>
        <Button
          text="Back"
          onClick={() => useTunnelWizardStore.getState().back(form.state.values)}
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
