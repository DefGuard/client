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

const { saveTunnel } = clientApi;

export const EditTunnelFormCard = ({ tunnel, submitRef }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addTunnelPage.forms.initTunnel;
  const navigate = useNavigate();

  const toaster = useToaster();
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
        address: z.string().refine((value) => {
          return patternValidIp.test(value);
        }, LL.form.errors.invalid()),
        endpoint: z
          .string()
          .min(1, LL.form.errors.required())
          .refine((value) => {
            return patternValidEndpoint.test(value);
          }, LL.form.errors.invalid()),
        dns: z
          .string()
          .refine((value) => {
            return validateIpOrDomainList(value, ',', true);
          }, LL.form.errors.invalid())
          .optional(),
        allowed_ips: z.string().refine((value) => {
          const ips = value.split(',').map((ip) => ip.trim());
          return ips.every((ip) => cidrRegex.test(ip));
        }, LL.form.errors.invalid()),
        persistent_keep_alive: z.number(),
        route_all_traffic: z.boolean(),
        pre_up: z.string().nullable(), // Add nullable to missing fields
        post_up: z.string().nullable(),
        pre_down: z.string().nullable(),
        post_down: z.string().nullable(),
      }),
    [LL.form.errors],
  );

  const handleValidSubmit: SubmitHandler<Tunnel> = (values) => {
    saveTunnel(values)
      .then(() => {
        navigate(routes.client.base, { replace: true });
        toaster.success(localLL.messages.addSuccess());
      })
      .catch(() => toaster.error(localLL.messages.addError()));
  };

  const { handleSubmit, control } = useForm<Tunnel>({
    resolver: zodResolver(schema),
    defaultValues: tunnel,
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
