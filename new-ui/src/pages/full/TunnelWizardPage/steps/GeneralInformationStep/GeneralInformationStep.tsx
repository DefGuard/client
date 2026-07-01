import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { open } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import { useMemo } from 'react';
import z from 'zod';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { ButtonMenu } from '../../../../../shared/components/ButtonMenu/MenuButton';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { IconKind } from '../../../../../shared/components/Icon';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import { Snackbar } from '../../../../../shared/providers/snackbar/snackbar';
import { api } from '../../../../../shared/rust-api/api';
import { ThemeSpacing } from '../../../../../shared/types';
import {
  patternValidIpV6WithMask,
  patternValidIpWithMask,
} from '../../../../../shared/utils/patterns';
import { useTunnelWizardStore } from '../../hooks/useTunnelWizardStore';

const formSchema = z.object({
  name: z.string().trim().min(1, 'Field is required'),
  address: z.string().refine((value) => {
    if (value) {
      const ips = value.split(',').map((ip) => ip.trim());
      return ips.every(
        (ip) => patternValidIpWithMask.test(ip) || patternValidIpV6WithMask.test(ip),
      );
    }
    return false;
  }, 'Field is invalid'),
});

type FormFields = z.infer<typeof formSchema>;

export const GeneralInformationStep = () => {
  const navigate = useNavigate();
  const initData = useTunnelWizardStore((s) => s.tunnelData);

  const { mutate: importTunnelFile, isPending } = useMutation({
    mutationFn: async () => {
      const filePath = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'wg-conf', extensions: ['conf', 'txt', 'config'] }],
      });
      if (filePath) {
        const decoder = new TextDecoder();
        const fileContents = await readFile(filePath);
        const fileString = decoder.decode(fileContents);
        const config = await api.parseTunnelConfig({
          filename: filePath,
          config: fileString,
        });
        const current = useTunnelWizardStore.getState().tunnelData;
        useTunnelWizardStore.setState({ tunnelData: { ...current, ...config } });
        if (config.name) {
          form.setFieldValue('name', config.name);
        }
        if (config.address) {
          form.setFieldValue('address', config.address);
        }
        Snackbar.default('Config file applied');
      }
    },
  });

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
        <div className="actions">
          <ButtonMenu
            variant={ButtonVariant.Outlined}
            text="Actions"
            loading={isPending}
            menuItems={[
              {
                items: [
                  {
                    text: 'Import WireGuard config file',
                    icon: IconKind.Upload,
                    onClick: () => {
                      importTunnelFile();
                    },
                  },
                ],
              },
            ]}
          />
        </div>
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
