import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { Split } from '../../../../../shared/components/Split/Split';
import { Toggle } from '../../../../../shared/components/Toggle/Toggle';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import { api } from '../../../../../shared/rust-api/api';
import { ThemeSpacing } from '../../../../../shared/types';
import { useTunnelWizardStore } from '../../hooks/useTunnelWizardStore';

const formSchema = z.object({
  pre_up: z.string(),
  post_up: z.string(),
  pre_down: z.string(),
  post_down: z.string(),
});

type FormFields = z.infer<typeof formSchema>;

export const AdvancedSettingsStep = () => {
  const initData = useTunnelWizardStore((s) => s.tunnelData);
  const enabled = useTunnelWizardStore((s) => s.advancedSettingsEnabled);

  const { mutateAsync } = useMutation({ mutationFn: api.saveTunnel });

  const defaultValues = useMemo(
    (): FormFields => ({
      pre_up: initData.pre_up ?? '',
      post_up: initData.post_up ?? '',
      pre_down: initData.pre_down ?? '',
      post_down: initData.post_down ?? '',
    }),
    [initData.pre_up, initData.post_up, initData.pre_down, initData.post_down],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const { tunnelData, advancedSettingsEnabled } = useTunnelWizardStore.getState();
      const toCommand = (command: string) => (advancedSettingsEnabled ? command : '');
      const toSend = {
        ...tunnelData,
        pre_up: toCommand(value.pre_up),
        post_up: toCommand(value.post_up),
        pre_down: toCommand(value.pre_down),
        post_down: toCommand(value.post_down),
      };
      await mutateAsync(toSend);
      useTunnelWizardStore.getState().next();
    },
  });

  return (
    <div id="advanced-settings-step" className="step-content">
      <header>
        <h1>Advanced settings (optional)</h1>
        <SizedBox height={ThemeSpacing.Md} />
        <p>
          Define optional shell commands to run before or after the tunnel interface is
          brought up or down. Useful for custom routing rules, firewall adjustments, or
          other network configuration.
        </p>
      </header>
      <SizedBox height={ThemeSpacing.Xl2} />
      <Toggle
        testId="advanced-settings-toggle"
        active={enabled}
        label="Enable advanced configuration"
        helper="PreUp, PostUp, PreDown and PostDown commands run on this computer every time the tunnel connects or disconnects. They are saved only when this is enabled. Otherwise they are discarded, including commands from an imported config file."
        onClick={() => {
          useTunnelWizardStore.getState().setAdvancedSettingsEnabled(!enabled);
        }}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <Split>
            <form.AppField name="pre_up">
              {(field) => <field.FormInput notNull disabled={!enabled} label="Pre-up" />}
            </form.AppField>
            <form.AppField name="post_up">
              {(field) => <field.FormInput notNull disabled={!enabled} label="Post-up" />}
            </form.AppField>
          </Split>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Split>
            <form.AppField name="pre_down">
              {(field) => (
                <field.FormInput notNull disabled={!enabled} label="Pre-down" />
              )}
            </form.AppField>
            <form.AppField name="post_down">
              {(field) => (
                <field.FormInput notNull disabled={!enabled} label="Post-down" />
              )}
            </form.AppField>
          </Split>
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
