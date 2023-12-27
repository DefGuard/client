import { zodResolver } from '@hookform/resolvers/zod';
import { useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';
import { clientApi } from '../../../../clientAPI/clientApi';

type FormFields = {
  name: string;
  pubkey: string;
  prvkey: string;
  address: string;
  server_pubkey: string;
  allowed_ips: string;
  endpoint: string;
  dns?: string;
  persistent_keep_alive: number;
  route_all_traffic: boolean;
  pre_up?: string;
  post_up?: string;
  pre_down?: string;
  post_down?: string;
};
const defaultValues: FormFields = {
  name: '',
  pubkey: '',
  prvkey: '',
  address: '',
  server_pubkey: '',
  allowed_ips: '',
  endpoint: '',
  dns: '',
  persistent_keep_alive: 25, // Adjust as needed
  route_all_traffic: false,
  pre_up: '',
  post_up: '',
  pre_down: '',
  post_down: '',
};

export const AddTunnelFormCard = () => {
  const { LL } = useI18nContext();
  const { parseConfig, saveTunnel } = clientApi;
  const toaster = useToaster();
  const cidrRegex =
    /^(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\/\d{1,2}|[0-9a-fA-F:.]+\/\d{1,3})$/;

  const localLL = LL.pages.client.pages.addTunnelPage.forms.initTunnel;
  /* eslint-disable no-useless-escape */
  const schema = useMemo(
    () =>
      z.object({
        name: z.string().trim().min(1, LL.form.errors.required()),
        pubkey: z.string().trim().min(1, LL.form.errors.required()),
        prvkey: z.string().trim().min(1, LL.form.errors.required()),
        server_pubkey: z.string().trim().min(1, LL.form.errors.required()),
        address: z.string().refine((value) => {
          // Regular expression to match IPv4 CIDR and IPv6 CIDR
          return cidrRegex.test(value);
        }, 'Invalid IP address with subnet mask format'),
        endpoint: z.string().refine((value) => {
          // Regular expression to match IPv4, IPv6, or domain name with port
          const endpointRegex = /^([0-9a-fA-F:\.]+|\w+(\.\w+)+)(:\d+)?$/;
          return endpointRegex.test(value);
        }, 'Invalid VPN server endpoint format'),
        dns: z.string().trim().min(1, LL.form.errors.required()),
        allowed_ips: z.string().refine((value) => {
          // Regular expression to match IPv4 CIDR and IPv6 CIDR
          const ips = value.split(',').map((ip) => ip.trim());
          return ips.every((ip) => cidrRegex.test(ip));
        }, 'Invalid allowed IPs format'),
        persistent_keep_alive: z.number(),
        route_all_traffic: z.boolean(),
      }),
    [LL.form.errors, cidrRegex],
  );
  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    saveTunnel(values)
      .then(() => toaster.success(localLL.messages.addSuccess()))
      .catch(() => toaster.error(localLL.messages.addError()));
  };
  const { handleSubmit, control, reset, setValue } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const [generatedKeys, setGeneratedKeys] = useState(false);

  const handleConfigUpload = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = false;
    input.style.display = 'none';
    input.onchange = () => {
      if (input.files && input.files.length === 1) {
        const reader = new FileReader();
        reader.onload = () => {
          if (reader.result && input.files) {
            const res = reader.result;
            parseConfig(res as string)
              .then((data) => reset(data as FormFields))
              .catch(() => toaster.error(localLL.messages.configError()));
          }
        };
        reader.onerror = () => {
          toaster.error(localLL.messages.configError());
        };
        reader.readAsText(input.files[0]);
      }
    };
    input.click();
  };

  const generateKeyPair = () => {
    const { privateKey, publicKey } = generateWGKeys();
    setValue('prvkey', privateKey);
    setValue('pubkey', publicKey);
    setGeneratedKeys(true);
  };

  useEffect(() => {
    // FIXME: handle any
    const onPrvKeyChange = (e) => {
      if (generatedKeys && e.target.value !== defaultValues.prvkey) {
        setGeneratedKeys(false);
      }
    };

    // Attach the event listener to the 'prvkey' input
    const prvKeyInput = document.getElementsByName('prvkey')[0];
    if (prvKeyInput) {
      prvKeyInput.addEventListener('input', onPrvKeyChange);

      // Cleanup the event listener when the component unmounts
      return () => {
        prvKeyInput.removeEventListener('input', onPrvKeyChange);
      };
    }
  }, [generatedKeys]);

  return (
    <Card id="add-tunnel-form-card">
      <header className="header">
        <h2>Tunnel Configuration</h2>
        <div className="controls">
          <Button
            styleVariant={ButtonStyleVariant.STANDARD}
            text={'Import Config File'}
            onClick={() => handleConfigUpload()}
          />
          <Button
            styleVariant={ButtonStyleVariant.STANDARD}
            text={'Generate key pair'}
            onClick={() => generateKeyPair()}
          />
        </div>
      </header>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput controller={{ control, name: 'name' }} label={localLL.labels.name()} />
        <FormInput
          controller={{ control, name: 'prvkey' }}
          label={localLL.labels.privateKey()}
        />
        <FormInput
          controller={{ control, name: 'pubkey' }}
          disabled={generatedKeys}
          label={localLL.labels.publicKey()}
        />
        <FormInput
          controller={{ control, name: 'address' }}
          label={localLL.labels.address()}
        />
        <h3> VPN Server</h3>
        <FormInput
          controller={{ control, name: 'server_pubkey' }}
          label={localLL.labels.serverPubkey()}
        />
        <FormInput
          controller={{ control, name: 'endpoint' }}
          label={localLL.labels.endpoint()}
        />
        <FormInput controller={{ control, name: 'dns' }} label={localLL.labels.dns()} />
        <FormInput
          controller={{ control, name: 'allowed_ips' }}
          label={localLL.labels.allowedips()}
        />

        <FormInput
          controller={{ control, name: 'persistent_keep_alive' }}
          label={localLL.labels.persistentKeepAlive()}
        />
        <h3>Advanced Options</h3>
        <FormInput
          controller={{ control, name: 'pre_up' }}
          label={localLL.labels.preUp()}
        />
        <FormInput
          controller={{ control, name: 'post_up' }}
          label={localLL.labels.postUp()}
        />
        <FormInput
          controller={{ control, name: 'pre_down' }}
          label={localLL.labels.PreDown()}
        />
        <FormInput
          controller={{ control, name: 'post_down' }}
          label={localLL.labels.PostDown()}
        />
        <div className="controls">
          <Button
            type="submit"
            className="submit"
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={localLL.submit()}
          />
        </div>
      </form>
    </Card>
  );
};
