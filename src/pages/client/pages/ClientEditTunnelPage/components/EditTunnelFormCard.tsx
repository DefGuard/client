import { zodResolver } from '@hookform/resolvers/zod';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { ArrowSingle } from '../../../../../shared/defguard-ui/components/icons/ArrowSingle/ArrowSingle';
import {
  ArrowSingleDirection,
  ArrowSingleSize,
} from '../../../../../shared/defguard-ui/components/icons/ArrowSingle/types';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { useToaster } from '../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import {
  cidrRegex,
  patternValidEndpoint,
  patternValidIp,
  patternValidIpV6,
  patternValidIpV6WithPort,
  patternValidWireguardKey,
} from '../../../../../shared/patterns';
import { routes } from '../../../../../shared/routes';
import { validateIpOrDomainList } from '../../../../../shared/validators/tunnel';
import { clientApi } from '../../../clientAPI/clientApi';
import { Tunnel } from '../../../types';

type Props = {
  tunnel: Tunnel;
  submitRef: React.MutableRefObject<HTMLInputElement | null>; // Add submitRef prop
};

type FormFields = {
  id?: number;
  name: string;
  pubkey: string;
  prvkey: string;
  address: string;
  server_pubkey: string;
  preshared_key?: string;
  allowed_ips?: string;
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
  preshared_key: '',
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
const { updateTunnel } = clientApi;

const tunnelToForm = (tunnel: Tunnel): FormFields => {
  const {
    id,
    pubkey,
    prvkey,
    server_pubkey,
    preshared_key,
    allowed_ips,
    dns,
    persistent_keep_alive,
    pre_up,
    post_up,
    pre_down,
    post_down,
    ...commonFields
  } = tunnel;

  return {
    id: id,
    pubkey,
    prvkey,
    server_pubkey,
    preshared_key: preshared_key || '',
    allowed_ips: allowed_ips || '',
    dns: dns || '',
    persistent_keep_alive,
    pre_up: pre_up || '',
    post_up: post_up || '',
    pre_down: pre_down || '',
    post_down: post_down || '',
    ...commonFields,
  };
};

export const EditTunnelFormCard = ({ tunnel, submitRef }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addTunnelPage.forms.initTunnel;
  const navigate = useNavigate();
  const toaster = useToaster();

  const defaultFormValues: FormFields = useMemo<FormFields>(() => {
    if (tunnel) {
      return tunnelToForm(tunnel);
    }
    return defaultValues;
  }, [tunnel]);

  const schema = useMemo(
    () =>
      z.object({
        id: z.number(),
        name: z.string().trim().min(1, LL.form.errors.required()),
        pubkey: z
          .string()
          .trim()
          .min(1, LL.form.errors.required())
          .refine((value) => {
            return patternValidWireguardKey.test(value);
          }, LL.form.errors.invalid()),
        prvkey: z
          .string()
          .trim()
          .min(1, LL.form.errors.required())
          .refine((value) => {
            return patternValidWireguardKey.test(value);
          }, LL.form.errors.invalid()),
        server_pubkey: z
          .string()
          .trim()
          .min(1, LL.form.errors.required())
          .refine((value) => {
            return patternValidWireguardKey.test(value);
          }, LL.form.errors.invalid()),
        preshared_key: z
          .string()
          .trim()
          .refine((value) => {
            return value === '' || patternValidWireguardKey.test(value);
          }, LL.form.errors.invalid()),
        address: z.string().refine((value) => {
          return patternValidIp.test(value) || patternValidIpV6.test(value);
        }, LL.form.errors.invalid()),
        endpoint: z
          .string()
          .min(1, LL.form.errors.required())
          .refine((value) => {
            return (
              patternValidEndpoint.test(value) || patternValidIpV6WithPort.test(value)
            );
          }, LL.form.errors.invalid()),
        dns: z
          .string()
          .refine((value) => {
            if (value && value.length != 0) {
              return validateIpOrDomainList(value, ',', true);
            }
            return true;
          }, LL.form.errors.invalid())
          .optional(),
        allowed_ips: z.string().refine((value) => {
          if (value) {
            const ips = value.split(',').map((ip) => ip.trim());
            return ips.every((ip) => cidrRegex.test(ip));
          }
          return true;
        }, LL.form.errors.invalid()),
        persistent_keep_alive: z.number(),
        route_all_traffic: z.boolean(),
        pre_up: z.string().nullable(),
        post_up: z.string().nullable(),
        pre_down: z.string().nullable(),
        post_down: z.string().nullable(),
      }),
    [LL.form.errors],
  );

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    updateTunnel(values)
      .then(() => {
        navigate(routes.client.base, { replace: true });
        toaster.success(LL.pages.client.pages.editTunnelPage.messages.editSuccess());
      })
      .catch(() =>
        toaster.error(LL.pages.client.pages.editTunnelPage.messages.editError()),
      );
  };

  const { handleSubmit, control } = useForm<Tunnel>({
    resolver: zodResolver(schema),
    defaultValues: defaultFormValues,
    mode: 'all',
  });

  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const handleToggleAdvancedOptions = () => {
    setShowAdvancedOptions(!showAdvancedOptions);
  };

  return (
    <>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <Card id="edit-tunnel-form-card">
          <header className="header">
            <h2>Tunnel Configuration</h2>
            <div className="controls"></div>
          </header>
          <div className="client">
            <FormInput
              controller={{ control, name: 'name' }}
              label={localLL.labels.name()}
              labelExtras={<Helper>{localLL.helpers.name()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'prvkey' }}
              label={localLL.labels.privateKey()}
              labelExtras={<Helper>{localLL.helpers.prvkey()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'pubkey' }}
              label={localLL.labels.publicKey()}
              labelExtras={<Helper>{localLL.helpers.pubkey()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'address' }}
              label={localLL.labels.address()}
              labelExtras={<Helper>{localLL.helpers.address()}</Helper>}
            />
          </div>
        </Card>
        <Card>
          <h3>{localLL.sections.vpnServer()}</h3>
          <FormInput
            controller={{ control, name: 'server_pubkey' }}
            label={localLL.labels.serverPubkey()}
            labelExtras={<Helper>{localLL.helpers.serverPubkey()}</Helper>}
          />
          <FormInput
            controller={{ control, name: 'preshared_key' }}
            label={localLL.labels.presharedKey()}
            labelExtras={<Helper>{localLL.helpers.presharedKey()}</Helper>}
          />
          <FormInput
            controller={{ control, name: 'endpoint' }}
            label={localLL.labels.endpoint()}
            labelExtras={<Helper>{localLL.helpers.endpoint()}</Helper>}
          />
          <FormInput
            controller={{ control, name: 'dns' }}
            label={localLL.labels.dns()}
            labelExtras={<Helper>{localLL.helpers.dns()}</Helper>}
          />
          <FormInput
            controller={{ control, name: 'allowed_ips' }}
            label={localLL.labels.allowedips()}
            labelExtras={<Helper>{localLL.helpers.allowedIps()}</Helper>}
          />

          <FormInput
            controller={{ control, name: 'persistent_keep_alive' }}
            label={localLL.labels.persistentKeepAlive()}
            labelExtras={<Helper>{localLL.helpers.persistentKeepAlive()}</Helper>}
          />
          <div className="advanced-options-header">
            <h3>{localLL.sections.advancedOptions()}</h3>
            <Helper> {localLL.helpers.advancedOptions()}</Helper>
            <div className="underscore"></div>
            <button type="button" onClick={handleToggleAdvancedOptions}>
              <ArrowSingle
                direction={
                  showAdvancedOptions
                    ? ArrowSingleDirection.UP
                    : ArrowSingleDirection.DOWN
                }
                size={ArrowSingleSize.SMALL}
              />
            </button>
          </div>
          <div className={`advanced-options ${showAdvancedOptions ? 'open' : ''}`}>
            <FormInput
              controller={{ control, name: 'pre_up' }}
              label={localLL.labels.preUp()}
              labelExtras={<Helper>{localLL.helpers.preUp()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'post_up' }}
              label={localLL.labels.postUp()}
              labelExtras={<Helper>{localLL.helpers.postUp()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'pre_down' }}
              label={localLL.labels.PreDown()}
              labelExtras={<Helper>{localLL.helpers.preDown()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'post_down' }}
              label={localLL.labels.PostDown()}
              labelExtras={<Helper>{localLL.helpers.postDown()}</Helper>}
            />
          </div>
        </Card>
        <input type="submit" aria-hidden="true" className="hidden" ref={submitRef} />
      </form>
    </>
  );
};
