import { useMemo } from 'react';
import z from 'zod';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { Split } from '../../../../../shared/components/Split/Split';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import { ThemeSpacing } from '../../../../../shared/types';
import {
  cidrRegex,
  patternValidEndpoint,
  patternValidWireguardKey,
} from '../../../../../shared/utils/patterns';
import { useTunnelWizardStore } from '../../hooks/useTunnelWizardStore';

const formSchema = z.object({
  server_pubkey: z
    .string()
    .refine((v) => patternValidWireguardKey.test(v), 'Invalid WireGuard key'),
  preshared_key: z
    .string()
    .refine((v) => !v || patternValidWireguardKey.test(v), 'Invalid WireGuard key'),
  endpoint: z.string().refine((v) => patternValidEndpoint.test(v), 'Invalid address'),
  dns: z.string(),
  allowed_ips: z.string().refine((v) => {
    if (!v) return true;
    return v
      .split(',')
      .map((s) => s.trim())
      .every((cidr) => cidrRegex.test(cidr));
  }, 'Invalid CIDR notation'),
  persistent_keep_alive: z.number().int().min(0),
});

type FormFields = z.infer<typeof formSchema>;

export const VpnServerStep = () => {
  const initData = useTunnelWizardStore((s) => s.tunnelData);

  const defaultValues = useMemo(
    (): FormFields => ({
      server_pubkey: initData.server_pubkey,
      preshared_key: initData.preshared_key,
      endpoint: initData.endpoint,
      dns: initData.dns ?? '',
      allowed_ips: initData.allowed_ips ?? '',
      persistent_keep_alive: initData.persistent_keep_alive,
    }),
    [
      initData.server_pubkey,
      initData.preshared_key,
      initData.endpoint,
      initData.dns,
      initData.allowed_ips,
      initData.persistent_keep_alive,
    ],
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
    <div id="vpn-server-step" className="step-content">
      <header>
        <h1>VPN Server</h1>
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
          <Split>
            <form.AppField name="server_pubkey">
              {(field) => <field.FormInput required label="Public key" />}
            </form.AppField>
            <form.AppField name="preshared_key">
              {(field) => <field.FormInput label="Preshared key" type="password" />}
            </form.AppField>
          </Split>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Split>
            <form.AppField name="endpoint">
              {(field) => <field.FormInput required label="VPN server address:port" />}
            </form.AppField>
            <form.AppField name="dns">
              {(field) => <field.FormInput label="DNS" />}
            </form.AppField>
          </Split>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="allowed_ips">
            {(field) => (
              <field.FormInput label="Allowed IPs (add multiple separated by coma)" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="persistent_keep_alive">
            {(field) => <field.FormInput required label="Persistent keep alive (sec)" />}
          </form.AppField>
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
